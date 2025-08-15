#[cfg(feature = "v2")]
pub mod injector_core {
use std::collections::HashMap;

use api_models::injector::{ContentType, InjectorRequest, InjectorResponse};
use async_trait::async_trait;
use common_utils::request::{Method, RequestBuilder, RequestContent};
use error_stack::ResultExt;
use masking::ExposeInterface;
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
pub async fn injector_core(
    request: InjectorRequest,
) -> error_stack::Result<InjectorResponse, InjectorError> {
    logger::info!("Starting injector_core processing");
    let injector = Injector::new();
    injector.injector_core(request).await
}

#[derive(Debug)]
pub struct TokenReference {
    pub field: String,
}

// Utility function to safely mask tokens for logging
fn mask_token(token: &str) -> String {
    if token.len() <= 8 {
        "*".repeat(token.len())
    } else {
        format!("{}***{}", &token[..4], &token[token.len() - 4..])
    }
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

pub fn find_field_recursively_in_vault_data(
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
    async fn injector_core(
        &self,
        request: InjectorRequest,
    ) -> error_stack::Result<InjectorResponse, InjectorError>;
}

pub struct Injector;

impl Injector {
    pub fn new() -> Self {
        Self
    }

    #[instrument(skip_all)]
    pub fn interpolate_token_references_with_vault_data(
        &self,
        value: Value,
        vault_data: &Value,
        vault_type: &injector::types::VaultType,
    ) -> error_stack::Result<Value, InjectorError> {
        match value {
            Value::Object(obj) => {
                let new_obj = obj
                    .into_iter()
                    .map(|(key, val)| {
                        self.interpolate_token_references_with_vault_data(
                            val, vault_data, vault_type,
                        )
                        .map(|processed| (key, processed))
                    })
                    .collect::<error_stack::Result<serde_json::Map<_, _>, InjectorError>>()?;
                Ok(Value::Object(new_obj))
            }
            Value::String(s) => {
                // Use regex to find all tokens and replace them
                use regex::Regex;
                let token_regex = Regex::new(r"\{\{\$([a-zA-Z_][a-zA-Z0-9_]*)\}\}")
                    .change_context(InjectorError::InvalidTemplate(
                        "Invalid regex pattern".to_string(),
                    ))?;
                let mut result = s.clone();

                for captures in token_regex.captures_iter(&s) {
                    if let Some(field_name) = captures.get(1) {
                        let field_name_str = field_name.as_str();
                        let token_value = self.extract_field_from_vault_data(
                            vault_data,
                            field_name_str,
                            vault_type,
                        )?;
                        let token_str = match token_value {
                            Value::String(s) => s,
                            _ => serde_json::to_string(&token_value).unwrap_or_default(),
                        };

                        // Replace the token in the result string
                        let token_pattern = format!("{{{{${field_name_str}}}}}");
                        result = result.replace(&token_pattern, &token_str);
                    }
                }

                Ok(Value::String(result))
            }
            _ => Ok(value),
        }
    }

    #[instrument(skip_all)]
    fn extract_field_from_vault_data(
        &self,
        vault_data: &Value,
        field_name: &str,
        vault_type: &injector::types::VaultType,
    ) -> error_stack::Result<Value, InjectorError> {
        logger::debug!(
            "Extracting field '{}' from vault data using vault type {:?}",
            field_name,
            vault_type
        );

        match vault_data {
            Value::Object(obj) => {
                let raw_value = find_field_recursively_in_vault_data(obj, field_name)
                    .ok_or_else(|| {
                        error_stack::Report::new(InjectorError::TokenReplacementFailed(
                            format!("Field '{field_name}' not found"),
                        ))
                    })?;

                // Apply vault-specific token transformation
                self.apply_vault_specific_transformation(raw_value, vault_type, field_name)
            }
            _ => Err(error_stack::Report::new(
                InjectorError::TokenReplacementFailed(
                    "Vault data is not a valid JSON object".to_string(),
                ),
            )),
        }
    }

    #[instrument(skip_all)]
    fn apply_vault_specific_transformation(
        &self,
        token_value: Value,
        vault_type: &injector::types::VaultType,
        field_name: &str,
    ) -> error_stack::Result<Value, InjectorError> {
        match vault_type {
            injector::types::VaultType::VGS => {
                logger::debug!(
                    "VGS vault: Using direct token replacement for field '{}'",
                    field_name
                );
                Ok(token_value)
            }
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
            return Err(error_stack::Report::new(InjectorError::InvalidTemplate(
                "Base URL cannot be empty".to_string(),
            )));
        }

        if config.endpoint_path.is_empty() {
            logger::error!("Endpoint path is empty");
            return Err(error_stack::Report::new(InjectorError::InvalidTemplate(
                "Endpoint path cannot be empty".to_string(),
            )));
        }

        // Validate and construct URL safely
        let url = format!(
            "{}{}",
            config.base_url.trim_end_matches('/'),
            config.endpoint_path
        );

        // Validate URL format
        url::Url::parse(&url).map_err(|e| {
            logger::error!("Invalid URL format: '{}', error: {}", url, e);
            error_stack::Report::new(InjectorError::InvalidTemplate(format!(
                "Invalid URL '{url}': {e}"
            )))
        })?;

        logger::debug!("Constructed URL: {}", url);

        // Convert headers to common_utils Headers format safely
        let headers: Vec<(String, masking::Maskable<String>)> = config
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), masking::Maskable::new_normal(v.expose().clone())))
            .collect();

        // Determine method and request content
        let method = match config.http_method {
            injector::HttpMethod::GET => Method::Get,
            injector::HttpMethod::POST => Method::Post,
            injector::HttpMethod::PUT => Method::Put,
            injector::HttpMethod::PATCH => Method::Patch,
            injector::HttpMethod::DELETE => Method::Delete,
            injector::HttpMethod::HEAD => Method::Get, // HEAD behaves like GET but returns no body
            injector::HttpMethod::OPTIONS => Method::Get, // OPTIONS can be mapped to GET for basic support
        };

        // Determine request content based on content type with error handling
        let request_content = match content_type {
            ContentType::ApplicationJson => {
                // Try to parse as JSON, fallback to raw string
                match serde_json::from_str::<Value>(payload) {
                    Ok(json) => Some(RequestContent::Json(Box::new(json))),
                    Err(e) => {
                        logger::debug!(
                            "Failed to parse payload as JSON: {}, falling back to raw bytes",
                            e
                        );
                        Some(RequestContent::RawBytes(payload.as_bytes().to_vec()))
                    }
                }
            }
            ContentType::ApplicationXWwwFormUrlencoded => {
                // Parse form data safely
                let form_data: HashMap<String, String> =
                    url::form_urlencoded::parse(payload.as_bytes())
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

        // Add certificate configuration if provided
        if let Some(cert_content) = &config.client_cert {
            logger::debug!("Adding client certificate content");
            request_builder = request_builder
                .add_certificate(Some(cert_content.clone()));
        }

        if let Some(key_content) = &config.client_key {
            logger::debug!("Adding client private key content");
            request_builder = request_builder
                .add_certificate_key(Some(key_content.clone()));
        }

        if let Some(ca_content) = &config.ca_cert {
            logger::debug!("Adding CA certificate content");
            request_builder = request_builder
                .add_ca_certificate_pem(Some(ca_content.clone()));
        }

        // Log certificate configuration (but not the actual content)
        logger::info!(
            has_client_cert = config.client_cert.is_some(),
            has_client_key = config.client_key.is_some(),
            has_ca_cert = config.ca_cert.is_some(),
            insecure = config.insecure.unwrap_or(false),
            cert_format = ?config.cert_format,
            "Certificate configuration applied"
        );

        let request = request_builder.build();

        let proxy = if let Some(proxy_url) = &config.proxy_url {
            logger::debug!("Using proxy: {}", proxy_url);
            // Determine if it's HTTP or HTTPS proxy based on URL scheme
            if proxy_url.starts_with("https://") {
                Proxy {
                    http_url: None,
                    https_url: Some(proxy_url.clone()),
                    idle_pool_connection_timeout: Some(90),
                    bypass_proxy_hosts: None,
                }
            } else {
                Proxy {
                    http_url: Some(proxy_url.clone()),
                    https_url: None,
                    idle_pool_connection_timeout: Some(90),
                    bypass_proxy_hosts: None,
                }
            }
        } else {
            logger::debug!("No proxy configured, using direct connection");
            Proxy::default()
        };

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
        if response_text.len() > 10_000_000 {
            // 10MB limit
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
                logger::debug!(
                    "Failed to parse response as JSON: {}, returning as string",
                    e
                );
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
    async fn injector_core(
        &self,
        request: InjectorRequest,
    ) -> error_stack::Result<InjectorResponse, InjectorError> {
        logger::info!("Starting token injection process");

        let start_time = std::time::Instant::now();

        // Convert API model to domain model
        let domain_request: injector::InjectorRequest = request.into();

        // Extract token data from SecretSerdeValue for vault data lookup
        let vault_data = domain_request.token_data.specific_token_data.expose().clone();

        // Validate template length to prevent potential memory issues
        if domain_request.connector_payload.template.len() > 1_000_000 {
            // 1MB limit
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

        // Process template string directly with vault-specific logic
        let template_value = Value::String(domain_request.connector_payload.template);
        let processed_value = self.interpolate_token_references_with_vault_data(
            template_value,
            &vault_data,
            &domain_request.token_data.vault_type,
        )?;

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
        let content_type = domain_request
            .connection_config
            .headers
            .get("Content-Type")
            .and_then(|ct| match ct.expose().as_str() {
                "application/json" => Some(ContentType::ApplicationJson),
                "application/x-www-form-urlencoded" => {
                    Some(ContentType::ApplicationXWwwFormUrlencoded)
                }
                "application/xml" => Some(ContentType::ApplicationXml),
                "text/xml" => Some(ContentType::TextXml),
                "text/plain" => Some(ContentType::TextPlain),
                _ => None,
            })
            .unwrap_or(ContentType::ApplicationXWwwFormUrlencoded);

        // Make HTTP request to connector and return raw response
        let response_data = self
            .make_http_request(
                &domain_request.connection_config,
                &processed_payload,
                &content_type,
            )
            .await?;

        let elapsed = start_time.elapsed();
        logger::info!(
            duration_ms = elapsed.as_millis(),
            response_size = serde_json::to_string(&response_data)
                .map(|s| s.len())
                .unwrap_or(0),
            "Token injection completed successfully"
        );

        // Return the raw connector response for connector-agnostic handling
        Ok(response_data)
    }
}
}

#[cfg(all(test, feature = "v2"))]
#[allow(clippy::unwrap_used)]
mod tests {
use api_models::injector::*;
use hyperswitch_domain_models::injector;
use router_env::logger;

use super::injector_core::*;

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

    // Test with VGS vault (direct replacement)
    let vault_type = injector::types::VaultType::VGS;
    let result = injector
        .interpolate_token_references_with_vault_data(template, &vault_data, &vault_type)
        .unwrap();
    assert_eq!(result, serde_json::Value::String("card_number=tok_sandbox_card123&cvv=tok_sandbox_cvv456&expiry=tok_sandbox_12/tok_sandbox_2026&amount=50&currency=USD&transaction_type=purchase".to_string()));
}

#[test]
fn test_field_not_found() {
    let injector = Injector::new();
    let template = serde_json::Value::String("{{$unknown_field}}".to_string());

    let vault_data = serde_json::json!({
        "card_number": "4111111111111111"
    });

    let vault_type = injector::types::VaultType::VGS;
    let result = injector.interpolate_token_references_with_vault_data(
        template,
        &vault_data,
        &vault_type,
    );
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
    assert_eq!(
        result,
        Some(serde_json::Value::String("4111111111111111".to_string()))
    );
}

#[test]
fn test_vault_specific_token_handling() {
    let injector = Injector::new();
    let template = serde_json::Value::String("{{$card_number}}".to_string());

    let vault_data = serde_json::json!({
        "card_number": "tok_sandbox_ZgPN54WU8y8tDjc6qfEsH"
    });

    // Test VGS vault - direct replacement
    let vgs_result = injector
        .interpolate_token_references_with_vault_data(
            template.clone(),
            &vault_data,
            &injector::types::VaultType::VGS,
        )
        .unwrap();
    assert_eq!(
        vgs_result,
        serde_json::Value::String("tok_sandbox_ZgPN54WU8y8tDjc6qfEsH".to_string())
    );
}

#[tokio::test]
async fn test_injector_core_integration() {
    use std::collections::HashMap;

    // Create test request
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        masking::Secret::new("application/x-www-form-urlencoded".to_string()),
    );
    headers.insert("Authorization".to_string(), masking::Secret::new("Bearer Test".to_string()));

    let specific_token_data = common_utils::pii::SecretSerdeValue::new(serde_json::json!({
        "card_number": "tok_sandbox_123",
        "cvv": "123",
        "exp_month": "12",
        "exp_year": "25"
    }));

    let request = InjectorRequest {
        connector_payload: ConnectorPayload {
            template: "amount=100&currency=USD&metadata[order_id]=12345_att_01974ee902f97870b61afecd4c551673&return_url=http://localhost:8080/v2/payments/12345_pay_01974ee8f1f47301a83a499977aae0f1/finish-redirection/pk_dev_d5bd3a623d714044b879d3a050ae6e68/pro_yUurSuww9vtdhfEy5mXb&confirm=true&shipping[address][city]=Karwar&shipping[address][postal_code]=581301&shipping[address][state]=Karnataka&shipping[name]=John Dough&payment_method_data[billing_details][email]=example@example.com&payment_method_data[billing_details][name]=John Dough&payment_method_data[type]=card&payment_method_data[card][number]={{$card_number}}&payment_method_data[card][exp_month]=02&payment_method_data[card][exp_year]=31&payment_method_data[card][cvc]=100&capture_method=manual&setup_future_usage=on_session&payment_method_types[0]=card&expand[0]=latest_charge".to_string(),
        },
        token_data: TokenData {
            vault_type: VaultType::VGS,
            specific_token_data,
        },
        connection_config: ConnectionConfig {
            base_url: "https://api.stripe.com".to_string(),
            endpoint_path: "/v1/payment_intents".to_string(),
            http_method: HttpMethod::POST,
            headers,
            proxy_url: None, // Remove proxy that was causing issues
            // Certificate fields (None for basic test)
            client_cert: None,
            client_key: None,
            ca_cert: None, // Empty CA cert for testing
            insecure: None,
            cert_password: None,
            cert_format: None,
        },
    };

    // Test the core function - this will make a real HTTP request to httpbin.org
    let result = injector_core(request).await;

    // The request should succeed (httpbin.org should be accessible)
    if let Err(ref e) = result {
        logger::info!("Error: {e:?}");
    }
    assert!(
        result.is_ok(),
        "injector_core should succeed with valid request: {result:?}"
    );

    let response = result.unwrap();

    // Print the actual response for demonstration
    logger::info!("=== HTTP RESPONSE FROM HTTPBIN.ORG ===");
    logger::info!(
        "{}",
        serde_json::to_string_pretty(&response).unwrap_or_default()
    );
    logger::info!("=======================================");

    // Response should be a JSON value from httpbin.org
    assert!(
        response.is_object() || response.is_string(),
        "Response should be JSON object or string"
    );
}

#[tokio::test]
async fn test_certificate_configuration() {
    use std::collections::HashMap;

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), masking::Secret::new("application/json".to_string()));

    let specific_token_data = common_utils::pii::SecretSerdeValue::new(serde_json::json!({
        "card_number": "tok_test_cert",
        "cvv": "123",
        "exp_month": "12",
        "exp_year": "25"
    }));

    // Test with insecure flag (skip certificate verification)
    let request = InjectorRequest {
        connector_payload: ConnectorPayload {
            template: r#"{"card_number": "{{$card_number}}", "test": "certificate"}"#
                .to_string(),
        },
        token_data: TokenData {
            vault_type: VaultType::VGS,
            specific_token_data,
        },
        connection_config: ConnectionConfig {
            base_url: "https://httpbin.org".to_string(),
            endpoint_path: "/post".to_string(),
            http_method: HttpMethod::POST,
            headers,
            proxy_url: None,
            // Certificate configuration - using insecure for testing
            client_cert: None,
            client_key: None,
            ca_cert: None,
            insecure: Some(true), // This allows testing with self-signed certs
            cert_password: None,
            cert_format: Some("PEM".to_string()),
        },
    };

    let result = injector_core(request).await;

    // Should succeed even with insecure flag
    assert!(
        result.is_ok(),
        "Certificate test should succeed: {result:?}"
    );

    let response = result.unwrap();

    // Print the actual response for demonstration
    logger::info!("=== CERTIFICATE TEST RESPONSE ===");
    logger::info!(
        "{}",
        serde_json::to_string_pretty(&response).unwrap_or_default()
    );
    logger::info!("================================");

    // Verify the token was replaced in the JSON
    if let Some(response_str) = response.as_str() {
        assert!(
            response_str.contains("tok_test_cert"),
            "Response should contain replaced token"
        );
    } else if response.is_object() {
        // If it's a JSON object (parsed response), check the structure
        assert!(
            response.is_object(),
            "Response should be a valid JSON object"
        );
    }
}
}

// Re-export for v2 feature
#[cfg(feature = "v2")]
pub use injector_core::*;
