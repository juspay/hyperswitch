#[cfg(feature = "v2")]
pub mod injector_core {
    use api_models::injector::{ContentType, InjectorRequest, InjectorResponse};
    use async_trait::async_trait;
    use common_utils::{
        request::{Method, RequestBuilder, RequestContent},
    };
    use error_stack::ResultExt;
    use external_services::http_client;
    use hyperswitch_domain_models::injector;
    use hyperswitch_interfaces::types::Proxy;
    use masking;
    use nom::{
        bytes::complete::{tag, take_while1},
        character::complete::{char, multispace0},
        sequence::{delimited, preceded, terminated},
        IResult,
    };
    use router_env::{instrument, logger, tracing};
    use serde_json::Value;
    use std::collections::HashMap;
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum InjectorError {
        #[error("Token replacement failed: {0}")]
        TokenReplacementFailed(String),
        #[error("HTTP request failed")]
        HttpRequestFailed,
        #[error("Serialization error: {0}")]
        SerializationError(String),
        #[error("Invalid template: {0}")]
        InvalidTemplate(String),
    }


    #[instrument(skip_all)]
    pub async fn injector_core(request: InjectorRequest) -> error_stack::Result<InjectorResponse, InjectorError> {
        logger::info!("Starting injector_core processing");
        let injector = Injector::new();
        injector.injector_core(request).await
    }

    #[derive(Debug)]
    pub struct TokenReference {
        pub field: String,
    }

    pub fn parse_token(input: &str) -> IResult<&str, TokenReference> {
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

    #[async_trait]
    pub trait TokenInjector {
        async fn injector_core(&self, request: InjectorRequest) -> error_stack::Result<InjectorResponse, InjectorError>;
    }

    pub struct Injector;

    impl Injector {
        pub fn new() -> Self {
            Self
        }

        #[instrument(skip_all)]
        fn interpolate_token_references_with_vault_data(
            &self,
            value: Value,
            vault_data: &Value,
        ) -> error_stack::Result<Value, InjectorError> {
            match value {
                Value::Object(obj) => {
                    let new_obj = obj
                        .into_iter()
                        .map(|(key, val)| {
                            self.interpolate_token_references_with_vault_data(val, vault_data)
                                .map(|processed| (key, processed))
                        })
                        .collect::<error_stack::Result<serde_json::Map<_, _>, InjectorError>>()?;
                    Ok(Value::Object(new_obj))
                }
                Value::String(s) => {
                    if let Ok((_, token_ref)) = parse_token(&s) {
                        self.extract_field_from_vault_data(vault_data, &token_ref.field)
                    } else {
                        Ok(Value::String(s))
                    }
                }
                _ => Ok(value),
            }
        }

        #[instrument(skip_all)]
        fn extract_field_from_vault_data(&self, vault_data: &Value, field_name: &str) -> error_stack::Result<Value, InjectorError> {
            logger::debug!("Extracting field '{}' from vault data", field_name);
            match vault_data {
                Value::Object(obj) => {
                    find_field_recursively_in_vault_data(obj, field_name)
                        .ok_or_else(|| {
                            error_stack::Report::new(InjectorError::TokenReplacementFailed(format!("Field '{field_name}' not found")))
                        })
                }
                _ => Err(error_stack::Report::new(InjectorError::TokenReplacementFailed(
                    "Vault data is not a valid JSON object".to_string(),
                ))),
            }
        }

        #[instrument(skip_all)]
        async fn make_http_request(
            &self,
            config: &injector::ConnectionConfig,
            payload: &str,
            content_type: &ContentType,
        ) -> error_stack::Result<Value, InjectorError> {
            logger::info!(
                method = ?config.http_method,
                base_url = %config.base_url,
                endpoint = %config.endpoint_path,
                content_type = ?content_type,
                payload_length = payload.len(),
                headers_count = config.headers.len(),
                "Making HTTP request to connector"
            );
            // Validate inputs first
            if config.base_url.is_empty() {
                logger::error!("Base URL is empty");
                return Err(error_stack::Report::new(InjectorError::InvalidTemplate("Base URL cannot be empty".to_string())));
            }
            
            if config.endpoint_path.is_empty() {
                logger::error!("Endpoint path is empty");
                return Err(error_stack::Report::new(InjectorError::InvalidTemplate("Endpoint path cannot be empty".to_string())));
            }
            
            // Validate and construct URL safely
            let url = format!("{}{}", config.base_url.trim_end_matches('/'), config.endpoint_path);
            
            // Validate URL format
            url::Url::parse(&url)
                .map_err(|e| {
                    logger::error!("Invalid URL format: '{}', error: {}", url, e);
                    error_stack::Report::new(InjectorError::InvalidTemplate(format!("Invalid URL '{}': {}", url, e)))
                })?;
            
            logger::debug!("Constructed URL: {}", url);
            
            // Convert headers to common_utils Headers format safely
            let headers: Vec<(String, masking::Maskable<String>)> = config.headers
                .iter()
                .map(|(k, v)| (k.clone(), masking::Maskable::new_normal(v.clone())))
                .collect();

            // Determine method and request content
            let method = match config.http_method {
                injector::HttpMethod::Get => Method::Get,
                injector::HttpMethod::Post => Method::Post,
                injector::HttpMethod::Put => Method::Put,
                injector::HttpMethod::Patch => Method::Patch,
                injector::HttpMethod::Delete => Method::Delete,
                _ => return Err(error_stack::Report::new(InjectorError::InvalidTemplate("Unsupported HTTP method".to_string()))),
            };

            // Determine request content based on content type with error handling
            let request_content = match content_type {
                ContentType::ApplicationJson => {
                    // Try to parse as JSON, fallback to raw string
                    match serde_json::from_str::<Value>(payload) {
                        Ok(json) => Some(RequestContent::Json(Box::new(json))),
                        Err(e) => {
                            logger::debug!("Failed to parse payload as JSON: {}, falling back to raw bytes", e);
                            Some(RequestContent::RawBytes(payload.as_bytes().to_vec()))
                        }
                    }
                }
                ContentType::ApplicationXWwwFormUrlencoded => {
                    // Parse form data safely
                    let form_data: HashMap<String, String> = url::form_urlencoded::parse(payload.as_bytes())
                        .into_owned()
                        .collect();
                    Some(RequestContent::FormUrlEncoded(Box::new(form_data)))
                }
                ContentType::ApplicationXml | ContentType::TextXml => {
                    Some(RequestContent::RawBytes(payload.as_bytes().to_vec()))
                }
                ContentType::TextPlain => {
                    Some(RequestContent::RawBytes(payload.as_bytes().to_vec()))
                }
            };

            // Build request safely
            let mut request_builder = RequestBuilder::new()
                .method(method)
                .url(&url)
                .headers(headers);
            
            if let Some(content) = request_content {
                request_builder = request_builder.set_body(content);
            }
            
            let request = request_builder.build();

            // Use default proxy for now
            let proxy = Proxy::default();
            
            // Send request using external_services http_client
            logger::debug!("Sending HTTP request to connector");
            let response = http_client::send_request(&proxy, request, None)
                .await
                .change_context(InjectorError::HttpRequestFailed)?;
            
            logger::info!(
                status_code = response.status().as_u16(),
                "Received response from connector"
            );

            let response_text = response
                .text()
                .await
                .change_context(InjectorError::HttpRequestFailed)?;

            // Validate response text length to prevent potential memory issues
            if response_text.len() > 10_000_000 {  // 10MB limit
                logger::warn!("Response text is very large: {} bytes", response_text.len());
                return Ok(Value::String("Response too large".to_string()));
            }

            logger::debug!(
                response_length = response_text.len(),
                "Processing connector response"
            );

            // Try to parse as JSON, fallback to string value with error logging
            match serde_json::from_str::<Value>(&response_text) {
                Ok(json) => {
                    logger::debug!("Successfully parsed response as JSON");
                    Ok(json)
                }
                Err(e) => {
                    logger::debug!("Failed to parse response as JSON: {}, returning as string", e);
                    Ok(Value::String(response_text))
                }
            }
        }
    }

    impl Default for Injector {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl TokenInjector for Injector {
        #[instrument(skip_all)]
        async fn injector_core(&self, request: InjectorRequest) -> error_stack::Result<InjectorResponse, InjectorError> {
            logger::info!("Starting token injection process");
            
            let start_time = std::time::Instant::now();
            
            // Convert API model to domain model
            let domain_request: injector::InjectorRequest = request.into();
            
            // Convert token data to JSON for vault data lookup with validation
            let vault_data = serde_json::to_value(&domain_request.token_data.specific_token_data)
                .change_context(InjectorError::SerializationError(
                    "Failed to serialize token data".to_string(),
                ))?;
            
            // Validate template length to prevent potential memory issues
            if domain_request.connector_payload.template.len() > 1_000_000 {  // 1MB limit
                logger::error!(
                    template_length = domain_request.connector_payload.template.len(),
                    "Template is too large"
                );
                return Err(error_stack::Report::new(InjectorError::InvalidTemplate(
                    "Template is too large".to_string(),
                )));
            }
            
            logger::debug!(
                template_length = domain_request.connector_payload.template.len(),
                vault_type = ?domain_request.token_data.vault_type,
                "Processing token injection request"
            );
            
            // Process template string directly
            let template_value = Value::String(domain_request.connector_payload.template);
            let processed_value = self.interpolate_token_references_with_vault_data(template_value, &vault_data)?;
            
            let processed_payload = match processed_value {
                Value::String(s) => s,
                _ => {
                    // This shouldn't happen since we started with a string
                    return Err(error_stack::Report::new(InjectorError::InvalidTemplate(
                        "Template processing resulted in non-string value".to_string(),
                    )));
                }
            };
            
            logger::debug!(
                processed_payload_length = processed_payload.len(),
                "Token replacement completed"
            );
            
            // Determine content type from headers or default to form-urlencoded
            let content_type = domain_request.connection_config.headers
                .get("Content-Type")
                .and_then(|ct| match ct.as_str() {
                    "application/json" => Some(ContentType::ApplicationJson),
                    "application/x-www-form-urlencoded" => Some(ContentType::ApplicationXWwwFormUrlencoded),
                    "application/xml" => Some(ContentType::ApplicationXml),
                    "text/xml" => Some(ContentType::TextXml),
                    "text/plain" => Some(ContentType::TextPlain),
                    _ => None,
                })
                .unwrap_or(ContentType::ApplicationXWwwFormUrlencoded);
            
            // Make HTTP request to connector
            match self.make_http_request(&domain_request.connection_config, &processed_payload, &content_type).await {
                Ok(response_data) => {
                    let elapsed = start_time.elapsed();
                    logger::info!(
                        duration_ms = elapsed.as_millis(),
                        "Token injection completed successfully"
                    );
                    Ok(InjectorResponse {
                        success: true,
                        message: "Token injection completed successfully".to_string(),
                        processed_payload: Some(processed_payload),
                        response_data: Some(response_data),
                    })
                }
                Err(e) => {
                    let elapsed = start_time.elapsed();
                    logger::error!(
                        duration_ms = elapsed.as_millis(),
                        error = ?e,
                        "Token injection failed"
                    );
                    Ok(InjectorResponse {
                        success: false,
                        message: format!("Token injection failed: {}", e),
                        processed_payload: Some(processed_payload),
                        response_data: None,
                    })
                }
            }
        }
    }
}

#[cfg(all(test, feature = "v2"))]
mod tests {
    use super::injector_core::*;
    use api_models::injector::*;

    #[test]
    fn test_token_parsing() {
        let result = parse_token("{{$card_number}}");
        assert!(result.is_ok());
        let (_, token_ref) = result.unwrap();
        assert_eq!(token_ref.field, "card_number");
    }

    #[test]
    fn test_token_interpolation() {
        let injector = Injector::new();
        let template = serde_json::Value::String("card_number={{$card_number}}&cvv={{$cvv}}&expiry={{$exp_month}}/{{$exp_year}}&amount=50&currency=USD&transaction_type=purchase".to_string());
        
        let vault_data = serde_json::json!({
            "card_number": "tok_sandbox_card123",
            "cvv": "tok_sandbox_cvv456", 
            "exp_month": "tok_sandbox_12",
            "exp_year": "tok_sandbox_2026"
        });
        
        let result = injector.interpolate_token_references_with_vault_data(template, &vault_data).unwrap();
        assert_eq!(result, serde_json::Value::String("card_number=tok_sandbox_card123&cvv=tok_sandbox_cvv456&expiry=tok_sandbox_12/tok_sandbox_2026&amount=50&currency=USD&transaction_type=purchase".to_string()));
    }

    #[test]
    fn test_field_not_found() {
        let injector = Injector::new();
        let template = serde_json::Value::String("{{$unknown_field}}".to_string());
        
        let vault_data = serde_json::json!({
            "card_number": "4111111111111111"
        });
        
        let result = injector.interpolate_token_references_with_vault_data(template, &vault_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_recursive_field_search() {
        let vault_data = serde_json::json!({
            "payment_method": {
                "card": {
                    "number": "4111111111111111"
                }
            }
        });
        
        let obj = vault_data.as_object().unwrap();
        let result = find_field_recursively_in_vault_data(obj, "number");
        assert_eq!(result, Some(serde_json::Value::String("4111111111111111".to_string())));
    }
}

// Re-export for v2 feature
#[cfg(feature = "v2")]
pub use injector_core::*;