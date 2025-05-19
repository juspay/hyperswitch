use super::errors::{self, RouterResponse, RouterResult};
use crate::{
    logger,
    routes::SessionState,
    services::{self, request::Mask},
    types::domain,
};
pub mod utils;
use api_models::proxy as proxy_api_models;
use common_utils::{
    ext_traits::BytesExt,
    request::{self, RequestBuilder},
};
use error_stack::ResultExt;
use hyperswitch_interfaces::types::Response;
use serde_json::Value;

fn process_value(value: Value, vault_data: &Value) -> RouterResult<Value> {
    match value {
        Value::Object(obj) => {
            let new_obj = obj
                .into_iter()
                .map(|(key, val)| process_value(val, vault_data).map(|processed| (key, processed)))
                .collect::<Result<serde_json::Map<_, _>, error_stack::Report<errors::ApiErrorResponse>>>()?;

            Ok(Value::Object(new_obj))
        }
        Value::String(s) => (!utils::contains_token(&s))
            .then_some(Value::String(s.clone()))
            .map(Ok)
            .unwrap_or_else(|| {
                utils::parse_token(&s)
                    .map(|(_, token_ref)| {
                        extract_field_from_vault_data(vault_data, &token_ref.field)
                    })
                    .unwrap_or_else(|_| {
                        Err(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable(format!("Invalid token format in string: {}", s))
                    })
            }),
        _ => Ok(value),
    }
}

fn find_field_recursively_in_vault_data(
    obj: &serde_json::Map<String, Value>,
    field_name: &str,
) -> Option<Value> {
    obj.get(field_name).cloned().or_else(|| {
        obj.values()
            .filter_map(|val| {
                if let Value::Object(inner_obj) = val {
                    find_field_recursively_in_vault_data(inner_obj, field_name)
                } else {
                    None
                }
            })
            .next()
    })
}

fn extract_field_from_vault_data(vault_data: &Value, field_name: &str) -> RouterResult<Value> {
    let result = match vault_data {
        Value::Object(obj) => find_field_recursively_in_vault_data(obj, field_name),
        _ => None,
    };
    match result {
        Some(value) => Ok(value),
        None => {
            logger::debug!(
                "Field '{}' not found in vault data: {:?}",
                field_name,
                vault_data
            );
            Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(format!("Field '{}' not found", field_name))
        }
    }
}

async fn execute_proxy_request(
    state: &SessionState,
    req: &proxy_api_models::ProxyRequest,
    processed_body: Value,
) -> RouterResult<Response> {
    let headers: Vec<(String, masking::Maskable<String>)> = req
        .headers
        .as_map()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone().into_masked()))
        .collect();

    let request = RequestBuilder::new()
        .method(req.method)
        .attach_default_headers()
        .headers(headers)
        .url(&req.destination_url.as_str())
        .set_body(request::RequestContent::Json(Box::new(processed_body)))
        .build();

    let response = services::call_connector_api(state, request, "proxy")
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Faile to call the destination");

    response
        .and_then(|inner| match inner {
            Err(err_res) => {
                logger::error!("Error while receiving response: {err_res:?}");
                Ok(err_res)
            }
            Ok(res) => Ok(res),
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while receiving response")
}

struct ProxyResponseWrapper(Response);

impl TryFrom<ProxyResponseWrapper> for proxy_api_models::ProxyResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn try_from(wrapper: ProxyResponseWrapper) -> Result<Self, Self::Error> {
        let res = wrapper.0;
        let response_body: Value = res
            .response
            .parse_struct("ProxyResponse")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse the response")?;

        let status_code = res.status_code;
        let response_headers = res
            .headers
            .as_ref()
            .map(|h| {
                let map: std::collections::HashMap<_, _> = h
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                serde_json::to_value(map).unwrap_or_else(|_| serde_json::json!({}))
            })
            .unwrap_or_else(|| serde_json::json!({}));

        Ok(Self {
            response: response_body,
            status_code,
            response_headers,
        })
    }
}

pub async fn proxy_core(
    state: SessionState,
    merchant_context: domain::MerchantContext,
    req: proxy_api_models::ProxyRequest,
) -> RouterResponse<proxy_api_models::ProxyResponse> {
    let vault_id = utils::ProxyRequestWrapper(req.clone())
        .get_vault_id(
            &state,
            &merchant_context.get_merchant_key_store(),
            merchant_context.get_merchant_account().storage_scheme,
        )
        .await?;

    let vault_response = super::payment_methods::vault::retrieve_payment_method_from_vault(
        &state,
        &merchant_context,
        &vault_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error while fetching data from vault")?;

    let vault_data = serde_json::to_value(&vault_response.data)
        .map_err(|err| {
            logger::error!("Error serializing data to JSON value: {:?}", err);
            errors::ApiErrorResponse::InternalServerError
        })
        .attach_printable("Failed to serialize vault data")?;

    let processed_body = process_value(req.req_body.clone(), &vault_data)?;

    let res = execute_proxy_request(&state, &req, processed_body).await?;

    let proxy_response = proxy_api_models::ProxyResponse::try_from(ProxyResponseWrapper(res))?;

    Ok(services::ApplicationResponse::Json(proxy_response))
}
