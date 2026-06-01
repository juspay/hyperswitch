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
        // ── Single-token mode (explicit token provided) ────────────────────
        // Fetch vault data for the one token, then replace {{$field}} placeholders.
        logger::info!("proxy_core: single-token mode (explicit token)");
        let req_wrapper = utils::ProxyRequestWrapper(req.clone());
        let proxy_record = req_wrapper
            .get_proxy_record(&state, platform.get_provider())
            .await?;
        let vault_data = proxy_record.get_vault_data(&state, platform).await?;
        interpolate_single_token(req.request_body.clone(), &vault_data)?
    } else {
        // ── Auto-detect mode: single or multi-token based on request body ──
        // Scan request_body for {{$field: token}} placeholders
        let all_tokens = utils::collect_tokens_from_value(&req.request_body);
        
        logger::info!(
            token_count = all_tokens.len(),
            "proxy_core: auto-detect mode - collected tokens from request body"
        );

        // Separate tokens into persistent and temporary
        let (temp_tokens, persistent_tokens): (Vec<String>, Vec<String>) = all_tokens
            .into_iter()
            .partition(|token| token.ends_with(':'));

        let unique_persistent_count = persistent_tokens
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        let unique_temp_count = temp_tokens
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        
        let total_unique_tokens = unique_persistent_count + unique_temp_count;

        logger::info!(
            total_unique = total_unique_tokens,
            persistent = unique_persistent_count,
            temporary = unique_temp_count,
            "proxy_core: token classification"
        );

        if total_unique_tokens == 0 {
            // No tokens found, return request body as-is
            logger::info!("proxy_core: no tokens found, returning request body as-is");
            req.request_body.clone()
        } else if total_unique_tokens == 1 {
            // Switch to single-token flow
            logger::info!("proxy_core: switching to single-token flow (1 unique token)");
            
            let single_token = if !persistent_tokens.is_empty() {
                persistent_tokens[0].clone()
            } else {
                temp_tokens[0].clone()
            };

            let vault_data = utils::get_vault_data_for_token(
                &state,
                &platform,
                &single_token,
                &req.token_type,
            )
            .await?;

            interpolate_multi_token_as_single(req.request_body.clone(), &single_token, &vault_data)?
        } else {
            // Multi-token flow: validate there's exactly 1 unique persistent token
            if unique_persistent_count != 1 {
                return Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!(
                        "Multi-token mode requires exactly 1 unique persistent token, found {}",
                        unique_persistent_count
                    )
                })
                .attach_printable("Invalid token configuration")?;
            }

            logger::info!(
                "proxy_core: multi-token flow ({} temp tokens, 1 persistent token)",
                unique_temp_count
            );

            // Build a map: token_value → vault_data
            let mut token_vault_map: HashMap<String, Value> = HashMap::new();
            
            // Fetch persistent token from DB (payment method table)
            for token in persistent_tokens.iter() {
                logger::info!(token = %token, "proxy_core: fetching persistent token from DB");
                let vault_data = utils::get_vault_data_for_token(
                    &state,
                    &platform,
                    token,
                    &req.token_type,
                )
                .await?;
                token_vault_map.insert(token.clone(), vault_data);
            }

            // Fetch temporary tokens from Redis
            for token in temp_tokens.iter() {
                logger::info!(token = %token, "proxy_core: fetching temporary token from Redis");
                let vault_data = utils::get_vault_data_for_token(
                    &state,
                    &platform,
                    token,
                    &req.token_type,
                )
                .await?;
                token_vault_map.insert(token.clone(), vault_data);
            }

            interpolate_multi_token(req.request_body.clone(), &token_vault_map)?
        }
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

/// Single token in multi-token format: replace `{{$field: token_value}}` with the matching field
/// from vault_data when there's only one unique token.
fn interpolate_multi_token_as_single(
    value: Value,
    token: &str,
    vault_data: &Value,
) -> RouterResult<Value> {
    match value {
        Value::Object(obj) => {
            let new_obj = obj
                .into_iter()
                .map(|(key, val)| {
                    interpolate_multi_token_as_single(val, token, vault_data).map(|v| (key, v))
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()?;
            Ok(Value::Object(new_obj))
        }
        Value::Array(arr) => {
            let new_arr = arr
                .into_iter()
                .map(|val| interpolate_multi_token_as_single(val, token, vault_data))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Array(new_arr))
        }
        Value::String(ref s) if utils::contains_multi_token(s) => {
            match utils::parse_multi_token(s) {
                Ok((_, multi_ref)) => {
                    // Verify the token matches
                    if multi_ref.token == token {
                        extract_field_from_vault_data(vault_data, &multi_ref.field)
                    } else {
                        Ok(value)
                    }
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
