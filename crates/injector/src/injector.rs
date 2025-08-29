pub mod core {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use common_utils::request::{Method, RequestBuilder, RequestContent};
    use error_stack::ResultExt;
    use masking::{self, ExposeInterface};
    use nom::{
        bytes::complete::{tag, take_while1},
        character::complete::{char, multispace0},
        sequence::{delimited, preceded, terminated},
        IResult,
    };
    use router_env::{instrument, logger, tracing};
    use serde_json::Value;
    use thiserror::Error;

    use crate as injector_types;
    use crate::{ContentType, InjectorRequest, InjectorResponse};

    impl From<injector_types::HttpMethod> for Method {
        fn from(method: injector_types::HttpMethod) -> Self {
            match method {
                injector_types::HttpMethod::GET => Self::Get,
                injector_types::HttpMethod::POST => Self::Post,
                injector_types::HttpMethod::PUT => Self::Put,
                injector_types::HttpMethod::PATCH => Self::Patch,
                injector_types::HttpMethod::DELETE => Self::Delete,
            }
        }
    }

    /// Proxy configuration structure (copied from hyperswitch_interfaces to make injector standalone)
    #[derive(Debug, serde::Deserialize, Clone)]
    #[serde(default)]
    pub struct Proxy {
        /// The URL of the HTTP proxy server.
        pub http_url: Option<String>,
        /// The URL of the HTTPS proxy server.
        pub https_url: Option<String>,
        /// The timeout duration (in seconds) for idle connections in the proxy pool.
        pub idle_pool_connection_timeout: Option<u64>,
        /// A comma-separated list of hosts that should bypass the proxy.
        pub bypass_proxy_hosts: Option<String>,
    }

    impl Default for Proxy {
        fn default() -> Self {
            Self {
                http_url: Default::default(),
                https_url: Default::default(),
                idle_pool_connection_timeout: Some(90),
                bypass_proxy_hosts: Default::default(),
            }
        }
    }

    /// Simplified HTTP client for injector (copied from external_services to make injector standalone)
    /// This is a minimal implementation that covers the essential functionality needed by injector
    #[instrument(skip_all)]
    pub async fn send_request(
        client_proxy: &Proxy,
        request: common_utils::request::Request,
        _option_timeout_secs: Option<u64>,
    ) -> error_stack::Result<reqwest::Response, InjectorError> {
        logger::info!("Making HTTP request using standalone injector HTTP client");

        // Create reqwest client with proxy configuration
        let mut client_builder = reqwest::Client::builder();

        // Configure proxy if provided
        if let Some(proxy_url) = &client_proxy.https_url {
            let proxy = reqwest::Proxy::https(proxy_url).map_err(|e| {
                logger::error!("Failed to configure HTTPS proxy: {}", e);
                error_stack::Report::new(InjectorError::HttpRequestFailed)
            })?;
            client_builder = client_builder.proxy(proxy);
        }

        if let Some(proxy_url) = &client_proxy.http_url {
            let proxy = reqwest::Proxy::http(proxy_url).map_err(|e| {
                logger::error!("Failed to configure HTTP proxy: {}", e);
                error_stack::Report::new(InjectorError::HttpRequestFailed)
            })?;
            client_builder = client_builder.proxy(proxy);
        }

        let client = client_builder.build().map_err(|e| {
            logger::error!("Failed to build HTTP client: {}", e);
            error_stack::Report::new(InjectorError::HttpRequestFailed)
        })?;

        // Build the request
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
        };

        let mut req_builder = client.request(method, &request.url);

        // Add headers
        for (key, value) in &request.headers {
            let header_value = match value {
                masking::Maskable::Masked(secret) => secret.clone().expose(),
                masking::Maskable::Normal(normal) => normal.clone(),
            };
            req_builder = req_builder.header(key, header_value);
        }

        // Add body if present
        if let Some(body) = request.body {
            match body {
                RequestContent::Json(payload) => {
                    req_builder = req_builder.json(&payload);
                }
                RequestContent::FormUrlEncoded(payload) => {
                    req_builder = req_builder.form(&payload);
                }
                RequestContent::RawBytes(payload) => {
                    req_builder = req_builder.body(payload);
                }
                _ => {
                    logger::warn!("Unsupported request content type, using raw bytes");
                }
            }
        }

        // Send the request
        let response = req_builder.send().await.map_err(|e| {
            logger::error!("HTTP request failed: {}", e);
            error_stack::Report::new(InjectorError::HttpRequestFailed)
        })?;

        Ok(response)
    }

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

    /// Represents a token reference found in a template string
    #[derive(Debug)]
    pub struct TokenReference {
        /// The field name to be replaced (without the {{$}} wrapper)
        pub field: String,
    }

    /// Parses a single token reference from a string using nom parser combinators
    ///
    /// Expects tokens in the format `{{$field_name}}` where field_name contains
    /// only alphanumeric characters and underscores.
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

    /// Finds all token references in a string using nom parser
    ///
    /// Scans through the entire input string and extracts all valid token references.
    /// Returns a vector of TokenReference structs containing the field names.
    pub fn find_all_tokens(input: &str) -> Vec<TokenReference> {
        let mut tokens = Vec::new();
        let mut current_input = input;

        while !current_input.is_empty() {
            if let Ok((remaining, token_ref)) = parse_token(current_input) {
                tokens.push(token_ref);
                current_input = remaining;
            } else {
                // Move forward one character if no token found
                if let Some((_, rest)) = current_input.split_at_checked(1) {
                    current_input = rest;
                } else {
                    break;
                }
            }
        }

        tokens
    }

    /// Recursively searches for a field in vault data JSON structure
    ///
    /// Performs a depth-first search through the JSON object hierarchy to find
    /// a field with the specified name. Returns the first matching value found.
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

        /// Processes a string template and replaces token references with vault data
        #[instrument(skip_all)]
        pub fn interpolate_string_template_with_vault_data(
            &self,
            template: String,
            vault_data: &Value,
            vault_connector: &injector_types::VaultConnectors,
        ) -> error_stack::Result<String, InjectorError> {
            // Find all tokens using nom parser
            let tokens = find_all_tokens(&template);
            let mut result = template;

            for token_ref in tokens.into_iter() {
                let extracted_field_value = self.extract_field_from_vault_data(
                    vault_data,
                    &token_ref.field,
                    vault_connector,
                )?;
                let token_str = match extracted_field_value {
                    Value::String(token_value) => token_value,
                    _ => serde_json::to_string(&extracted_field_value).unwrap_or_default(),
                };

                // Replace the token in the result string
                let token_pattern = format!("{{{{${}}}}}", token_ref.field);
                result = result.replace(&token_pattern, &token_str);
            }

            Ok(result)
        }

        #[instrument(skip_all)]
        pub fn interpolate_token_references_with_vault_data(
            &self,
            value: Value,
            vault_data: &Value,
            vault_connector: &injector_types::VaultConnectors,
        ) -> error_stack::Result<Value, InjectorError> {
            match value {
                Value::Object(obj) => {
                    let new_obj = obj
                        .into_iter()
                        .map(|(key, val)| {
                            self.interpolate_token_references_with_vault_data(
                                val,
                                vault_data,
                                vault_connector,
                            )
                            .map(|processed| (key, processed))
                        })
                        .collect::<error_stack::Result<serde_json::Map<_, _>, InjectorError>>()?;
                    Ok(Value::Object(new_obj))
                }
                Value::String(s) => {
                    let processed_string = self.interpolate_string_template_with_vault_data(
                        s,
                        vault_data,
                        vault_connector,
                    )?;
                    Ok(Value::String(processed_string))
                }
                _ => Ok(value),
            }
        }

        #[instrument(skip_all)]
        fn extract_field_from_vault_data(
            &self,
            vault_data: &Value,
            field_name: &str,
            vault_connector: &injector_types::VaultConnectors,
        ) -> error_stack::Result<Value, InjectorError> {
            logger::debug!(
                "Extracting field '{}' from vault data using vault type {:?}",
                field_name,
                vault_connector
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
                    self.apply_vault_specific_transformation(raw_value, vault_connector, field_name)
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
            extracted_field_value: Value,
            vault_connector: &injector_types::VaultConnectors,
            field_name: &str,
        ) -> error_stack::Result<Value, InjectorError> {
            match vault_connector {
                injector_types::VaultConnectors::VGS => {
                    logger::debug!(
                        "VGS vault: Using direct token replacement for field '{}'",
                        field_name
                    );
                    Ok(extracted_field_value)
                }
            }
        }

        #[instrument(skip_all)]
        async fn make_http_request(
            &self,
            config: &injector_types::DomainConnectionConfig,
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
            if config.endpoint_path.is_empty() {
                logger::error!("Endpoint path is empty");
                Err(error_stack::Report::new(InjectorError::InvalidTemplate(
                    "Endpoint path cannot be empty".to_string(),
                )))?;
            }

            // Construct URL safely by joining base URL with endpoint path
            let url = config.base_url.join(&config.endpoint_path).map_err(|e| {
                logger::error!("Failed to join base URL with endpoint path: {}", e);
                error_stack::Report::new(InjectorError::InvalidTemplate(format!(
                    "Invalid URL construction: {e}"
                )))
            })?;

            logger::debug!("Constructed URL: {}", url);

            // Convert headers to common_utils Headers format safely
            let headers: Vec<(String, masking::Maskable<String>)> = config
                .headers
                .clone()
                .into_iter()
                .map(|(k, v)| (k, masking::Maskable::new_normal(v.expose().clone())))
                .collect();

            // Determine method and request content
            let method = Method::from(config.http_method);

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
                .url(url.as_str())
                .headers(headers);

            if let Some(content) = request_content {
                request_builder = request_builder.set_body(content);
            }

            // Add certificate configuration if provided
            if let Some(cert_content) = &config.client_cert {
                logger::debug!("Adding client certificate content");
                request_builder = request_builder.add_certificate(Some(cert_content.clone()));
            }

            if let Some(key_content) = &config.client_key {
                logger::debug!("Adding client private key content");
                request_builder = request_builder.add_certificate_key(Some(key_content.clone()));
            }

            if let Some(ca_content) = &config.ca_cert {
                logger::debug!("Adding CA certificate content");
                request_builder = request_builder.add_ca_certificate_pem(Some(ca_content.clone()));
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
                if proxy_url.scheme() == "https" {
                    Proxy {
                        http_url: None,
                        https_url: Some(proxy_url.to_string()),
                        idle_pool_connection_timeout: Some(90),
                        bypass_proxy_hosts: None,
                    }
                } else {
                    Proxy {
                        http_url: Some(proxy_url.to_string()),
                        https_url: None,
                        idle_pool_connection_timeout: Some(90),
                        bypass_proxy_hosts: None,
                    }
                }
            } else {
                logger::debug!("No proxy configured, using direct connection");
                Proxy::default()
            };

            // Send request using local standalone http client
            logger::debug!("Sending HTTP request to connector");
            let response = send_request(&proxy, request, None).await?;

            logger::info!(
                status_code = response.status().as_u16(),
                "Received response from connector"
            );

            let response_text = response
                .text()
                .await
                .change_context(InjectorError::HttpRequestFailed)?;

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
            let domain_request: injector_types::DomainInjectorRequest = request.into();

            // Extract token data from SecretSerdeValue for vault data lookup
            let vault_data = domain_request
                .token_data
                .specific_token_data
                .expose()
                .clone();

            logger::debug!(
                template_length = domain_request.connector_payload.template.len(),
                vault_connector = ?domain_request.token_data.vault_connector,
                "Processing token injection request"
            );

            // Process template string directly with vault-specific logic
            let processed_payload = self.interpolate_string_template_with_vault_data(
                domain_request.connector_payload.template,
                &vault_data,
                &domain_request.token_data.vault_connector,
            )?;

            logger::debug!(
                processed_payload_length = processed_payload.len(),
                "Token replacement completed"
            );

            // Determine content type from headers or default to form-urlencoded
            let content_type = domain_request
                .connection_config
                .headers
                .get("Content-Type")
                .and_then(|ct| match ct.clone().expose().as_str() {
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

// Re-export all items
pub use core::*;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use router_env::logger;

    use super::core::*;
    use crate::*;

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
            "card_number": "TEST_card123",
            "cvv": "TEST_cvv456",
            "exp_month": "TEST_12",
            "exp_year": "TEST_2026"
        });

        // Test with VGS vault (direct replacement)
        let vault_connector = VaultConnectors::VGS;
        let result = injector
            .interpolate_token_references_with_vault_data(template, &vault_data, &vault_connector)
            .unwrap();
        assert_eq!(result, serde_json::Value::String("card_number=TEST_card123&cvv=TEST_cvv456&expiry=TEST_12/TEST_2026&amount=50&currency=USD&transaction_type=purchase".to_string()));
    }

    #[test]
    fn test_field_not_found() {
        let injector = Injector::new();
        let template = serde_json::Value::String("{{$unknown_field}}".to_string());

        let vault_data = serde_json::json!({
            "card_number": "TEST_CARD_NUMBER"
        });

        let vault_connector = VaultConnectors::VGS;
        let result = injector.interpolate_token_references_with_vault_data(
            template,
            &vault_data,
            &vault_connector,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_recursive_field_search() {
        let vault_data = serde_json::json!({
            "payment_method": {
                "card": {
                    "number": "TEST_CARD_NUMBER"
                }
            }
        });

        let obj = vault_data.as_object().unwrap();
        let result = find_field_recursively_in_vault_data(obj, "number");
        assert_eq!(
            result,
            Some(serde_json::Value::String("TEST_CARD_NUMBER".to_string()))
        );
    }

    #[test]
    fn test_vault_specific_token_handling() {
        let injector = Injector::new();
        let template = serde_json::Value::String("{{$card_number}}".to_string());

        let vault_data = serde_json::json!({
            "card_number": "TOKEN"
        });

        // Test VGS vault - direct replacement
        let vgs_result = injector
            .interpolate_token_references_with_vault_data(
                template.clone(),
                &vault_data,
                &VaultConnectors::VGS,
            )
            .unwrap();
        assert_eq!(vgs_result, serde_json::Value::String("TOKEN".to_string()));
    }

    #[tokio::test]
    #[ignore = "Integration test that requires network access"]
    async fn test_injector_core_integration() {
        use std::collections::HashMap;

        // Create test request
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            masking::Secret::new("application/x-www-form-urlencoded".to_string()),
        );
        headers.insert(
            "Authorization".to_string(),
            masking::Secret::new("Bearer Test".to_string()),
        );

        let specific_token_data = common_utils::pii::SecretSerdeValue::new(serde_json::json!({
            "card_number": "TEST_123",
            "cvv": "123",
            "exp_month": "12",
            "exp_year": "25"
        }));

        let request = InjectorRequest {
            connector_payload: ConnectorPayload {
                template: "card_number={{$card_number}}&cvv={{$cvv}}&expiry={{$exp_month}}/{{$exp_year}}&amount=50&currency=USD&transaction_type=purchase".to_string(),
            },
            token_data: TokenData {
                vault_connector: VaultConnectors::VGS,
                specific_token_data,
            },
            connection_config: ConnectionConfig {
                base_url: "https://api.stripe.com".parse().unwrap(),
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
                max_response_size: None, // Use default
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
    #[ignore = "Integration test that requires network access"]
    async fn test_certificate_configuration() {
        use std::collections::HashMap;

        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            masking::Secret::new("application/x-www-form-urlencoded".to_string()),
        );
        headers.insert(
            "Authorization".to_string(),
            masking::Secret::new("Bearer API_KEY".to_string()),
        );

        let specific_token_data = common_utils::pii::SecretSerdeValue::new(serde_json::json!({
            "card_number": "TOKEN",
            "cvv": "123",
            "exp_month": "12",
            "exp_year": "25"
        }));

        // Test with insecure flag (skip certificate verification)
        let request = InjectorRequest {
            connector_payload: ConnectorPayload {
                template: "card_number={{$card_number}}&cvv={{$cvv}}&expiry={{$exp_month}}/{{$exp_year}}&amount=50&currency=USD&transaction_type=purchase".to_string(),
            },
            token_data: TokenData {
                vault_connector: VaultConnectors::VGS,
                specific_token_data,
            },
            connection_config: ConnectionConfig {
                base_url: "https://api.stripe.com".parse().unwrap(),
                endpoint_path: "/v1/payment_intents".to_string(),
                http_method: HttpMethod::POST,
                headers,
                proxy_url: Some("https://proxy.example.com:8443".parse().unwrap()),
                // Certificate configuration - using insecure for testing
                client_cert: None,
                client_key: None,
                ca_cert: Some(masking::Secret::new("CERT".to_string())),
                insecure: None, // This allows testing with self-signed certs
                cert_password: None,
                cert_format: None,
                max_response_size: None, // Use default
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
        // httpbin.org returns the request data in the 'data' or 'json' field
        let response_contains_token = if let Some(response_str) = response.as_str() {
            response_str.contains("TOKEN")
        } else if response.is_object() {
            // Check if the response contains our token in the request data
            let response_str = serde_json::to_string(&response).unwrap_or_default();
            response_str.contains("TOKEN")
        } else {
            false
        };

        assert!(
            response_contains_token,
            "Response should contain replaced token: {}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );
    }
}
