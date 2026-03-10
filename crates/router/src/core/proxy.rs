use super::errors::{self, RouterResponse, RouterResult};
use crate::{logger, routes::SessionState, services, types::domain};
pub mod utils;
use api_models::proxy as proxy_api_models;
use common_utils::{
    ext_traits::BytesExt,
    request::{self, RequestBuilder},
};
use error_stack::ResultExt;
use hyperswitch_interfaces::types::Response;
use serde_json::Value;

pub async fn proxy_core(
    state: SessionState,
    platform: domain::Platform,
    req: proxy_api_models::ProxyRequest,
) -> RouterResponse<proxy_api_models::ProxyResponse> {
    let req_wrapper = utils::ProxyRequestWrapper(req.clone());
    let proxy_record = req_wrapper
        .get_proxy_record(
            &state,
            platform.get_processor().get_key_store(),
            platform.get_processor().get_account().storage_scheme,
        )
        .await?;

    let vault_data = proxy_record.get_vault_data(&state, platform).await?;

    let processed_body =
        interpolate_token_references_with_vault_data(req.request_body.clone(), &vault_data)?;

    let res = execute_proxy_request(&state, &req_wrapper, processed_body).await?;

    let proxy_response = proxy_api_models::ProxyResponse::try_from(ProxyResponseWrapper(res))?;

    Ok(services::ApplicationResponse::Json(proxy_response))
}

fn interpolate_token_references_with_vault_data(
    value: Value,
    vault_data: &Value,
) -> RouterResult<Value> {
    match value {
        Value::Object(obj) => {
            let new_obj = obj
                .into_iter()
                .map(|(key, val)| interpolate_token_references_with_vault_data(val, vault_data).map(|processed| (key, processed)))
                .collect::<Result<serde_json::Map<_, _>, error_stack::Report<errors::ApiErrorResponse>>>()?;

            Ok(Value::Object(new_obj))
        }
        Value::String(s) => utils::parse_token(&s)
            .map(|(_, token_ref)| extract_field_from_vault_data(vault_data, &token_ref.field))
            .unwrap_or(Ok(Value::String(s.clone()))),
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
    match vault_data {
        Value::Object(obj) => find_field_recursively_in_vault_data(obj, field_name)
            .ok_or_else(|| errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!("Field '{field_name}' not found")),
        _ => Err(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Vault data is not a valid JSON object"),
    }
}

async fn execute_proxy_request(
    state: &SessionState,
    req_wrapper: &utils::ProxyRequestWrapper,
    processed_body: Value,
) -> RouterResult<Response> {
    let request = RequestBuilder::new()
        .method(req_wrapper.get_method())
        .attach_default_headers()
        .headers(req_wrapper.get_headers())
        .url(req_wrapper.get_destination_url())
        .set_body(request::RequestContent::Json(Box::new(processed_body)))
        .build();

    let response = services::call_connector_api(state, request, "proxy")
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to call the destination");

    response
        .map(|inner| match inner {
            Err(err_res) => {
                logger::error!("Error while receiving response: {err_res:?}");
                err_res
            }
            Ok(res) => res,
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
        let response_headers = proxy_api_models::Headers::from_header_map(res.headers.as_ref());

        Ok(Self {
            response: response_body,
            status_code,
            response_headers,
        })
    }
}
