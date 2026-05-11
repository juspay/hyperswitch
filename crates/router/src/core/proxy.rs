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
use std::collections::HashMap;

pub async fn proxy_core(
    state: SessionState,
    platform: domain::Platform,
    req: proxy_api_models::ProxyRequest,
) -> RouterResponse<proxy_api_models::ProxyResponse> {
    let processed_body = if let Some(ref token) = req.token {
        // ── Single-token mode ──────────────────────────────────────────────
        // Fetch vault data for the one token, then replace {{$field}} placeholders.
        let req_wrapper = utils::ProxyRequestWrapper(req.clone());
        let proxy_record = req_wrapper
            .get_proxy_record(&state, platform.get_provider())
            .await?;
        let vault_data = proxy_record.get_vault_data(&state, platform).await?;
        interpolate_single_token(req.request_body.clone(), &vault_data)?
    } else {
        // ── Multi-token mode ───────────────────────────────────────────────
        // Scan request_body for {{$field: token}} placeholders, fetch vault data
        // per unique token, then replace each placeholder with the corresponding field.
        let tokens = utils::collect_tokens_from_value(&req.request_body);

        // Build a map: token_value → vault_data
        let mut token_vault_map: HashMap<String, Value> = HashMap::new();
        for token in tokens {
            let vault_data = utils::get_vault_data_for_token(
                &state,
                &platform,
                &token,
                &req.token_type,
            )
            .await?;
            token_vault_map.insert(token, vault_data);
        }

        interpolate_multi_token(req.request_body.clone(), &token_vault_map)?
    };

    let req_wrapper = utils::ProxyRequestWrapper(req.clone());
    let res = execute_proxy_request(&state, &req_wrapper, processed_body).await?;

    let proxy_response = proxy_api_models::ProxyResponse::try_from(ProxyResponseWrapper(res))?;

    Ok(services::ApplicationResponse::Json(proxy_response))
}

/// Single-token mode: replace `{{$field}}` with the matching field from vault_data.
fn interpolate_single_token(
    value: Value,
    vault_data: &Value,
) -> RouterResult<Value> {
    match value {
        Value::Object(obj) => {
            let new_obj = obj
                .into_iter()
                .map(|(key, val)| {
                    interpolate_single_token(val, vault_data).map(|v| (key, v))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()?;
            Ok(Value::Object(new_obj))
        }
        Value::Array(arr) => {
            let new_arr = arr
                .into_iter()
                .map(|val| interpolate_single_token(val, vault_data))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Array(new_arr))
        }
        Value::String(s) => utils::parse_token(&s.clone())
            .map(|(_, token_ref)| extract_field_from_vault_data(vault_data, &token_ref.field))
            .unwrap_or(Ok(Value::String(s))),
        _ => Ok(value),
    }
}

/// Multi-token mode: replace `{{$field: token_value}}` with the matching field
/// from the vault data fetched for that specific token.
fn interpolate_multi_token(
    value: Value,
    token_vault_map: &HashMap<String, Value>,
) -> RouterResult<Value> {
    match value {
        Value::Object(obj) => {
            let new_obj = obj
                .into_iter()
                .map(|(key, val)| {
                    interpolate_multi_token(val, token_vault_map).map(|v| (key, v))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()?;
            Ok(Value::Object(new_obj))
        }
        Value::Array(arr) => {
            let new_arr = arr
                .into_iter()
                .map(|val| interpolate_multi_token(val, token_vault_map))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Array(new_arr))
        }
        Value::String(ref s) if utils::contains_multi_token(s) => {
            match utils::parse_multi_token(s) {
                Ok((_, multi_ref)) => {
                    let vault_data =
                        token_vault_map
                            .get(&multi_ref.token)
                            .ok_or_else(|| errors::ApiErrorResponse::InternalServerError)
                            .attach_printable(format!(
                                "No vault data found for token '{}'",
                                multi_ref.token
                            ))?;
                    extract_field_from_vault_data(vault_data, &multi_ref.field)
                }
                Err(_) => Ok(value),
            }
        }
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
