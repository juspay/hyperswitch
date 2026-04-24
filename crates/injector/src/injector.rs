pub mod core {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use common_utils::request::{Method, RequestBuilder, RequestContent};
    use error_stack::{self, ResultExt};
    use hyperswitch_masking::{self, ExposeInterface};
    use nom::{
        bytes::complete::{tag, take_while1},
        character::complete::{char, multispace0},
        sequence::{delimited, preceded, terminated},
        IResult,
    };
    use router_env::{instrument, logger};
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

    impl Proxy {
        /// Builds a `Proxy` from an optional proxy URL secret, applying it to both HTTP and
        /// HTTPS slots. Returns `Proxy::default()` when `proxy_url` is `None`.
        fn from_optional_url(proxy_url: Option<hyperswitch_masking::Secret<String>>) -> Self {
            match proxy_url {
                Some(url) => {
                    let url_str = url.expose();
                    Self {
                        http_url: Some(url_str.clone()),
                        https_url: Some(url_str),
                        idle_pool_connection_timeout: Some(90),
                        bypass_proxy_hosts: None,
                    }
                }
                None => Self::default(),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // TLS / HTTP client helpers
    // ---------------------------------------------------------------------------

    fn create_client(
        proxy_config: &Proxy,
        client_certificate: Option<hyperswitch_masking::Secret<String>>,
        client_certificate_key: Option<hyperswitch_masking::Secret<String>>,
        ca_certificate: Option<hyperswitch_masking::Secret<String>>,
    ) -> error_stack::Result<reqwest::Client, InjectorError> {
        logger::debug!(
            has_client_cert = client_certificate.is_some(),
            has_client_key = client_certificate_key.is_some(),
            has_ca_cert = ca_certificate.is_some(),
            "Creating HTTP client"
        );

        // Case 1: Mutual TLS
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
                .fold(client_builder, |client, cert| {
                    client.add_root_certificate(cert)
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

        // Case 2: One-way TLS with CA cert
        if let Some(ca_pem) = ca_certificate {
            let pem = ca_pem.expose().replace("\\r\\n", "\n");
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

        // Case 3: Default (no certs)
        get_client_builder(proxy_config)?
            .build()
            .change_context(InjectorError::HttpRequestFailed)
            .inspect_err(|e| logger::error!("Failed to build default HTTP client: {:?}", e))
    }

    /// Extracts vault proxy URL and CA cert from headers when the vault-metadata header is present.
    /// Returns `(proxy_url, ca_cert)` — both `None` when the header is absent or extraction fails.
    fn extract_vault_metadata(
        headers: &HashMap<String, hyperswitch_masking::Secret<String>>,
        endpoint: url::Url,
        http_method: injector_types::HttpMethod,
    ) -> (
        Option<hyperswitch_masking::Secret<String>>,
        Option<hyperswitch_masking::Secret<String>>,
    ) {
        if !headers.contains_key(crate::consts::EXTERNAL_VAULT_METADATA_HEADER) {
            return (None, None);
        }
        let mut temp_config = injector_types::ConnectionConfig::new(endpoint, http_method);
        if temp_config.extract_and_apply_vault_metadata_with_fallback(headers) {
            (temp_config.proxy_url, temp_config.ca_cert)
        } else {
            (None, None)
        }
    }

    fn get_client_builder(
        proxy_config: &Proxy,
    ) -> error_stack::Result<reqwest::ClientBuilder, InjectorError> {
        let add_proxy = |builder: reqwest::ClientBuilder,
                         proxy: error_stack::Result<reqwest::Proxy, InjectorError>|
         -> error_stack::Result<reqwest::ClientBuilder, InjectorError> {
            Ok(builder.proxy(proxy?))
        };

        let mut builder = reqwest::Client::builder();

        if let Some(url) = &proxy_config.https_url {
            let proxy = reqwest::Proxy::https(url)
                .change_context(InjectorError::HttpRequestFailed)
                .inspect_err(|e| logger::error!("Failed to configure HTTPS proxy: {:?}", e))?;
            builder = add_proxy(builder, Ok(proxy))?;
        }

        if let Some(url) = &proxy_config.http_url {
            let proxy = reqwest::Proxy::http(url)
                .change_context(InjectorError::HttpRequestFailed)
                .inspect_err(|e| logger::error!("Failed to configure HTTP proxy: {:?}", e))?;
            builder = add_proxy(builder, Ok(proxy))?;
        }

        Ok(builder)
    }

    fn create_identity_from_certificate_and_key(
        encoded_certificate: hyperswitch_masking::Secret<String>,
        encoded_certificate_key: hyperswitch_masking::Secret<String>,
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
        encoded_certificate: hyperswitch_masking::Secret<String>,
    ) -> error_stack::Result<Vec<reqwest::Certificate>, InjectorError> {
        let cert_str = encoded_certificate.expose();
        let cert = reqwest::Certificate::from_pem(cert_str.as_bytes())
            .change_context(InjectorError::HttpRequestFailed)
            .inspect_err(|e| logger::error!("Failed to create certificate from PEM: {:?}", e))?;
        Ok(vec![cert])
    }

    fn log_and_convert_http_error(e: reqwest::Error, context: &str) -> InjectorError {
        logger::error!(
            context = context,
            is_timeout = e.is_timeout(),
            is_connect = e.is_connect(),
            is_request = e.is_request(),
            is_decode = e.is_decode(),
            "HTTP request failed: {}",
            e
        );
        InjectorError::HttpRequestFailed
    }

    /// Attaches individual certificate values onto a [`RequestBuilder`] and returns the built
    /// request. Accepts cert fields directly so callers do not need to clone the whole config.
    fn build_request_with_certificates(
        mut request_builder: RequestBuilder,
        client_cert: Option<hyperswitch_masking::Secret<String>>,
        client_key: Option<hyperswitch_masking::Secret<String>>,
        ca_cert: Option<hyperswitch_masking::Secret<String>>,
    ) -> common_utils::request::Request {
        if let Some(cert) = client_cert {
            request_builder = request_builder.add_certificate(Some(cert));
        }
        if let Some(key) = client_key {
            request_builder = request_builder.add_certificate_key(Some(key));
        }
        if let Some(ca) = ca_cert {
            request_builder = request_builder.add_ca_certificate_pem(Some(ca));
        }
        request_builder.build()
    }

    #[instrument(skip_all)]
    pub async fn send_request(
        client_proxy: &Proxy,
        request: common_utils::request::Request,
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
        for (key, value) in &request.headers {
            let header_value = match value {
                hyperswitch_masking::Maskable::Masked(secret) => secret.clone().expose(),
                hyperswitch_masking::Maskable::Normal(normal) => normal.clone(),
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

    // ---------------------------------------------------------------------------
    // Error type
    // ---------------------------------------------------------------------------

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

    // ---------------------------------------------------------------------------
    // VaultConnectorStrategy trait
    //
    // Each vault connector variant implements this trait to encapsulate both:
    //   1. how to process the payload template (interpolate vs. keep placeholders)
    //   2. how to route the processed payload (direct HTTP vs. vault proxy)
    //
    // Adding a new vault connector only requires implementing this trait; the
    // orchestration in `Injector::run` never needs to change.
    // ---------------------------------------------------------------------------

    #[async_trait]
    trait VaultConnectorStrategy: Send + Sync {
        /// Transform the raw template into a payload ready for the HTTP call.
        ///
        /// - **Proxy** (e.g. VGS): interpolate `{{$field}}` with token aliases so the
        ///   forward proxy can detokenize them on the wire.
        /// - **Transformation** (e.g. HyperswitchVault): return the template unchanged;
        ///   the vault resolves placeholders downstream.
        fn process_payload(
            &self,
            injector: &Injector,
            template: String,
            vault_data: &Value,
        ) -> error_stack::Result<String, InjectorError>;

        /// Send the processed payload and return the connector response.
        async fn send(
            &self,
            injector: &Injector,
            request: &InjectorRequest,
            processed_payload: &str,
            content_type: &ContentType,
        ) -> error_stack::Result<InjectorResponse, InjectorError>;
    }

    // ---- VGS / Proxy strategy --------------------------------------------------

    struct ProxyStrategy;

    #[async_trait]
    impl VaultConnectorStrategy for ProxyStrategy {
        fn process_payload(
            &self,
            injector: &Injector,
            template: String,
            vault_data: &Value,
        ) -> error_stack::Result<String, InjectorError> {
            logger::debug!("Proxy vault: interpolating template with token aliases");
            injector.interpolate_string_template_with_vault_data(template, vault_data)
        }

        async fn send(
            &self,
            injector: &Injector,
            request: &InjectorRequest,
            processed_payload: &str,
            content_type: &ContentType,
        ) -> error_stack::Result<InjectorResponse, InjectorError> {
            injector
                .make_http_request(&request.connection_config, processed_payload, content_type)
                .await
        }
    }

    // ---- HyperswitchVault / Transformation strategy ----------------------------

    struct HyperswitchVaultStrategy;

    #[async_trait]
    impl VaultConnectorStrategy for HyperswitchVaultStrategy {
        fn process_payload(
            &self,
            _injector: &Injector,
            template: String,
            _vault_data: &Value,
        ) -> error_stack::Result<String, InjectorError> {
            logger::debug!(
                "HyperswitchVault transformation: skipping interpolation, keeping placeholders"
            );
            Ok(template)
        }

        async fn send(
            &self,
            injector: &Injector,
            request: &InjectorRequest,
            processed_payload: &str,
            _content_type: &ContentType,
        ) -> error_stack::Result<InjectorResponse, InjectorError> {
            logger::debug!("Routing request through HyperswitchVault proxy");
            injector
                .make_hyperswitch_vault_request(request, processed_payload)
                .await
        }
    }

    // ---- Fallback strategy for unknown Transformation connectors ---------------

    struct FallbackTransformationStrategy;

    #[async_trait]
    impl VaultConnectorStrategy for FallbackTransformationStrategy {
        fn process_payload(
            &self,
            _injector: &Injector,
            template: String,
            _vault_data: &Value,
        ) -> error_stack::Result<String, InjectorError> {
            logger::debug!(
                "Transformation vault (non-HyperswitchVault): no transformation implemented yet, keeping placeholders"
            );
            Ok(template)
        }

        async fn send(
            &self,
            injector: &Injector,
            request: &InjectorRequest,
            processed_payload: &str,
            content_type: &ContentType,
        ) -> error_stack::Result<InjectorResponse, InjectorError> {
            logger::debug!(
                "Transformation vault (non-HyperswitchVault): falling back to direct HTTP request"
            );
            injector
                .make_http_request(&request.connection_config, processed_payload, content_type)
                .await
        }
    }

    impl From<&InjectorRequest> for Box<dyn VaultConnectorStrategy> {
        fn from(request: &InjectorRequest) -> Self {
            match (
                &request.connection_config.vault_connector_type,
                &request.connection_config.vault_connector_id,
            ) {
                (
                    Some(injector_types::VaultConnectorType::Transformation),
                    Some(injector_types::VaultConnectors::HyperswitchVault),
                ) => Box::new(HyperswitchVaultStrategy),
                (Some(injector_types::VaultConnectorType::Proxy), _) => Box::new(ProxyStrategy),
                (Some(injector_types::VaultConnectorType::Transformation), _) => {
                    Box::new(FallbackTransformationStrategy)
                }
                _ => Box::new(ProxyStrategy), //by default, the token replacement is done in the forward proxy (e.g. VGS), so ProxyStrategy is the safe default
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Token parsing helpers
    // ---------------------------------------------------------------------------

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
            } else if let Some((_, rest)) = current_input.split_at_checked(1) {
                current_input = rest;
            } else {
                break;
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

    // ---------------------------------------------------------------------------
    // Public entry point
    // ---------------------------------------------------------------------------

    #[instrument(skip_all)]
    pub async fn injector_core(
        request: InjectorRequest,
    ) -> error_stack::Result<InjectorResponse, InjectorError> {
        let start_time = std::time::Instant::now();
        logger::info!("Starting injector_core processing");

        // Capture metric dimensions before moving `request`
        let vault_connector_str = request.connection_config.vault_connector_metric_str();
        let http_method_str = request.connection_config.http_method.as_metric_str();
        let endpoint_host = request.connection_config.endpoint_host();

        metrics::INJECTOR_INVOCATIONS_COUNT.add(
            1,
            router_env::metric_attributes!(("vault_connector", vault_connector_str)),
        );

        let injector = Injector::new();
        let result = injector.injector_core(request).await;

        let request_duration = start_time.elapsed();

        metrics::INJECTOR_REQUEST_TIME.record(
            request_duration.as_secs_f64(),
            router_env::metric_attributes!(
                ("vault_connector", vault_connector_str),
                ("http_method", http_method_str),
                ("endpoint_host", endpoint_host.clone())
            ),
        );

        result.inspect_err(|e| {
            logger::error!("Injector core failed: {:?}", e);
            metrics::INJECTOR_FAILED_TOKEN_REPLACEMENTS_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("vault_connector", vault_connector_str),
                    ("http_method", http_method_str),
                    ("endpoint_host", endpoint_host)
                ),
            );
        })
    }

    // ---------------------------------------------------------------------------
    // Injector struct
    // ---------------------------------------------------------------------------

    pub struct Injector;

    impl Injector {
        pub fn new() -> Self {
            Self
        }

        /// Top-level orchestration: select strategy -> process payload -> send -> record metrics.
        #[instrument(skip_all)]
        async fn injector_core(
            &self,
            request: InjectorRequest,
        ) -> error_stack::Result<InjectorResponse, InjectorError> {
            let start_time = std::time::Instant::now();

            let vault_data = request.token_data.specific_token_data.clone().expose();

            logger::debug!(
                template_length = request.connector_payload.template.len(),
                vault_connector_type = ?request.connection_config.vault_connector_type,
                "Processing token injection request"
            );

            let strategy = Box::<dyn VaultConnectorStrategy>::from(&request);

            // Step 1: process the template
            let processed_payload = strategy.process_payload(
                self,
                request.connector_payload.template.clone(),
                &vault_data,
            )?;

            logger::debug!(
                processed_payload_length = processed_payload.len(),
                "Token replacement completed"
            );

            // Step 2: determine content-type from headers
            let content_type = request
                .connection_config
                .headers
                .get("Content-Type")
                .map(|ct| ContentType::from_header_value(&ct.clone().expose()))
                .unwrap_or(ContentType::ApplicationXWwwFormUrlencoded);

            // Step 3: send via the strategy
            let response = strategy
                .send(self, &request, &processed_payload, &content_type)
                .await?;

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

            metrics::INJECTOR_SUCCESSFUL_TOKEN_REPLACEMENTS_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("status_code", response.status_code.to_string()),
                    (
                        "vault_connector",
                        request.connection_config.vault_connector_metric_str()
                    ),
                    (
                        "http_method",
                        request.connection_config.http_method.as_metric_str()
                    ),
                    ("endpoint_host", request.connection_config.endpoint_host())
                ),
            );

            Ok(response)
        }

        /// Replaces `{{$field_name}}` placeholders in `template` with values from `vault_data`.
        #[instrument(skip_all)]
        pub(crate) fn interpolate_string_template_with_vault_data(
            &self,
            template: String,
            vault_data: &Value,
        ) -> error_stack::Result<String, InjectorError> {
            let token_replacement_start = std::time::Instant::now();
            let tokens = find_all_tokens(&template);

            let result = tokens.into_iter().try_fold(template, |acc, token_ref| {
                let token_str = match extract_field_from_vault_data(vault_data, &token_ref.field)? {
                    Value::String(s) => s,
                    other => serde_json::to_string(&other).unwrap_or_default(),
                };
                let pattern = format!("{{{{${}}}}}", token_ref.field);
                Ok::<String, error_stack::Report<InjectorError>>(acc.replace(&pattern, &token_str))
            })?;

            metrics::INJECTOR_TOKEN_REPLACEMENT_TIME.record(
                token_replacement_start.elapsed().as_secs_f64(),
                router_env::metric_attributes!(("operation", "interpolation")),
            );

            Ok(result)
        }

        /// Builds a [`RequestContent`] from a raw payload string and a content type.
        fn build_request_content(payload: &str, content_type: ContentType) -> RequestContent {
            match content_type {
                ContentType::ApplicationJson => match serde_json::from_str::<Value>(payload) {
                    Ok(json) => RequestContent::Json(Box::new(json)),
                    Err(e) => {
                        logger::debug!(
                            "Failed to parse payload as JSON: {}, falling back to raw bytes",
                            e
                        );
                        RequestContent::RawBytes(payload.as_bytes().to_vec())
                    }
                },
                ContentType::ApplicationXWwwFormUrlencoded => {
                    let form_data: HashMap<String, String> =
                        url::form_urlencoded::parse(payload.as_bytes())
                            .into_owned()
                            .collect();
                    RequestContent::FormUrlEncoded(Box::new(form_data))
                }
                ContentType::ApplicationXml | ContentType::TextXml | ContentType::TextPlain => {
                    RequestContent::RawBytes(payload.as_bytes().to_vec())
                }
            }
        }

        /// Sends the processed payload directly to the connector endpoint.
        ///
        /// Vault-metadata extraction (proxy URL + CA cert) is scoped here — it is only
        /// relevant for the proxy path (e.g. VGS).
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

            logger::debug!("Constructed URL: {}", config.endpoint);

            let headers: Vec<(String, hyperswitch_masking::Maskable<String>)> = config
                .headers
                .clone()
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        hyperswitch_masking::Maskable::new_normal(v.expose().clone()),
                    )
                })
                .collect();

            let method = Method::from(config.http_method);

            // Extract vault metadata (proxy URL + CA cert) from headers when present.
            // This is specific to the proxy path (e.g. VGS).
            let (vault_proxy_url, vault_ca_cert) = extract_vault_metadata(
                &config.headers,
                config.endpoint.clone(),
                config.http_method,
            );

            // Vault-derived CA cert takes priority; fall back to config's own ca_cert
            let effective_ca_cert = vault_ca_cert.or_else(|| config.ca_cert.clone());

            logger::info!(
                has_client_cert = config.client_cert.is_some(),
                has_client_key = config.client_key.is_some(),
                has_ca_cert = effective_ca_cert.is_some(),
                insecure = config.insecure.unwrap_or(false),
                cert_format = ?config.cert_format,
                "Certificate configuration applied"
            );

            let request_builder = RequestBuilder::new()
                .method(method)
                .url(config.endpoint.as_str())
                .headers(headers)
                .set_body(Self::build_request_content(payload, *content_type));

            let request = build_request_with_certificates(
                request_builder,
                config.client_cert.clone(),
                config.client_key.clone(),
                effective_ca_cert,
            );

            // Proxy priority: vault metadata -> backup_proxy_url -> none
            let final_proxy_url = vault_proxy_url.or_else(|| config.backup_proxy_url.clone());
            let proxy = Proxy::from_optional_url(final_proxy_url);

            metrics::INJECTOR_OUTGOING_CALLS_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("http_method", config.http_method.as_metric_str()),
                    ("endpoint_host", config.endpoint_host())
                ),
            );

            let response = send_request(&proxy, request).await?;
            response
                .into_injector_response()
                .await
                .map_err(error_stack::Report::new)
        }

        /// Wraps the template (with `{{$field_name}}` placeholders intact) into the
        /// HyperswitchVault proxy request format and sends it to the vault endpoint.
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

            let vault_proxy_request =
                injector_types::HyperswitchVaultProxyRequest::try_from_injector_request(
                    processed_payload,
                    &request.connection_config,
                    &request.token_data,
                )
                .map_err(error_stack::Report::new)?;

            logger::info!(
                vault_endpoint = %vault_endpoint,
                destination_url = %vault_proxy_request.destination_url,
                method = %vault_proxy_request.method,
                "Sending request to HyperswitchVault proxy"
            );

            let vault_headers: Vec<(String, hyperswitch_masking::Maskable<String>)> = vec![
                (
                    "Content-Type".to_string(),
                    hyperswitch_masking::Maskable::new_normal("application/json".to_string()),
                ),
                (
                    "Accept".to_string(),
                    hyperswitch_masking::Maskable::new_normal("application/json".to_string()),
                ),
                (
                    "Authorization".to_string(),
                    hyperswitch_masking::Maskable::Masked(
                        format!("api-key={}", vault_auth.api_key.clone().expose()).into(),
                    ),
                ),
                (
                    "x-profile-id".to_string(),
                    hyperswitch_masking::Maskable::Masked(vault_auth.profile_id.clone()),
                ),
            ];

            let request_builder = RequestBuilder::new()
                .method(Method::Post)
                .url(vault_endpoint.as_str())
                .headers(vault_headers)
                .set_body(RequestContent::Json(Box::new(vault_proxy_request)));

            let http_request = request_builder.build();

            let vault_endpoint_host = vault_endpoint.host_str().unwrap_or("unknown").to_string();
            metrics::INJECTOR_OUTGOING_CALLS_COUNT.add(
                1,
                router_env::metric_attributes!(
                    ("http_method", "POST"),
                    ("endpoint_host", vault_endpoint_host)
                ),
            );

            let response = send_request(&Proxy::default(), http_request).await?;
            response
                .into_injector_response()
                .await
                .map_err(error_stack::Report::new)
        }
    }

    impl Default for Injector {
        fn default() -> Self {
            Self::new()
        }
    }

    fn extract_field_from_vault_data(
        vault_data: &Value,
        field_name: &str,
    ) -> error_stack::Result<Value, InjectorError> {
        logger::debug!("Extracting field '{}' from vault data", field_name);

        match vault_data {
            Value::Object(obj) => {
                find_field_recursively_in_vault_data(obj, field_name).ok_or_else(|| {
                    error_stack::Report::new(InjectorError::TokenReplacementFailed(format!(
                        "Field '{field_name}' not found"
                    )))
                })
            }
            _ => Err(error_stack::Report::new(
                InjectorError::TokenReplacementFailed(
                    "Vault data is not a valid JSON object".to_string(),
                ),
            )),
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
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            hyperswitch_masking::Secret::new("application/x-www-form-urlencoded".to_string()),
        );
        headers.insert(
            "Authorization".to_string(),
            hyperswitch_masking::Secret::new("Bearer Test".to_string()),
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
                endpoint: url::Url::parse("https://api.stripe.com/v1/payment_intents").unwrap(),
                http_method: HttpMethod::POST,
                headers,
                proxy_url: None,
                backup_proxy_url: None,
                client_cert: None,
                client_key: None,
                ca_cert: None,
                insecure: None,
                cert_password: None,
                cert_format: None,
                max_response_size: None,
                vault_auth_data: None,
                vault_connector_id: Some(VaultConnectors::VGS),
                vault_connector_type: Some(VaultConnectorType::Proxy),
                vault_endpoint: None,
            },
        };

        let result = injector_core(request).await;

        if let Err(ref e) = result {
            logger::info!("Error: {e:?}");
        }
        assert!(
            result.is_ok(),
            "injector_core should succeed with valid request: {result:?}"
        );

        let response = result.unwrap();

        logger::info!("=== HTTP RESPONSE FROM HTTPBIN.ORG ===");
        logger::info!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );
        logger::info!("=======================================");

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
            hyperswitch_masking::Secret::new("application/x-www-form-urlencoded".to_string()),
        );
        headers.insert(
            "Authorization".to_string(),
            hyperswitch_masking::Secret::new("Bearer TEST".to_string()),
        );

        let specific_token_data = common_utils::pii::SecretSerdeValue::new(serde_json::json!({
            "card_number": "4242429789164242",
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
                endpoint: url::Url::parse("https://httpbin.org/post").unwrap(),
                http_method: HttpMethod::POST,
                headers,
                proxy_url: None,
                backup_proxy_url: None,
                client_cert: None,
                client_key: None,
                ca_cert: None,
                insecure: None,
                cert_password: None,
                cert_format: None,
                max_response_size: None,
                vault_auth_data: None,
                vault_connector_id: Some(VaultConnectors::VGS),
                vault_connector_type: Some(VaultConnectorType::Proxy),
                vault_endpoint: None,
            },
        };

        let result = injector_core(request).await;

        assert!(
            result.is_ok(),
            "Certificate test should succeed: {result:?}"
        );

        let response = result.unwrap();

        logger::info!("=== CERTIFICATE TEST RESPONSE ===");
        logger::info!(
            "{}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );
        logger::info!("================================");

        assert!(
            response.status_code >= 200 && response.status_code < 300,
            "Certificate test should have successful status code: {}",
            response.status_code
        );

        let response_str = serde_json::to_string(&response.response).unwrap_or_default();

        let tokens_replaced = response_str.contains("4242429789164242")
            && response_str.contains("123")
            && response_str.contains("12/25");

        assert!(
            tokens_replaced,
            "Response should contain replaced tokens (card_number, cvv, expiry): {}",
            serde_json::to_string_pretty(&response.response).unwrap_or_default()
        );
    }
}
