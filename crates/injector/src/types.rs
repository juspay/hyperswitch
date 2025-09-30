pub mod models {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use common_utils::pii::SecretSerdeValue;
    use masking::Secret;
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
    }

    /// Token data containing vault-specific information for token replacement
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TokenData {
        /// The specific token data retrieved from the vault
        pub specific_token_data: SecretSerdeValue,
        /// The type of vault connector being used (e.g., VGS)
        pub vault_connector: VaultConnectors,
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
