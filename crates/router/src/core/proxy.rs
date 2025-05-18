use super::errors::{self, RouterResponse, RouterResult};
use crate::{
    logger,
    routes::SessionState,
    services::{self, request::Mask},
    types::domain,
};
use api_models::proxy as proxy_api_models;
use common_utils::{ext_traits::BytesExt, request::{self, RequestBuilder}};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::errors::api_error_response::NotImplementedMessage;
use serde_json::Value;
use x509_parser::nom::{
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    sequence::{delimited, preceded, terminated},
    IResult,
};
use hyperswitch_interfaces::types::Response;
#[derive(Debug)]
struct TokenReference {
    field: String,
}

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
            field: field.to_string(),
        },
    ))
}

fn contains_token(s: &str) -> bool {
    s.contains("{{") && s.contains("$") && s.contains("}}")
}

fn process_value(value: Value, token: &str, vault_data: &Value) -> RouterResult<Value> {
    match value {
        Value::Object(obj) => {
            let new_obj = obj
                .into_iter()
                .map(|(key, val)| process_value(val, token, vault_data).map(|processed| (key, processed)))
                .collect::<Result<serde_json::Map<_, _>, error_stack::Report<errors::ApiErrorResponse>>>()?;

            Ok(Value::Object(new_obj))
        }
        Value::String(s) => {
            if contains_token(&s) {
                // Check if string contains multiple tokens
                if s.matches("{{").count() > 1 {
                    let mut result = s.clone();
                    let mut tokens_processed = true;

                    while result.contains("{{") && result.contains("}}") {
                        if let Some(start) = result.find("{{") {
                            let end = result[start..]
                                .find("}}")
                                .map(|pos| start + pos + 2)
                                .unwrap_or(result.len());

                            if let Ok((_, token_ref)) = parse_token(&result[start..end]) {
                                if let Ok(field_value) =
                                    extract_field_from_vault_data(vault_data, &token_ref.field)
                                {
                                    let value_str = match field_value {
                                        Value::String(s) => s,
                                        _ => field_value.to_string(),
                                    };
                                    result =
                                        result[0..start].to_string() + &value_str + &result[end..];
                                } else {
                                    tokens_processed = false;
                                    break;
                                }
                            } else {
                                tokens_processed = false;
                                break;
                            }
                        } else {
                            tokens_processed = false;
                            break;
                        }
                    }

                    Ok(tokens_processed
                        .then(|| Value::String(result))
                        .unwrap_or(Value::String(s)))
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

fn find_field_recursively_in_vault_data(obj: &serde_json::Map<String, Value>, field_name: &str) -> Option<Value> {
    obj.get(field_name)
        .cloned()
        .or_else(|| {
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
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: proxy_api_models::ProxyRequest,
) -> RouterResponse<proxy_api_models::ProxyResponse> {
    use api_models::payment_methods::PaymentMethodId;
    use common_utils::{ext_traits::OptionExt, id_type};
    let token = &req.token;

    //TODO: match on token type,
    //if token_type is tokenization id then fetch vault id from tokenization table
    //else if token_type is payment method id then fetch vault id from payment method table

    let db = state.store.as_ref();
    let vault_id = match req.token_type {
        proxy_api_models::TokenType::PaymentMethodId => {
            let pm_id = PaymentMethodId {
                payment_method_id: token.clone(),
            };
            let pm_id =
                id_type::GlobalPaymentMethodId::generate_from_string(pm_id.payment_method_id)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to generate GlobalPaymentMethodId")?;

            db
                .find_payment_method(
                    &((&state).into()),
                    &key_store,
                    &pm_id,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)?
                .locker_id
                .get_required_value("vault_id")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "vault id",
                })?
        }
        proxy_api_models::TokenType::TokenizationId => {
            Err(report!(errors::ApiErrorResponse::NotImplemented {
                message: NotImplementedMessage::Reason(
                    "Proxy flow using tokenization id".to_string(),
                ),
            }
            ))?
        }
    };

    let vault_response = super::payment_methods::vault::retrieve_payment_method_from_vault(
        &state,
        &merchant_account,
        &vault_id,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError).attach_printable("Error while fetching data from vault")?;

    let vault_data = serde_json::to_value(&vault_response.data)
        .map_err(|err| {
            logger::error!("Error serializing data to JSON value: {:?}", err);
            errors::ApiErrorResponse::InternalServerError
        })
        .attach_printable("Failed to serialize vault data")?;

    let processed_body = process_value(req.req_body.clone(), token, &vault_data)?;

    let res = execute_proxy_request(&state, &req, processed_body).await?;
    
    let proxy_response = proxy_api_models::ProxyResponse::try_from(ProxyResponseWrapper(res))?;
    
    Ok(services::ApplicationResponse::Json(proxy_response))
}
