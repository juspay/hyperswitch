#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use api_models::proxy as proxy_api_models;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::{ext_traits::BytesExt, request};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use error_stack::ResultExt;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use x509_parser::nom::{
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    sequence::{delimited, preceded, terminated},
    IResult,
};

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use super::errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::{
    core::payments,
    logger,
    routes::SessionState,
    services::{self, request::Mask},
    types::{
        api::{self},
        domain,
    },
    utils::ConnectorResponseExt,
};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use serde_json::Value;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug)]
struct TokenReference {
    full_match: String,
    field: String,
    token: String,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
fn parse_token(input: &str) -> IResult<&str, TokenReference> {
    let (input, field) = delimited(
        tag("{{"),
        preceded(
            multispace0,
            preceded(
                char('$'),
                terminated(
                    take_while1(|c: char| c.is_alphanumeric() || c == '_'),
                    multispace0,
                ),
            ),
        ),
        tag("}}"),
    )(input)?;

    Ok((
        input,
        TokenReference {
            full_match: format!("{{{{ ${} }}}}", field),
            field: field.to_string(),
            token: String::new(),
        },
    ))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
fn contains_token(s: &str) -> bool {
    s.contains("{{") && s.contains("$") && s.contains("}}")
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
async fn process_value(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    value: Value,
    token: &str,
    vault_data: &Value,
) -> RouterResult<Value> {
    match value {
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();

            for (key, val) in obj {
                let processed = Box::pin(process_value(
                    state,
                    merchant_account,
                    val,
                    token,
                    vault_data,
                ))
                .await?;
                new_obj.insert(key, processed);
            }

            Ok(Value::Object(new_obj))
        }
        Value::String(s) => {
            if contains_token(&s) {
                // Check if string contains multiple tokens
                if s.matches("{{").count() > 1 {
                    let mut result = s.clone();
                    let mut tokens_processed = true;
                    
                    while result.contains("{{") && result.contains("}}") {
                        let start = result.find("{{").unwrap();
                        let end = result[start..].find("}}").map(|pos| start + pos + 2).unwrap_or(result.len());
                        
                        if let Ok((_, token_ref)) = parse_token(&result[start..end]) {
                            if let Ok(field_value) = extract_field_from_vault_data(vault_data, &token_ref.field) {
                                let value_str = match field_value {
                                    Value::String(s) => s,
                                    _ => field_value.to_string(),
                                };
                                result = result[0..start].to_string() + &value_str + &result[end..];
                            } else {
                                tokens_processed = false;
                                break;
                            }
                        } else {
                            tokens_processed = false;
                            break;
                        }
                    }
                    
                    if tokens_processed {
                        Ok(Value::String(result))
                    } else {
                        Ok(Value::String(s))
                    }
                } else {
                    if let Ok((_, token_ref)) = parse_token(&s) {
                        extract_field_from_vault_data(vault_data, &token_ref.field)
                    } else {
                        Ok(Value::String(s))
                    }
                }
            } else {
                Ok(Value::String(s))
            }
        }
        _ => Ok(value),
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
fn extract_field_from_vault_data(vault_data: &Value, field_name: &str) -> RouterResult<Value> {
    let result = match vault_data {
        Value::Object(obj) => {
            obj.get(field_name).cloned().or_else(|| {
                obj.values()
                    .filter_map(|val| {
                        if let Value::Object(inner_obj) = val {
                            inner_obj.get(field_name).cloned().or_else(|| {
                                inner_obj
                                    .values()
                                    .filter_map(|deeper_val| {
                                        if let Value::Object(deepest_obj) = deeper_val {
                                            deepest_obj.get(field_name).cloned()
                                        } else {
                                            None
                                        }
                                    })
                                    .next()
                            })
                        } else {
                            None
                        }
                    })
                    .next()
            })
        }
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub async fn proxy_core(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    req: proxy_api_models::ProxyRequest,
) -> RouterResponse<proxy_api_models::ProxyResponse> {
    let token = &req.token;
    let vault_id = domain::VaultId::generate(token.clone());

    let vault_response = super::payment_methods::vault::retrieve_payment_method_from_vault(
        &state,
        &merchant_account,
        &vault_id,
    )
    .await
    .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;

    let vault_data = serde_json::to_value(&vault_response.data)
        .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;


    let processed_body =
        process_value(&state, &merchant_account, req.req_body, token, &vault_data).await?;

    let mut request = services::Request::new(services::Method::Post, &req.destination_url);
    request.set_body(request::RequestContent::Json(Box::new(processed_body)));

    if let Value::Object(headers) = req.headers {
        headers.iter().for_each(|(key, value)| {
            let header_value = match value {
                Value::String(value_str) => value_str.clone(),
                _ => value.to_string(),
            }
            .into_masked();
            request.add_header(key, header_value);
        });
    }

    let response = services::call_connector_api(&state, request, "proxy")
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError);
    let res = response
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(format!("Response Deserialization Failed: {err_res:?}")), //move it to 2xx
            Ok(res) => Ok(res),
        })
        .inspect_err(|_| {})?;

    let response_body: Value = res
        .response
        .parse_struct("ProxyResponse")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(services::ApplicationResponse::Json(
        proxy_api_models::ProxyResponse {
            response: response_body,//send status code, response headers
        },
    ))
}
