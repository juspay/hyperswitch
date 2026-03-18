pub mod core {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use common_utils::request::{Method, RequestBuilder, RequestContent};
    use error_stack::{self, ResultExt};
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
    use crate::{
        metrics,
        types::{ContentType, InjectorRequest, InjectorResponse, IntoInjectorResponse},
        vault_metadata::VaultMetadataExtractorExt,
    };

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

    /// Create HTTP client using the proven external_services create_client logic
    fn create_client(
        proxy_config: &Proxy,
        client_certificate: Option<masking::Secret<String>>,
        client_certificate_key: Option<masking::Secret<String>>,
        ca_certificate: Option<masking::Secret<String>>,
    ) -> error_stack::Result<reqwest::Client, InjectorError> {
        logger::debug!(
            has_client_cert = client_certificate.is_some(),
            has_client_key = client_certificate_key.is_some(),
            has_ca_cert = ca_certificate.is_some(),
            "Creating HTTP client"
        );

        // Case 1: Mutual TLS with client certificate and key
        if let (Some(encoded_certificate), Some(encoded_certificate_key)) =
            (client_certificate.clone(), client_certificate_key.clone())
        {
            if ca_certificate.is_some() {
                logger::warn!("All of client certificate, client key, and CA certificate are provided. CA certificate will be ignored in mutual TLS setup.");
            }

            let client_builder = get_client_builder(proxy_config)?;

            let identity = create_identity_from_certificate_and_key(
                encoded_certificate.clone(),
                encoded_certificate_key,
            )?;

            let certificate_list = create_certificate(encoded_certificate)?;
            let client_builder = certificate_list
                .into_iter()
                .fold(client_builder, |client_builder, certificate| {
                    client_builder.add_root_certificate(certificate)
                });
            return client_builder
                .identity(identity)
                .use_rustls_tls()
                .build()
                .change_context(InjectorError::HttpRequestFailed)
                .inspect_err(|e| {
                    logger::error!(
                        "Failed to construct client with certificate and certificate key: {:?}",
                        e
                    );
                });
        }

        // Case 2: Use provided CA certificate for server authentication only (one-way TLS)
        if let Some(ca_pem) = ca_certificate {
            let pem = ca_pem.expose().replace("\\r\\n", "\n"); // Fix escaped newlines
            let cert = reqwest::Certificate::from_pem(pem.as_bytes())
                .change_context(InjectorError::HttpRequestFailed)
                .inspect_err(|e| {
                    logger::error!("Failed to parse CA certificate PEM block: {:?}", e)
                })?;
            let client_builder = get_client_builder(proxy_config)?.add_root_certificate(cert);
            return client_builder
                .use_rustls_tls()
                .build()
                .change_context(InjectorError::HttpRequestFailed)
                .inspect_err(|e| {
                    logger::error!("Failed to construct client with CA certificate: {:?}", e);
                });
        }

        // Case 3: Default client (no certs)
        get_base_client(proxy_config)
    }

    /// Helper functions from external_services
    fn get_client_builder(
        proxy_config: &Proxy,
    ) -> error_stack::Result<reqwest::ClientBuilder, InjectorError> {
        let mut client_builder = reqwest::Client::builder();

        // Configure proxy if provided
        if let Some(proxy_url) = &proxy_config.https_url {
            let proxy = reqwest::Proxy::https(proxy_url)
                .change_context(InjectorError::HttpRequestFailed)
                .inspect_err(|e| {
                    logger::error!("Failed to configure HTTPS proxy: {:?}", e);
                })?;
            client_builder = client_builder.proxy(proxy);
        }

        if let Some(proxy_url) = &proxy_config.http_url {
            let proxy = reqwest::Proxy::http(proxy_url)
                .change_context(InjectorError::HttpRequestFailed)
                .inspect_err(|e| {
                    logger::error!("Failed to configure HTTP proxy: {:?}", e);
                })?;
            client_builder = client_builder.proxy(proxy);
        }

        Ok(client_builder)
    }

    fn get_base_client(
        proxy_config: &Proxy,
    ) -> error_stack::Result<reqwest::Client, InjectorError> {
        let client_builder = get_client_builder(proxy_config)?;
        client_builder
            .build()
            .change_context(InjectorError::HttpRequestFailed)
            .inspect_err(|e| {
                logger::error!("Failed to build default HTTP client: {:?}", e);
            })
    }

    fn create_identity_from_certificate_and_key(
        encoded_certificate: masking::Secret<String>,
        encoded_certificate_key: masking::Secret<String>,
    ) -> error_stack::Result<reqwest::Identity, InjectorError> {
        let cert_str = encoded_certificate.expose();
        let key_str = encoded_certificate_key.expose();

        let combined_pem = format!("{cert_str}\n{key_str}");
        reqwest::Identity::from_pem(combined_pem.as_bytes())
            .change_context(InjectorError::HttpRequestFailed)
            .inspect_err(|e| {
                logger::error!(
                    "Failed to create identity from certificate and key: {:?}",
                    e
                );
            })
    }

    fn create_certificate(
        encoded_certificate: masking::Secret<String>,
    ) -> error_stack::Result<Vec<reqwest::Certificate>, InjectorError> {
        let cert_str = encoded_certificate.expose();

        let cert = reqwest::Certificate::from_pem(cert_str.as_bytes())
            .change_context(InjectorError::HttpRequestFailed)
            .inspect_err(|e| {
                logger::error!("Failed to create certificate from PEM: {:?}", e);
            })?;
        Ok(vec![cert])
    }

    /// Generic function to log HTTP request errors with detailed error type information
    fn log_and_convert_http_error(e: reqwest::Error, context: &str) -> InjectorError {
        let error_msg = e.to_string();
        logger::error!("HTTP request failed in {}: {}", context, error_msg);

        // Log specific error types for debugging
        if e.is_timeout() {
            logger::error!("Request timed out in {}", context);
        }
        if e.is_connect() {
            logger::error!("Connection error occurred in {}", context);
        }
        if e.is_request() {
            logger::error!("Request construction error in {}", context);
        }
        if e.is_decode() {
            logger::error!("Response decoding error in {}", context);
        }

        InjectorError::HttpRequestFailed
    }

    /// Apply certificate configuration to request builder and return built request
    fn build_request_with_certificates(
        mut request_builder: RequestBuilder,
        config: &injector_types::ConnectionConfig,
    ) -> common_utils::request::Request {
        // Add certificate configuration if provided
        if let Some(cert_content) = &config.client_cert {
            request_builder = request_builder.add_certificate(Some(cert_content.clone()));
        }

        if let Some(key_content) = &config.client_key {
            request_builder = request_builder.add_certificate_key(Some(key_content.clone()));
        }

        if let Some(ca_content) = &config.ca_cert {
            request_builder = request_builder.add_ca_certificate_pem(Some(ca_content.clone()));
        }

        request_builder.build()
    }

    /// Simplified HTTP client for injector using the proven external_services create_client logic
    #[instrument(skip_all)]
    pub async fn send_request(
        client_proxy: &Proxy,
        request: common_utils::request::Request,
        _option_timeout_secs: Option<u64>,
    ) -> error_stack::Result<reqwest::Response, InjectorError> {
        logger::info!(
            has_client_cert = request.certificate.is_some(),
            has_client_key = request.certificate_key.is_some(),
            has_ca_cert = request.ca_certificate.is_some(),
            "Making HTTP request using standalone injector HTTP client with configuration"
        );

        // Create reqwest client using the proven create_client function
        let client = create_client(
            client_proxy,
            request.certificate.clone(),
            request.certificate_key.clone(),
            request.ca_certificate.clone(),
        )?;

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
        logger::debug!("=== REQUEST HEADERS ({} total) ===", request.headers.len());
        for (key, value) in &request.headers {
            let header_value = match value {
                masking::Maskable::Masked(secret) => secret.clone().expose(),
                masking::Maskable::Normal(normal) => normal.clone(),
            };
            logger::debug!("  Header: {} = {}", key, header_value);
            req_builder = req_builder.header(key, header_value);
        }
        logger::debug!("=== END REQUEST HEADERS ===");

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
        let response = req_builder
            .send()
            .await
            .map_err(|e| log_and_convert_http_error(e, "send_request"))?;

        logger::info!(
            status_code = response.status().as_u16(),
            "HTTP request completed successfully"
        );

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
        let start_time = std::time::Instant::now();
        logger::info!("Starting injector_core processing");

        // Extract values for metrics before moving request
        let vault_connector_str = format!("{:?}", request.connection_config.vault_connector_id);
        let http_method_str = format!("{:?}", request.connection_config.http_method);

        // Track total number of invocations with vault connector dimension
        metrics::INJECTOR_INVOCATIONS_COUNT.add(
            1,
            router_env::metric_attributes!(("vault_connector", vault_connector_str.clone())),
        );

        // Extract endpoint host for dimension (privacy-friendly)
        let endpoint_host = request
            .connection_config
            .endpoint
            .parse::<url::Url>()
            .map(|url| url.host_str().unwrap_or("unknown").to_string())
            .unwrap_or_else(|_| "invalid_url".to_string());

        let injector = Injector::new();
        let result = injector.injector_core(request).await;

        // Record total request time and track success/failure
        let request_duration = start_time.elapsed();

        let base_attributes = router_env::metric_attributes!(
            ("vault_connector", vault_connector_str.clone()),
            ("http_method", http_method_str.clone()),
            ("endpoint_host", endpoint_host.clone())
        );

        metrics::INJECTOR_REQUEST_TIME.record(request_duration.as_secs_f64(), base_attributes);

        // Track success/failure metrics
        result.inspect_err(|e| {
            logger::error!("Injector core failed: {:?}", e);
            metrics::INJECTOR_FAILED_TOKEN_REPLACEMENTS_COUNT.add(1, base_attributes);
        })
    }

    /// Represents a token reference found in a template string
    #[derive(Debug)]
    struct TokenReference {
        /// The field name to be replaced (without the {{$}} wrapper)
        pub field: String,
    }

    /// Parses a single token reference from a string using nom parser combinators
    ///
    /// Expects tokens in the format `{{$field_name}}` where field_name contains
    /// only alphanumeric characters and underscores.
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

    /// Finds all token references in a string using nom parser
    ///
    /// Scans through the entire input string and extracts all valid token references.
    /// Returns a vector of TokenReference structs containing the field names.
    fn find_all_tokens(input: &str) -> Vec<TokenReference> {
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
    trait TokenInjector {
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

        /// Processes a string template by replacing {{$field_name}} placeholders with
        /// corresponding values from vault data (token aliases).
        #[instrument(skip_all)]
        pub(crate) fn interpolate_string_template_with_vault_data(
            &self,
            template: String,
            vault_data: &Value,
        ) -> error_stack::Result<String, InjectorError> {
            let token_replacement_start = std::time::Instant::now();
            // Find all tokens using nom parser
            let tokens = find_all_tokens(&template);
            let mut result = template;

            for token_ref in tokens.into_iter() {
                let extracted_field_value =
                    self.extract_field_from_vault_data(vault_data, &token_ref.field)?;
                let token_str = match extracted_field_value {
                    Value::String(token_value) => token_value,
                    _ => serde_json::to_string(&extracted_field_value).unwrap_or_default(),
                };

                // Replace the token in the result string
                let token_pattern = format!("{{{{${}}}}}", token_ref.field);
                result = result.replace(&token_pattern, &token_str);
            }

            // Record token replacement time
            let token_replacement_duration = token_replacement_start.elapsed();
            metrics::INJECTOR_TOKEN_REPLACEMENT_TIME.record(
                token_replacement_duration.as_secs_f64(),
                router_env::metric_attributes!(("operation", "interpolation")),
            );

            Ok(result)
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
                    let processed_string =
                        self.interpolate_string_template_with_vault_data(s, vault_data)?;
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
        ) -> error_stack::Result<Value, InjectorError> {
            logger::debug!("Extracting field '{}' from vault data", field_name,);

            match vault_data {
                Value::Object(obj) => find_field_recursively_in_vault_data(obj, field_name)
                    .ok_or_else(|| {
                        error_stack::Report::new(InjectorError::TokenReplacementFailed(format!(
                            "Field '{field_name}' not found"
                        )))
                    }),
                _ => Err(error_stack::Report::new(
                    InjectorError::TokenReplacementFailed(
                        "Vault data is not a valid JSON object".to_string(),
                    ),
                )),
            }
        }

        #[instrument(skip_all)]
        async fn make_http_request(
            &self,
            config: &injector_types::ConnectionConfig,
            payload: &str,
            content_type: &ContentType,
        ) -> error_stack::Result<InjectorResponse, InjectorError> {
            logger::info!(
                method = ?config.http_method,
                endpoint = %config.endpoint,
                content_type = ?content_type,
                payload_length = payload.len(),
                headers_count = config.headers.len(),
                "Making HTTP request to connector"
            );
            // Validate inputs first
            if config.endpoint.is_empty() {
                logger::error!("Endpoint URL is empty");
                Err(error_stack::Report::new(InjectorError::InvalidTemplate(
                    "Endpoint URL cannot be empty".to_string(),
                )))?;
            }

            // Parse and validate the complete endpoint URL
            let url = reqwest::Url::parse(&config.endpoint).map_err(|e| {
                logger::error!("Failed to parse endpoint URL: {}", e);
                error_stack::Report::new(InjectorError::InvalidTemplate(format!(
                    "Invalid endpoint URL: {e}"
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

            // Extract vault metadata directly from headers using existing functions

            let (vault_proxy_url, vault_ca_cert) = if config
                .headers
                .contains_key(crate::consts::EXTERNAL_VAULT_METADATA_HEADER)
            {
                let mut temp_config = injector_types::ConnectionConfig::new(
                    config.endpoint.clone(),
                    config.http_method,
                );

                // Use existing vault metadata extraction with fallback
                if temp_config.extract_and_apply_vault_metadata_with_fallback(&config.headers) {
                    (temp_config.proxy_url, temp_config.ca_cert)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Build request safely with certificate configuration
            let mut request_builder = RequestBuilder::new()
                .method(method)
                .url(url.as_str())
                .headers(headers);

            if let Some(content) = request_content {
                request_builder = request_builder.set_body(content);
            }

            // Create final config with vault CA certificate if available
            let mut final_config = config.clone();
            let has_vault_ca_cert = vault_ca_cert.is_some();
            if has_vault_ca_cert {
                final_config.ca_cert = vault_ca_cert;
            }

            // Log certificate configuration (but not the actual content)
            logger::info!(
                has_client_cert = final_config.client_cert.is_some(),
                has_client_key = final_config.client_key.is_some(),
                has_ca_cert = final_config.ca_cert.is_some(),
                has_vault_ca_cert = has_vault_ca_cert,
                insecure = final_config.insecure.unwrap_or(false),
                cert_format = ?final_config.cert_format,
                "Certificate configuration applied"
            );

            // Build request with certificate configuration applied
            let request = build_request_with_certificates(request_builder, &final_config);

            // Determine which proxy to use: vault metadata > backup > none
            let final_proxy_url = vault_proxy_url.or_else(|| config.backup_proxy_url.clone());

            let proxy = if let Some(proxy_url) = final_proxy_url {
                let proxy_url_str = proxy_url.expose();

                // Set proxy URL for both HTTP and HTTPS traffic
                Proxy {
                    http_url: Some(proxy_url_str.clone()),
                    https_url: Some(proxy_url_str),
                    idle_pool_connection_timeout: Some(90),
                    bypass_proxy_hosts: None,
                }
            } else {
                Proxy::default()
            };
            logger::debug!("Vault Proxy URL: {}", url);

            // Track outgoing HTTP calls with dimensions
            let endpoint_host = config
                .endpoint
                .parse::<url::Url>()
                .map(|url| url.host_str().unwrap_or("unknown").to_string())
                .unwrap_or_else(|_| "invalid_url".to_string());

            metrics::INJECTOR_OUTGOING_CALLS_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("http_method", format!("{:?}", config.http_method)),
                    ("endpoint_host", endpoint_host)
                ),
            );

            // Log the full outgoing request details
            logger::debug!("=== OUTGOING HTTP REQUEST ===");
            logger::debug!("  URL: {}", request.url);
            logger::debug!("  Method: {:?}", request.method);
            logger::debug!("  Headers count: {}", request.headers.len());
            for (key, value) in &request.headers {
                let header_value = match value {
                    masking::Maskable::Masked(secret) => secret.clone().expose(),
                    masking::Maskable::Normal(normal) => normal.clone(),
                };
                logger::debug!("  Header: {} = {}", key, header_value);
            }
            if let Some(ref body) = request.body {
                match body {
                    RequestContent::Json(json) => {
                        logger::debug!("  Body (JSON): {}", serde_json::to_string_pretty(json.as_ref()).unwrap_or_default());
                    }
                    RequestContent::FormUrlEncoded(form) => {
                        logger::debug!("  Body (Form): {:?}", form);
                    }
                    RequestContent::RawBytes(bytes) => {
                        logger::debug!("  Body (Raw): {} bytes", bytes.len());
                        if let Ok(text) = std::str::from_utf8(bytes) {
                            logger::debug!("  Body (Raw text): {}", text);
                        }
                    }
                    _ => {
                        logger::debug!("  Body: (other content type)");
                    }
                }
            } else {
                logger::debug!("  Body: None");
            }
            logger::debug!("  has_certificate: {}", request.certificate.is_some());
            logger::debug!("  has_certificate_key: {}", request.certificate_key.is_some());
            logger::debug!("  has_ca_certificate: {}", request.ca_certificate.is_some());
            logger::debug!("  Proxy HTTP URL: {:?}", proxy.http_url);
            logger::debug!("  Proxy HTTPS URL: {:?}", proxy.https_url);
            logger::debug!("=== END OUTGOING HTTP REQUEST ===");

            // Send request using local standalone http client
            let response = send_request(&proxy, request, None).await?;

            // Convert reqwest::Response to InjectorResponse using trait
            response
                .into_injector_response()
                .await
                .map_err(|e| error_stack::Report::new(e))
        }

        /// Constructs and sends a request to the HyperswitchVault proxy endpoint.
        ///
        /// Unlike `make_http_request` (which sends the processed payload directly to the connector),
        /// this method wraps the template (with {{$field_name}} placeholders intact) into the
        /// HyperswitchVault proxy request format and sends it to the vault endpoint.
        /// The vault resolves placeholders with actual card data and forwards to the connector.
        #[instrument(skip_all)]
        async fn make_hyperswitch_vault_request(
            &self,
            request: &InjectorRequest,
            processed_payload: &str,
        ) -> error_stack::Result<InjectorResponse, InjectorError> {
            let vault_endpoint = request
                .connection_config
                .vault_endpoint
                .as_ref()
                .ok_or_else(|| {
                    error_stack::Report::new(InjectorError::InvalidTemplate(
                        "vault_endpoint is required for HyperswitchVault proxy request".to_string(),
                    ))
                })?;

            let vault_auth = request
                .connection_config
                .vault_auth_data
                .as_ref()
                .ok_or_else(|| {
                    error_stack::Report::new(InjectorError::InvalidTemplate(
                        "vault_auth_data is required for HyperswitchVault proxy request"
                            .to_string(),
                    ))
                })?;

            // Build the HS Vault proxy request body from injector request components
            let vault_proxy_request =
                injector_types::HyperswitchVaultProxyRequest::try_from_injector_request(
                    processed_payload,
                    &request.connection_config,
                    &request.token_data,
                )
                .map_err(error_stack::Report::new)?;

            let vault_request_body = serde_json::to_string(&vault_proxy_request).map_err(|e| {
                error_stack::Report::new(InjectorError::SerializationError(format!(
                    "Failed to serialize HyperswitchVault proxy request: {e}"
                )))
            })?;

            logger::info!(
                vault_endpoint = %vault_endpoint,
                destination_url = %vault_proxy_request.destination_url,
                method = %vault_proxy_request.method,
                "Sending request to HyperswitchVault proxy"
            );

            // Build headers for the vault endpoint request
            let vault_headers: Vec<(String, masking::Maskable<String>)> = vec![
                (
                    "Content-Type".to_string(),
                    masking::Maskable::new_normal("application/json".to_string()),
                ),
                (
                    "Accept".to_string(),
                    masking::Maskable::new_normal("application/json".to_string()),
                ),
                (
                    "Authorization".to_string(),
                    masking::Maskable::Masked(
                        format!("api-key={}", vault_auth.api_key.clone().expose()).into(),
                    ),
                ),
                (
                    "x-profile-id".to_string(),
                    masking::Maskable::Masked(vault_auth.api_secret.clone()),
                ),
            ];

            // Build the HTTP request to the vault endpoint
            let vault_url = reqwest::Url::parse(vault_endpoint).map_err(|e| {
                logger::error!("Failed to parse vault endpoint URL: {}", e);
                error_stack::Report::new(InjectorError::InvalidTemplate(format!(
                    "Invalid vault endpoint URL: {e}"
                )))
            })?;

            let request_builder = RequestBuilder::new()
                .method(Method::Post)
                .url(vault_url.as_str())
                .headers(vault_headers)
                .set_body(RequestContent::Json(Box::new(
                    serde_json::from_str::<Value>(&vault_request_body).map_err(|e| {
                        error_stack::Report::new(InjectorError::SerializationError(format!(
                            "Failed to parse vault request body as JSON: {e}"
                        )))
                    })?,
                )));

            let http_request = request_builder.build();

            // Track outgoing call to vault
            let vault_host = vault_url.host_str().unwrap_or("unknown").to_string();

            metrics::INJECTOR_OUTGOING_CALLS_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("http_method", "POST".to_string()),
                    ("endpoint_host", vault_host)
                ),
            );

            // Send to vault using default proxy (no VGS proxy needed)
            let response = send_request(&Proxy::default(), http_request, None).await?;

            response
                .into_injector_response()
                .await
                .map_err(|e| error_stack::Report::new(e))
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
            let start_time = std::time::Instant::now();

            // Extract token data from SecretSerdeValue for vault data lookup
            let vault_data = request.token_data.specific_token_data.clone().expose();

            let vault_connector_type = &request.connection_config.vault_connector_type;

            logger::debug!(
                template_length = request.connector_payload.template.len(),
                vault_connector_type = ?vault_connector_type,
                "Processing token injection request"
            );

            // Process template based on vault connector type:
            //
            // Template arrives with {{$field_name}} placeholders, e.g.:
            //   "card_number={{$card_number}}&cvv={{$cvv}}"
            //
            // specific_token_data contains token aliases from the vault, e.g.:
            //   VGS:              {"card_number": "tok_sandbox_abc", "cvv": "tok_sandbox_xyz"}
            //   HyperswitchVault:  {"card_number": "token_1", "cvv": "token_1"}
            //
            // Proxy | None (e.g. VGS):
            //   Interpolate placeholders with token aliases directly.
            //   Output: "card_number=tok_sandbox_abc&cvv=tok_sandbox_xyz"
            //   VGS forward proxy then detokenizes the aliases to real card data on the wire.
            //
            // Transformation (e.g. HyperswitchVault):
            //   Return template as-is with {{$field_name}} placeholders intact.
            //   Output: "card_number={{$card_number}}&cvv={{$cvv}}"
            //   HyperswitchVault resolves placeholders downstream with actual card data.
            let processed_payload = match vault_connector_type {
                Some(injector_types::VaultConnectorType::Proxy) | None => {
                    // Proxy vault: interpolate {{$field_name}} placeholders with token aliases
                    // The forward proxy (e.g. VGS) detokenizes the aliases on the wire
                    logger::debug!("Proxy vault: interpolating template with token aliases");
                    self.interpolate_string_template_with_vault_data(
                        request.connector_payload.template.clone(),
                        &vault_data,
                    )?
                }
                Some(injector_types::VaultConnectorType::Transformation) => {
                    let vault_connector_id = &request.connection_config.vault_connector_id;
                    match vault_connector_id {
                        Some(injector_types::VaultConnectors::HyperswitchVault) => {
                            // HyperswitchVault: return template as-is with {{$field_name}} placeholders
                            // HS Vault resolves these downstream with actual card data
                            logger::debug!(
                                "HyperswitchVault transformation: skipping interpolation, keeping placeholders"
                            );
                            request.connector_payload.template.clone()
                        }
                        _ => {
                            // Future transformation connectors: placeholder for additional logic
                            // For now, return template as-is
                            logger::debug!(
                                vault_connector_id = ?vault_connector_id,
                                "Transformation vault (non-HyperswitchVault): no transformation implemented yet, keeping placeholders"
                            );
                            request.connector_payload.template.clone()
                        }
                    }
                }
            };

            logger::debug!(
                processed_payload_length = processed_payload.len(),
                "Token replacement completed"
            );
            logger::debug!("Processedd payload: {:?}", processed_payload);

            // Determine content type from headers or default to form-urlencoded
            let content_type = request
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

            // Make HTTP request — routing depends on vault connector type:
            //
            // Proxy | None (e.g. VGS):
            //   Send processed payload (with token aliases) directly to the connector endpoint.
            //   The forward proxy detokenizes the aliases to real card data on the wire.
            //
            // Transformation + HyperswitchVault:
            //   Wrap the template (with {{$field_name}} placeholders intact) into the HS Vault
            //   proxy request format and send to the vault endpoint. The vault resolves
            //   placeholders with actual card data and forwards to the connector.
            let response = match vault_connector_type {
                Some(injector_types::VaultConnectorType::Transformation) => {
                    match &request.connection_config.vault_connector_id {
                        Some(injector_types::VaultConnectors::HyperswitchVault) => {
                            logger::debug!("Routing request through HyperswitchVault proxy");
                            self.make_hyperswitch_vault_request(&request, &processed_payload)
                                .await?
                        }
                        other => {
                            logger::debug!(
                                vault_connector_id = ?other,
                                "Transformation vault (non-HyperswitchVault): falling back to direct HTTP request"
                            );
                            self.make_http_request(
                                &request.connection_config,
                                &processed_payload,
                                &content_type,
                            )
                            .await?
                        }
                    }
                }
                Some(injector_types::VaultConnectorType::Proxy) | None => {
                    self.make_http_request(
                        &request.connection_config,
                        &processed_payload,
                        &content_type,
                    )
                    .await?
                }
            };

            let elapsed = start_time.elapsed();
            logger::info!(
                duration_ms = elapsed.as_millis(),
                status_code = response.status_code,
                response_size = serde_json::to_string(&response.response)
                    .map(|s| s.len())
                    .unwrap_or(0),
                headers_count = response.headers.as_ref().map(|h| h.len()).unwrap_or(0),
                "Token injection completed successfully"
            );

            // Track successful token replacements with comprehensive dimensions
            let endpoint_host = request
                .connection_config
                .endpoint
                .parse::<url::Url>()
                .map(|url| url.host_str().unwrap_or("unknown").to_string())
                .unwrap_or_else(|_| "invalid_url".to_string());

            let vault_connector_str = format!("{:?}", request.connection_config.vault_connector_id);
            let http_method_str = format!("{:?}", request.connection_config.http_method);

            metrics::INJECTOR_SUCCESSFUL_TOKEN_REPLACEMENTS_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("status_code", response.status_code.to_string()),
                    ("vault_connector", vault_connector_str),
                    ("http_method", http_method_str),
                    ("endpoint_host", endpoint_host)
                ),
            );

            Ok(response)
        }
    }
}

// Re-export all items
pub use core::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use router_env::logger;

    use crate::*;

    #[tokio::test]
    #[ignore = "Integration test that requires network access"]
    async fn test_injector_core_integration() {
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
                specific_token_data,
            },
            connection_config: ConnectionConfig {
                endpoint: "https://api.stripe.com/v1/payment_intents".to_string(),
                http_method: HttpMethod::POST,
                headers,
                proxy_url: None, // Remove proxy that was causing issues
                backup_proxy_url: None,
                // Certificate fields (None for basic test)
                client_cert: None,
                client_key: None,
                ca_cert: None, // Empty CA cert for testing
                insecure: None,
                cert_password: None,
                cert_format: None,
                max_response_size: None, // Use default
                vault_auth_data: None,
                vault_connector_id: Some(VaultConnectors::VGS), // Use VGS as example vault connector
                vault_connector_type: Some(VaultConnectorType::Proxy), // Use Proxy type for direct interpolation
                vault_endpoint: None, // No vault endpoint needed for proxy test

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

        // Response should have a proper status code and response data
        assert!(
            response.status_code >= 200 && response.status_code < 300,
            "Response should have successful status code: {}",
            response.status_code
        );

        assert!(
            response.response.is_object() || response.response.is_string(),
            "Response data should be JSON object or string"
        );
    }

    #[tokio::test]
    async fn test_certificate_configuration() {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            masking::Secret::new("application/x-www-form-urlencoded".to_string()),
        );
        headers.insert(
            "Authorization".to_string(),
            masking::Secret::new("Bearer TEST".to_string()),
        );

        let specific_token_data = common_utils::pii::SecretSerdeValue::new(serde_json::json!({
            "card_number": "4242429789164242",
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
                specific_token_data,
            },
            connection_config: ConnectionConfig {
                endpoint: "https://httpbin.org/post".to_string(),
                http_method: HttpMethod::POST,
                headers,
                proxy_url: None, // Remove proxy to make test work reliably
                backup_proxy_url: None,
                // Test without certificates for basic functionality
                client_cert: None,
                client_key: None,
                ca_cert: None,
                insecure: None,
                cert_password: None,
                cert_format: None,
                max_response_size: None,
                vault_auth_data: None,
                vault_connector_id: Some(VaultConnectors::VGS), // Use VGS as example vault connector
                vault_connector_type: Some(VaultConnectorType::Proxy), // Use Proxy type for direct interpolation
                vault_endpoint: None, // No vault endpoint needed for proxy test
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

        // Should succeed with proper status code
        assert!(
            response.status_code >= 200 && response.status_code < 300,
            "Certificate test should have successful status code: {}",
            response.status_code
        );

        // Verify the tokens were replaced correctly in the form data
        // httpbin.org returns the request data in the 'form' field
        let response_str = serde_json::to_string(&response.response).unwrap_or_default();

        // Check that our test tokens were replaced with the actual values from vault data
        let tokens_replaced = response_str.contains("4242429789164242") && // card_number
                              response_str.contains("123") &&               // cvv
                              response_str.contains("12/25"); // expiry

        assert!(
            tokens_replaced,
            "Response should contain replaced tokens (card_number, cvv, expiry): {}",
            serde_json::to_string_pretty(&response.response).unwrap_or_default()
        );
    }

    /// End-to-end test simulating the real VGS Checkout.com injector request.
    /// Logs all secret values (token data, interpolated payload, final connector payload)
    /// to verify correct template interpolation without making a real HTTP call.
    #[tokio::test]
    async fn test_vgs_checkout_interpolation_end_to_end() {
        use crate::injector::core::Injector;
        use masking::ExposeInterface;

        // ── Token data exactly as received from UCS ──
        let specific_token_data = common_utils::pii::SecretSerdeValue::new(serde_json::json!({
            "card_number": "tok_sandbox_2YDNnDoani5L3EEV5RqrMF",
            "card_cvc": "737",
            "card_exp_month": "12",
            "card_exp_year": "31"
        }));

        // ── Template exactly as built by UCS for Checkout.com ──
        let template = r#"{"source":{"type":"card","number":"{{$card_number}}","expiry_month":"{{$card_exp_month}}","expiry_year":"{{$card_exp_year}}","cvv":"{{$card_cvc}}","billing_address":{},"account_holder":{"first_name":"John","last_name":"Doe"}},"amount":100,"currency":"USD","processing_channel_id":"pc_jx5lvimg4obe7nhoqnhptm6xoq","3ds":{"enabled":false,"force_3ds":false,"eci":null,"cryptogram":null,"xid":null,"version":null,"challenge_indicator":"no_preference"},"success_url":"http://localhost:8080/v2/payments/12345_pay_019cfbbc8a057611860ef2d51243ba7c/finish-redirection/pk_dev_79f3ea3dccc44e8cbd6f5c19840d2268/pro_ePpSh8JjJSsrxmXTP51U?status=success","failure_url":"http://localhost:8080/v2/payments/12345_pay_019cfbbc8a057611860ef2d51243ba7c/finish-redirection/pk_dev_79f3ea3dccc44e8cbd6f5c19840d2268/pro_ePpSh8JjJSsrxmXTP51U?status=failure","capture":true,"reference":"12345_att_019cfbbce54e7340b089f7aca93b4f71","metadata":{},"payment_type":"Regular","merchant_initiated":false}"#.to_string();

        // ── Connection config matching real VGS request ──
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            masking::Secret::new("application/json".to_string()),
        );
        headers.insert(
            "via".to_string(),
            masking::Secret::new("HyperSwitch".to_string()),
        );
        headers.insert(
            "Authorization".to_string(),
            masking::Secret::new(
                "Bearer test".to_string(),
            ),
        );

        let ca_cert_pem = "-----BEGIN CERTIFICATE-----\nMIID2TCCAsGgAwIBAgIHAN4Gs/LGhzANBgkqhkiG9w0BAQ0FADB5MSQwIgYDVQQD\nDBsqLnNhbmRib3gudmVyeWdvb2Rwcm94eS5jb20xITAfBgNVBAoMGFZlcnkgR29v\nZCBTZWN1cml0eSwgSW5jLjEuMCwGA1UECwwlVmVyeSBHb29kIFNlY3VyaXR5IC0g\nRW5naW5lZXJpbmcgVGVhbTAgFw0xNjAyMDkyMzUzMzZaGA8yMTE3MDExNTIzNTMz\nNloweTEkMCIGA1UEAwwbKi5zYW5kYm94LnZlcnlnb29kcHJveHkuY29tMSEwHwYD\nVQQKDBhWZXJ5IEdvb2QgU2VjdXJpdHksIEluYy4xLjAsBgNVBAsMJVZlcnkgR29v\nZCBTZWN1cml0eSAtIEVuZ2luZWVyaW5nIFRlYW0wggEiMA0GCSqGSIb3DQEBAQUA\nA4IBDwAwggEKAoIBAQDI3ukHpxIlDCvFjpqn4gAkrQVdWll/uI0Kv3wirwZ3Qrpg\nBVeXjInJ+rV9r0ouBIoY8IgRLak5Hy/tSeV6nAVHv0t41B7VyoeTAsZYSWU11deR\nDBSBXHWH9zKEvXkkPdy9tgHnvLIzui2H59OPljV7z3sCLguRIvIIw8djaV9z7FRm\nKRsfmYHKOBlSO4TlpfXQg7jQ5ds65q8FFGvTB5qAgLXS8W8pvdk8jccmuzQXFUY+\nZtHgjThg7BHWWUn+7m6hQ6iHHCj34Qu69F8nLamd+KJ//14lukdyKs3AMrYsFaby\nk+UGemM/s2q3B+39B6YKaHao0SRzSJC7qDwbWPy3AgMBAAGjZDBiMB0GA1UdDgQW\nBBRWlIRrE2p2P018VTzTb6BaeOFhAzAPBgNVHRMBAf8EBTADAQH/MAsGA1UdDwQE\nAwIBtjAjBgNVHSUEHDAaBggrBgEFBQcDAQYIKwYBBQUHAwIGBFUdJQAwDQYJKoZI\nhvcNAQENBQADggEBAGWxLFlr0b9lWkOLcZtR9IDVxDL9z+UPFEk70D3NPaqXkoE/\nTNNUkXgS6+VBA2G8nigq2Yj8qoIM+kTXPb8TzWv+lrcLm+i+4AShKVknpB15cC1C\n/NJfyYGRW66s/w7HNS20RmrdN+bWS0PA4CVLXdGzUJn0PCsfsS+6Acn7RPAE+0A8\nWB7JzXWi8x9mOJwiOhodp4j41mv+5eHM0reMh6ycuYbjquDNpiNnsLztk6MGsgAP\n5C59drQWJU47738BcfbByuSTYFog6zNYCm7ACqbtiwvFTwjneNebOhsOlaEAHjup\nd4QBqYVs7pzkhNNp9oUvv4wGf/KJcw5B9E6Tpfk=\n-----END CERTIFICATE-----";

        let connection_config = ConnectionConfig {
            endpoint: "https://api.sandbox.checkout.com/payments".to_string(),
            http_method: HttpMethod::POST,
            headers,
            proxy_url: Some(masking::Secret::new(
                "https://test1:testpwd@testcred.sandbox.verygoodproxy.com:8443/".to_string(),
            )),
            backup_proxy_url: None,
            client_cert: None,
            client_key: None,
            ca_cert: Some(masking::Secret::new(ca_cert_pem.to_string())),
            insecure: None,
            cert_password: None,
            cert_format: None,
            max_response_size: None,
            vault_auth_data: None,
            vault_connector_id: Some(VaultConnectors::VGS),
            vault_connector_type: Some(VaultConnectorType::Proxy),
            vault_endpoint: None,
        };

        let request = InjectorRequest {
            connector_payload: ConnectorPayload {
                template: template.clone(),
            },
            token_data: TokenData {
                specific_token_data: specific_token_data.clone(),
            },
            connection_config,
        };

        // ══════════════════════════════════════════════════════════════
        // STEP 1: Log raw inputs (token data + template) with secrets exposed
        // ══════════════════════════════════════════════════════════════
        let vault_data = specific_token_data.clone().expose();
        println!("\n{}", "=".repeat(80));
        println!("=== STEP 1: RAW INPUTS ===");
        println!(
            "Token Data (exposed): {}",
            serde_json::to_string_pretty(&vault_data).unwrap()
        );
        println!("\nTemplate (raw):\n{}", template);
        println!(
            "\nTemplate (pretty JSON):\n{}",
            serde_json::to_string_pretty(
                &serde_json::from_str::<serde_json::Value>(&template).unwrap()
            )
            .unwrap()
        );

        // ══════════════════════════════════════════════════════════════
        // STEP 2: Run interpolation — replace {{$field}} with token aliases
        // ══════════════════════════════════════════════════════════════
        let injector = Injector::new();
        let interpolated = injector
            .interpolate_string_template_with_vault_data(template.clone(), &vault_data)
            .expect("interpolation must succeed");

        println!("\n=== STEP 2: AFTER INTERPOLATION (Proxy/VGS) ===");
        println!("Interpolated payload (raw):\n{}", interpolated);
        println!(
            "\nInterpolated payload (pretty JSON):\n{}",
            serde_json::to_string_pretty(
                &serde_json::from_str::<serde_json::Value>(&interpolated).unwrap()
            )
            .unwrap()
        );

        // ══════════════════════════════════════════════════════════════
        // STEP 3: Verify each field was replaced correctly
        // ══════════════════════════════════════════════════════════════
        let interpolated_json: serde_json::Value =
            serde_json::from_str(&interpolated).expect("interpolated payload must be valid JSON");

        let source = &interpolated_json["source"];
        let card_number = source["number"].as_str().unwrap();
        let exp_month = source["expiry_month"].as_str().unwrap();
        let exp_year = source["expiry_year"].as_str().unwrap();
        let cvv = source["cvv"].as_str().unwrap();

        println!("\n=== STEP 3: FIELD-BY-FIELD VERIFICATION ===");
        println!("source.number     = \"{}\"", card_number);
        println!("source.expiry_month = \"{}\"", exp_month);
        println!("source.expiry_year  = \"{}\"", exp_year);
        println!("source.cvv          = \"{}\"", cvv);

        // Assert token aliases were interpolated, not the raw placeholders
        assert_eq!(
            card_number, "tok_sandbox_2YDNnDoani5L3EEV5RqrMF",
            "card_number should be VGS token alias"
        );
        assert_eq!(exp_month, "12", "card_exp_month should be '12'");
        assert_eq!(exp_year, "31", "card_exp_year should be '31'");
        assert_eq!(cvv, "737", "card_cvc should be '737'");

        // Assert no leftover placeholders
        assert!(
            !interpolated.contains("{{$"),
            "No {{$...}} placeholders should remain after interpolation"
        );

        // ══════════════════════════════════════════════════════════════
        // STEP 4: Log the final connector payload that would go through VGS proxy
        // ══════════════════════════════════════════════════════════════
        println!("\n=== STEP 4: FINAL CONNECTOR PAYLOAD (what VGS proxy receives) ===");
        println!("HTTP Method:  POST");
        println!(
            "Endpoint:     https://api.sandbox.checkout.com/payments"
        );
        println!(
            "Proxy URL:    {}",
            request
                .connection_config
                .proxy_url
                .as_ref()
                .map(|p| p.clone().expose())
                .unwrap_or_else(|| "None".to_string())
        );
        println!(
            "CA Cert:      {} bytes",
            request
                .connection_config
                .ca_cert
                .as_ref()
                .map(|c| c.clone().expose().len())
                .unwrap_or(0)
        );
        println!("Headers:");
        for (k, v) in &request.connection_config.headers {
            println!("  {}: {}", k, v.clone().expose());
        }
        println!(
            "\nRequest Body (pretty):\n{}",
            serde_json::to_string_pretty(&interpolated_json).unwrap()
        );

        // ══════════════════════════════════════════════════════════════
        // STEP 5: Show what VGS should detokenize on the wire
        // ══════════════════════════════════════════════════════════════
        println!("\n=== STEP 5: VGS DETOKENIZATION EXPECTED ===");
        println!(
            "VGS should replace 'tok_sandbox_2YDNnDoani5L3EEV5RqrMF' with the real card number on the wire."
        );
        println!(
            "The connector (Checkout.com) will see the real card number, NOT the token alias."
        );
        println!("If Checkout returns 'invalid card number', the issue is in VGS proxy config, NOT in interpolation.\n");
        println!("{}\n", "=".repeat(80));
    }
}
