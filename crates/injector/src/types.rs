pub mod models {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use common_utils::pii::SecretSerdeValue;
    pub use common_utils::proxy::{ProxyBody, ProxyMultipartPart};
    use hyperswitch_masking::{ExposeInterface, Secret};
    use router_env::logger;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    // Enums for the injector - making it standalone

    /// Content types supported by the injector for HTTP requests
    #[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ContentType {
        ApplicationJson,
        ApplicationXWwwFormUrlencoded,
        ApplicationXml,
        TextXml,
        TextPlain,
    }

    impl ContentType {
        /// Parse a `Content-Type` header value string into a [`ContentType`].
        /// Returns the default (`ApplicationXWwwFormUrlencoded`) for unknown values.
        pub fn from_header_value(value: &str) -> Self {
            let kind = common_utils::proxy::media_type(value);
            match kind.as_str() {
                "application/json" => Self::ApplicationJson,
                "application/x-www-form-urlencoded" => Self::ApplicationXWwwFormUrlencoded,
                "application/xml" => Self::ApplicationXml,
                "text/xml" => Self::TextXml,
                "text/plain" => Self::TextPlain,
                _ if kind.ends_with("+json") => Self::ApplicationJson,
                _ if kind.ends_with("+xml") => Self::ApplicationXml,
                _ => Self::ApplicationXWwwFormUrlencoded,
            }
        }
    }

    /// HTTP methods supported by the injector
    #[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "UPPERCASE")]
    pub enum HttpMethod {
        GET,
        POST,
        PUT,
        PATCH,
        DELETE,
    }

    impl HttpMethod {
        /// Returns a stable string representation suitable for metric dimensions.
        pub fn as_metric_str(self) -> &'static str {
            match self {
                Self::GET => "GET",
                Self::POST => "POST",
                Self::PUT => "PUT",
                Self::PATCH => "PATCH",
                Self::DELETE => "DELETE",
            }
        }
    }

    /// Vault connectors supported by the injector for token management
    ///
    /// Currently supports VGS as the primary vault connector. While only VGS is
    /// implemented today, this enum structure is maintained for future extensibility
    /// to support additional vault providers (e.g., Basis Theory, Skyflow, etc.)
    /// without breaking API compatibility.
    #[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "UPPERCASE")]
    pub enum VaultConnectors {
        /// VGS (Very Good Security) vault connector
        VGS,
        HyperswitchVault,
    }

    impl VaultConnectors {
        /// Returns a stable string representation suitable for metric dimensions.
        pub fn as_metric_str(self) -> &'static str {
            match self {
                Self::VGS => "VGS",
                Self::HyperswitchVault => "HyperswitchVault",
            }
        }
    }

    /// Token data containing vault-specific information for token replacement
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TokenData {
        /// The specific token data retrieved from the vault.
        /// Contains token aliases mapped to field names (never real card data).
        pub specific_token_data: SecretSerdeValue,
    }

    /// Connector payload containing the template to be processed
    #[derive(Clone, Deserialize, Serialize)]
    pub struct ConnectorPayload {
        /// Template string containing token references in the format {{$field_name}}
        pub template: String,
        /// Explicit lossless body for HyperswitchVault. Legacy callers retain `template`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub body: Option<ProxyBody>,
    }

    impl std::fmt::Debug for ConnectorPayload {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ConnectorPayload")
                .field("template_length", &self.template.len())
                .field("body", &self.body)
                .finish()
        }
    }

    /// Configuration for HTTP connection to the external connector
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ConnectionConfig {
        /// Complete URL endpoint for the connector (e.g., "https://api.stripe.com/v1/payment_intents")
        pub endpoint: url::Url,
        /// HTTP method to use for the request
        pub http_method: HttpMethod,
        /// HTTP headers to include in the request
        pub headers: HashMap<String, Secret<String>>,

        /// Optional vault endpoint to use for token retrieval (overrides default vault endpoint if provided)
        pub vault_endpoint: Option<url::Url>,

        /// Optional vault connector type to use for token retrieval (overrides default vault connector if provided)
        pub vault_connector_id: Option<VaultConnectors>,

        /// Optional vault authentication data for authenticating with the vault connector (e.g., API keys, client credentials, etc.)
        pub vault_auth_data: Option<VaultConnectorAuth>,

        /// Optional vault connector type to use for token retrieval (overrides default vault connector if provided)
        pub vault_connector_type: Option<VaultConnectorType>,
        /// Optional proxy URL for routing the request through a proxy server
        pub proxy_url: Option<Secret<String>>,
        /// Optional backup proxy URL to use if vault metadata doesn't provide one
        #[serde(default)]
        pub backup_proxy_url: Option<Secret<String>>,
        /// Optional client certificate for mutual TLS authentication
        pub client_cert: Option<Secret<String>>,
        /// Optional client private key for mutual TLS authentication
        pub client_key: Option<Secret<String>>,
        /// Optional CA certificate for verifying the server certificate
        pub ca_cert: Option<Secret<String>>,
        /// Whether to skip certificate verification (for testing only)
        pub insecure: Option<bool>,
        /// Optional password for encrypted client certificate
        pub cert_password: Option<Secret<String>>,
        /// Format of the client certificate (e.g., "PEM")
        pub cert_format: Option<String>,
        /// Maximum response size in bytes (defaults to 10MB if not specified)
        pub max_response_size: Option<usize>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum VaultConnectorType {
        /// Proxy vault - forwards requests through a proxy (e.g., VGS forward proxy)
        Proxy,
        /// Transformation vault - transforms/tokenizes data (e.g., HyperswitchVault)
        Transformation,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct VaultConnectorAuth {
        /// API key for authenticating with the vault connector
        pub api_key: Secret<String>,
        /// API secret for authenticating with the vault connector
        pub profile_id: Secret<String>,
    }

    /// Request body for HyperswitchVault proxy endpoint.
    ///
    /// The HS Vault proxy receives the original template (with {{$field_name}} placeholders),
    /// the destination connector URL, headers, and a token reference. The vault resolves
    /// placeholders with actual card data and forwards the request to the destination.
    #[derive(Clone, Serialize, Deserialize)]
    pub struct HyperswitchVaultProxyRequest {
        /// The connector request body template with {{$field_name}} placeholders.
        /// HS Vault will resolve these with actual card data before forwarding.
        pub request_body: SecretSerdeValue,
        /// Typed connector body, including multipart/binary/absent bodies.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub body: Option<Secret<ProxyBody>>,
        /// TLS credentials apply to the downstream connector, not the SaaS endpoint.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub certificate: Option<Secret<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub certificate_key: Option<Secret<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ca_certificate: Option<Secret<String>>,
        /// The connector's actual endpoint URL (e.g. "https://api.sandbox.checkout.com/payments")
        pub destination_url: url::Url,
        /// The connector's headers (Content-Type, Authorization, etc.)
        pub headers: HashMap<String, Secret<String>>,
        /// The single token reference extracted from specific_token_data.card_number.
        /// For HyperswitchVault, all fields share the same token value.
        pub token: Secret<String>,

        pub token_type: String,
        /// HTTP method to use for the forwarded request
        pub method: String,
    }

    impl std::fmt::Debug for HyperswitchVaultProxyRequest {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("HyperswitchVaultProxyRequest")
                .field("body", &self.body)
                .field("header_count", &self.headers.len())
                .field("method", &self.method)
                .finish_non_exhaustive()
        }
    }

    impl HyperswitchVaultProxyRequest {
        /// Constructs a HyperswitchVaultProxyRequest from the injector request components.
        ///
        /// Extracts the single shared token from `specific_token_data.card_number`
        /// (HyperswitchVault uses a single token for all fields).
        pub fn try_from_injector_request(
            processed_payload: &str,
            connection_config: &ConnectionConfig,
            token_data: &TokenData,
        ) -> Result<Self, crate::injector::core::InjectorError> {
            // Extract the single token from card_number field in specific_token_data
            let vault_data = token_data.specific_token_data.clone().expose();
            let token = vault_data
                .as_object()
                .and_then(|obj| obj.get("card_number"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    crate::injector::core::InjectorError::InvalidTemplate(
                        "card_number field is required in specific_token_data for HyperswitchVault"
                            .to_string(),
                    )
                })?;

            // Parse the processed payload as JSON value.
            // The template may be JSON or form-urlencoded; wrap non-JSON as a string value.
            let request_body = serde_json::from_str::<Value>(processed_payload)
                .unwrap_or_else(|_| Value::String(processed_payload.to_string()));

            // Convert connector headers from Secret<String> → plain String for the vault request
            let headers: HashMap<String, Secret<String>> = connection_config
                .headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            let method = connection_config.http_method.as_metric_str().to_string();

            Ok(Self {
                request_body: request_body.into(),
                body: None,
                certificate: connection_config.client_cert.clone(),
                certificate_key: connection_config.client_key.clone(),
                ca_certificate: connection_config.ca_cert.clone(),
                destination_url: connection_config.endpoint.clone(),
                headers,
                token: token.into(),
                token_type: "payment_method_token".to_string(),
                method,
            })
        }
    }

    /// Complete request structure for the injector service
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct InjectorRequest {
        /// Token data from the vault
        pub token_data: TokenData,
        /// Payload template to process
        pub connector_payload: ConnectorPayload,
        /// HTTP connection configuration
        pub connection_config: ConnectionConfig,
    }

    /// Response from the injector including status code and response data
    #[derive(Clone, Deserialize, Serialize)]
    pub struct InjectorResponse {
        /// HTTP status code from the connector response
        pub status_code: u16,
        /// Response headers from the connector (optional)
        pub headers: Option<HashMap<String, String>>,
        /// Response body from the connector
        pub response: Value,
        /// Original HTTP response bytes, before JSON parsing or UTF-8 conversion.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub response_body_base64: Option<String>,
        /// Connector headers preserving repeated names before conversion to the legacy map.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub response_headers_multi: Option<Vec<(String, String)>>,
    }

    impl std::fmt::Debug for InjectorResponse {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("InjectorResponse")
                .field("status_code", &self.status_code)
                .field("header_count", &self.headers.as_ref().map(HashMap::len))
                .finish_non_exhaustive()
        }
    }

    /// Actual connector response reconstructed from a direct or SaaS proxy reply.
    pub struct InjectorConnectorResponse {
        pub status_code: u16,
        pub headers: Option<Vec<(String, String)>>,
        pub body: Vec<u8>,
    }

    impl InjectorResponse {
        /// Status of the connector reply, distinct from the outer SaaS HTTP status.
        pub fn connector_status_code(&self, wrapped: bool) -> Result<u16, crate::InjectorError> {
            if !wrapped {
                return Ok(self.status_code);
            }
            if !(200..=299).contains(&self.status_code) {
                return Err(crate::InjectorError::HttpRequestFailed);
            }
            self.response
                .get("status_code")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| (100..=599).contains(value))
                .ok_or_else(|| {
                    crate::InjectorError::InvalidTemplate(
                        "Invalid vault proxy response status".into(),
                    )
                })
        }

        /// Outer status and API error code/message of a failed SaaS proxy call, so
        /// the injector error explains why. Proxy error bodies hold no card data.
        pub fn vault_proxy_failure_summary(&self) -> Option<String> {
            if (200..=299).contains(&self.status_code) {
                return None;
            }
            let error = self.response.get("error").unwrap_or(&self.response);
            let field = |name: &str| {
                error
                    .get(name)
                    .and_then(Value::as_str)
                    .map(|value| value.chars().take(256).collect::<String>())
                    .unwrap_or_else(|| "unknown".to_owned())
            };
            Some(format!(
                "Vault proxy returned HTTP {}, error code: {}, message: {}",
                self.status_code,
                field("code"),
                field("message")
            ))
        }

        /// Unwrap only the HyperswitchVault envelope. Direct connector replies may
        /// legitimately contain fields called `response` and `status_code`.
        pub fn into_connector_response(
            self,
            wrapped: bool,
        ) -> Result<InjectorConnectorResponse, crate::InjectorError> {
            let status = self.connector_status_code(wrapped)?;
            let (status_code, headers, encoded, fallback) = if wrapped {
                let legacy_headers = self
                    .response
                    .get("response_headers")
                    .map(|value| serde_json::from_value::<HashMap<String, String>>(value.clone()))
                    .transpose()
                    .map_err(|_| {
                        crate::InjectorError::InvalidTemplate(
                            "Invalid vault proxy response headers".into(),
                        )
                    })?;
                let headers = self
                    .response
                    .get("response_headers_multi")
                    .filter(|value| !value.is_null())
                    .map(|value| serde_json::from_value::<Vec<(String, String)>>(value.clone()))
                    .transpose()
                    .map_err(|_| {
                        crate::InjectorError::InvalidTemplate(
                            "Invalid vault proxy response headers".into(),
                        )
                    })?
                    .or_else(|| legacy_headers.map(|headers| headers.into_iter().collect()));
                let encoded = self
                    .response
                    .get("response_body_base64")
                    .filter(|v| !v.is_null())
                    .map(|v| {
                        v.as_str().map(str::to_owned).ok_or_else(|| {
                            crate::InjectorError::InvalidTemplate(
                                "Invalid vault proxy response bytes".into(),
                            )
                        })
                    })
                    .transpose()?;
                let fallback = self.response.get("response").cloned().ok_or_else(|| {
                    crate::InjectorError::InvalidTemplate(
                        "Missing vault proxy response body".into(),
                    )
                })?;
                (status, headers, encoded, fallback)
            } else {
                (
                    self.status_code,
                    self.response_headers_multi
                        .or_else(|| self.headers.map(|headers| headers.into_iter().collect())),
                    self.response_body_base64,
                    self.response,
                )
            };
            let fallback_is_json = headers
                .as_ref()
                .and_then(|headers| {
                    headers
                        .iter()
                        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
                })
                .map(|(_, value)| common_utils::proxy::media_type(value))
                .map(|kind| kind == "application/json" || kind.ends_with("+json"))
                .unwrap_or(wrapped);
            let body = match encoded {
                Some(encoded) => STANDARD.decode(encoded).map_err(|_| {
                    crate::InjectorError::InvalidTemplate(
                        "Invalid base64 connector response".into(),
                    )
                })?,
                None => match fallback {
                    Value::String(text) if !fallback_is_json => text.into_bytes(),
                    value => serde_json::to_vec(&value).map_err(|_| {
                        crate::InjectorError::InvalidTemplate("Invalid connector response".into())
                    })?,
                },
            };
            Ok(InjectorConnectorResponse {
                status_code,
                headers,
                body,
            })
        }
    }

    /// Trait for converting HTTP responses to InjectorResponse
    #[async_trait]
    pub trait IntoInjectorResponse {
        /// Convert to InjectorResponse with proper error handling
        async fn into_injector_response(
            self,
        ) -> Result<InjectorResponse, crate::injector::core::InjectorError>;
    }

    #[async_trait]
    impl IntoInjectorResponse for reqwest::Response {
        async fn into_injector_response(
            self,
        ) -> Result<InjectorResponse, crate::injector::core::InjectorError> {
            let status_code = self.status().as_u16();

            logger::info!(
                status_code = status_code,
                "Converting reqwest::Response to InjectorResponse"
            );

            // Extract headers
            // A non-ASCII header value must not discard a reply the connector has
            // already processed; JSON strings cannot carry its raw bytes.
            let response_headers_multi = self
                .headers()
                .iter()
                .map(|(name, value)| {
                    let value = value.to_str().map(str::to_owned).unwrap_or_else(|_| {
                        logger::warn!(
                            header_name = %name,
                            "Response header value is not visible ASCII; converting lossily"
                        );
                        String::from_utf8_lossy(value.as_bytes()).into_owned()
                    });
                    (name.to_string(), value)
                })
                .collect::<Vec<_>>();
            let headers: Option<HashMap<String, String>> = {
                let header_map: HashMap<String, String> = self
                    .headers()
                    .iter()
                    .filter_map(|(name, value)| {
                        value
                            .to_str()
                            .ok()
                            .map(|v| (name.to_string(), v.to_string()))
                    })
                    .collect();

                if header_map.is_empty() {
                    None
                } else {
                    Some(header_map)
                }
            };

            let response_bytes = self
                .bytes()
                .await
                .map_err(|_| crate::injector::core::InjectorError::HttpRequestFailed)?;

            logger::debug!(
                response_length = response_bytes.len(),
                headers_count = headers.as_ref().map(|h| h.len()).unwrap_or(0),
                "Processing connector response"
            );

            let response_data = match serde_json::from_slice::<Value>(&response_bytes) {
                Ok(json) => json,
                Err(_e) => std::str::from_utf8(&response_bytes)
                    .map(|text| Value::String(text.to_owned()))
                    .unwrap_or(Value::Null),
            };

            Ok(InjectorResponse {
                status_code,
                headers,
                response: response_data,
                response_body_base64: Some(STANDARD.encode(&response_bytes)),
                response_headers_multi: Some(response_headers_multi),
            })
        }
    }

    impl InjectorRequest {
        /// Creates a new InjectorRequest
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            endpoint: url::Url,
            http_method: HttpMethod,
            template: String,
            token_data: TokenData,
            headers: Option<HashMap<String, Secret<String>>>,
            proxy_url: Option<Secret<String>>,
            client_cert: Option<Secret<String>>,
            client_key: Option<Secret<String>>,
            ca_cert: Option<Secret<String>>,
        ) -> Self {
            let headers = headers.unwrap_or_default();
            let mut connection_config = ConnectionConfig::new(endpoint, http_method);

            // Keep vault metadata header for processing in make_http_request

            // Store backup proxy for make_http_request to use as fallback
            connection_config.backup_proxy_url = proxy_url;
            connection_config.client_cert = connection_config.client_cert.or(client_cert);
            connection_config.client_key = connection_config.client_key.or(client_key);
            connection_config.ca_cert = connection_config.ca_cert.or(ca_cert);
            connection_config.headers = headers;

            Self {
                token_data,
                connector_payload: ConnectorPayload {
                    template,
                    body: None,
                },
                connection_config,
            }
        }
    }

    impl ConnectionConfig {
        /// Creates a new ConnectionConfig from basic parameters
        pub fn new(endpoint: url::Url, http_method: HttpMethod) -> Self {
            Self {
                endpoint,
                http_method,
                headers: HashMap::new(),
                vault_endpoint: None,
                vault_connector_id: None,
                vault_auth_data: None,
                vault_connector_type: None,
                proxy_url: None,
                backup_proxy_url: None,
                client_cert: None,
                client_key: None,
                ca_cert: None,
                insecure: None,
                cert_password: None,
                cert_format: None,
                max_response_size: None,
            }
        }

        /// Extracts the host portion of the endpoint URL for use as a metric dimension.
        /// Returns `"unknown"` when the host cannot be determined, or `"invalid_url"` when
        /// the endpoint cannot be parsed as a URL at all.
        pub fn endpoint_host(&self) -> String {
            self.endpoint.host_str().unwrap_or("unknown").to_string()
        }

        /// HyperswitchVault transformation sends the template to the SaaS vault proxy,
        /// whose reply wraps the connector response in a JSON envelope.
        pub fn uses_hyperswitch_vault_proxy(&self) -> bool {
            matches!(
                (&self.vault_connector_type, &self.vault_connector_id),
                (
                    Some(VaultConnectorType::Transformation),
                    Some(VaultConnectors::HyperswitchVault)
                )
            )
        }

        /// Returns the metric-string for the vault connector, falling back to `"None"`.
        pub fn vault_connector_metric_str(&self) -> &'static str {
            self.vault_connector_id
                .map(VaultConnectors::as_metric_str)
                .unwrap_or("None")
        }
    }
}

pub use models::*;
