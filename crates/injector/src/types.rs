pub mod models {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use common_utils::pii::SecretSerdeValue;
    use hyperswitch_masking::{ExposeInterface, Secret};
    use router_env::logger;
    use serde::{Deserialize, Serialize};

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

    /// Token data containing vault-specific information for token replacement
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TokenData {
        /// The specific token data retrieved from the vault.
        /// Contains token aliases mapped to field names (never real card data).
        pub specific_token_data: SecretSerdeValue,
    }

    /// Connector payload containing the template to be processed
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ConnectorPayload {
        /// Template string containing token references in the format {{$field_name}}
        pub template: String,
    }

    /// Configuration for HTTP connection to the external connector
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ConnectionConfig {
        /// Complete URL endpoint for the connector (e.g., "https://api.stripe.com/v1/payment_intents")
        pub endpoint: String,
        /// HTTP method to use for the request
        pub http_method: HttpMethod,
        /// HTTP headers to include in the request
        pub headers: HashMap<String, Secret<String>>,

        /// Optional vault endpoint to use for token retrieval (overrides default vault endpoint if provided)
        pub vault_endpoint: Option<String>,

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
        pub api_secret: Secret<String>,
    }

    /// Request body for HyperswitchVault proxy endpoint.
    ///
    /// The HS Vault proxy receives the original template (with {{$field_name}} placeholders),
    /// the destination connector URL, headers, and a token reference. The vault resolves
    /// placeholders with actual card data and forwards the request to the destination.
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct HyperswitchVaultProxyRequest {
        /// The connector request body template with {{$field_name}} placeholders.
        /// HS Vault will resolve these with actual card data before forwarding.
        pub request_body: serde_json::Value,
        /// The connector's actual endpoint URL (e.g. "https://api.sandbox.checkout.com/payments")
        pub destination_url: String,
        /// The connector's headers (Content-Type, Authorization, etc.)
        pub headers: HashMap<String, String>,
        /// The single token reference extracted from specific_token_data.card_number.
        /// For HyperswitchVault, all fields share the same token value.
        pub token: String,

        pub token_type: String,
        /// HTTP method to use for the forwarded request
        pub method: String,
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
            let request_body = serde_json::from_str::<serde_json::Value>(processed_payload)
                .unwrap_or_else(|_| serde_json::Value::String(processed_payload.to_string()));

            // Convert connector headers from Secret<String> → plain String for the vault request
            let headers: HashMap<String, String> = connection_config
                .headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone().expose().clone()))
                .collect();

            let method = format!("{:?}", connection_config.http_method);

            Ok(Self {
                request_body,
                destination_url: connection_config.endpoint.clone(),
                headers,
                token,
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
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct InjectorResponse {
        /// HTTP status code from the connector response
        pub status_code: u16,
        /// Response headers from the connector (optional)
        pub headers: Option<HashMap<String, String>>,
        /// Response body from the connector
        pub response: serde_json::Value,
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

            let response_text = self
                .text()
                .await
                .map_err(|_| crate::injector::core::InjectorError::HttpRequestFailed)?;

            logger::debug!(
                response_length = response_text.len(),
                headers_count = headers.as_ref().map(|h| h.len()).unwrap_or(0),
                "Processing connector response"
            );

            let response_data = match serde_json::from_str::<serde_json::Value>(&response_text) {
                Ok(json) => json,
                Err(_e) => serde_json::Value::String(response_text),
            };

            Ok(InjectorResponse {
                status_code,
                headers,
                response: response_data,
            })
        }
    }

    impl InjectorRequest {
        /// Creates a new InjectorRequest
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            endpoint: String,
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
                connector_payload: ConnectorPayload { template },
                connection_config,
            }
        }
    }

    impl ConnectionConfig {
        /// Creates a new ConnectionConfig from basic parameters
        pub fn new(endpoint: String, http_method: HttpMethod) -> Self {
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
    }
}

pub use models::*;
