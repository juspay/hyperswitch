use api_models::proxy as proxy_api_models;
use error_stack::ResultExt;
use x509_parser::nom::{
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    sequence::{delimited, preceded},
    IResult,
};
use async_trait::async_trait;
use common_utils::{request, ext_traits::BytesExt};

use super::errors::{self, ConnectorErrorExt, RouterResponse, RouterResult, StorageErrorExt};
use crate::{
    core::payments,
    routes::SessionState,
    services,
    types::{
        api::{self},
        domain,
    },
    utils::ConnectorResponseExt,
};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
struct TokenReference {
    full_match: String,
    field: String,
    token: String,
}

// Nom parser for token pattern
fn parse_token(input: &str) -> IResult<&str, TokenReference> {
    let (input, _) = tag("{{")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('$')(input)?;
    
    let (input, field) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, _) = tag(":")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, token) = take_while1(|c: char| c.is_alphanumeric() || c == '-')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("}}")(input)?;

    Ok((input, TokenReference {
        full_match: format!("{{{{ ${}:{} }}}}", field, token),
        field: field.to_string(),
        token: token.to_string(),
    }))
}

// Check if a string contains a token pattern
fn contains_token(s: &str) -> bool {
    s.contains("{{") && s.contains("$") && s.contains("}}") && s.contains(":")
}

// Process a JSON value recursively
async fn process_value(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    value: Value,
    token_cache: &mut HashMap<String, Value>,
) -> RouterResult<Value> {
    match value {
        Value::Object(obj) => {
            // Process each field in the object
            let mut new_obj = serde_json::Map::new();
            
            for (key, val) in obj {
                // Box the recursive call to avoid infinite size
                let processed = Box::pin(process_value(state, merchant_account, val, token_cache)).await?;
                new_obj.insert(key, processed);
            }
            
            Ok(Value::Object(new_obj))
        },
        Value::String(s) => {
            if contains_token(&s) {
                // Try to parse the token
                if let Ok((_, token_ref)) = parse_token(&s) {
                    // Check cache first
                    if let Some(cached_value) = token_cache.get(&token_ref.token) {
                        // If the field exists in the cached value, return that field
                        if let Value::Object(obj) = cached_value {
                            if let Some(field_value) = obj.get(&token_ref.field) {
                                return Ok(field_value.clone());
                            }
                        }
                        return Ok(cached_value.clone());
                    }
                    
                    let vault_id = domain::VaultId::generate(token_ref.token.clone()); 
                    
                    let vault_response = super::payment_methods::vault::retrieve_payment_method_from_vault(
                        state,
                        merchant_account,
                        &vault_id,
                    )
                    .await
                    .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;
                    
                    // Cache the result
                    let vault_data_value = serde_json::to_value(&vault_response.data)
                        .map_err(|_| errors::ApiErrorResponse::InternalServerError)?;
                    token_cache.insert(token_ref.token.clone(), vault_data_value.clone());
                    
                    // Extract the specific field if needed
                    if let Value::Object(obj) = &vault_data_value {
                        if let Some(field_value) = obj.get(&token_ref.field) {
                            return Ok(field_value.clone());
                        }
                    }
                    
                    return Ok(vault_data_value);
                }
            }
            Ok(Value::String(s))
        },
        // For other value types, return as is
        _ => Ok(value),
    }
}

pub async fn proxy_core(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    req: proxy_api_models::ProxyRequest,
) -> RouterResponse<proxy_api_models::ProxyResponse> {
    // Process the request body recursively
    let mut token_cache = HashMap::new();
    let processed_body = process_value(&state, &merchant_account, req.req_body, &mut token_cache).await?;

    // Create request for call_connector_api
    let mut request = services::Request::new(services::Method::Post, &req.destination_url);
    request.set_body(request::RequestContent::Json(Box::new(processed_body)));

    // Make the API call using call_connector_api
    let response = services::call_connector_api(&state, request, "proxy")
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError);

    let res = response
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while receiving response")
        .and_then(|inner| match inner {
            Err(err_res) => {
                Err(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(format!("Response Deserialization Failed: {err_res:?}"))
            }
            Ok(res) => Ok(res),
        })
        .inspect_err(|_| {})?;

    let response_body: Value = res
        .response
        .parse_struct("ProxyResponse")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(services::ApplicationResponse::Json(
        proxy_api_models::ProxyResponse {
            response: response_body,
        },
    ))
}
