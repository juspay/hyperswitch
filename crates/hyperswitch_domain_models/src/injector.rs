#[cfg(feature = "v2")]
pub mod types {
    use std::collections::HashMap;

    pub use common_enums::{
        InjectorAcceptType as AcceptType, InjectorContentType as ContentType,
        InjectorHttpMethod as HttpMethod, InjectorVaultConnectors as VaultConnectors,
    };
    use url::Url;

    /// Domain model for token data containing vault-specific information
    #[derive(Clone, Debug)]
    pub struct TokenData {
        /// The specific token data retrieved from the vault, containing sensitive PII
        pub specific_token_data: common_utils::pii::SecretSerdeValue,
        /// The type of vault connector being used for token retrieval
        pub vault_type: VaultConnectors,
    }

    impl From<api_models::injector::TokenData> for TokenData {
        fn from(token_data: api_models::injector::TokenData) -> Self {
            Self {
                specific_token_data: token_data.specific_token_data,
                vault_type: token_data.vault_type,
            }
        }
    }

    /// Domain model for connector payload containing the template to be processed
    #[derive(Clone, Debug)]
    pub struct ConnectorPayload {
        /// Template string containing token references in the format {{$field_name}}
        pub template: String,
    }

    impl From<api_models::injector::ConnectorPayload> for ConnectorPayload {
        fn from(payload: api_models::injector::ConnectorPayload) -> Self {
            Self {
                template: payload.template,
            }
        }
    }

    /// Domain model for HTTP connection configuration to external connectors
    #[derive(Clone, Debug)]
    pub struct ConnectionConfig {
        /// Base URL of the connector endpoint
        pub base_url: Url,
        /// Path to append to the base URL for the specific endpoint
        pub endpoint_path: String,
        /// HTTP method to use for the request
        pub http_method: HttpMethod,
        /// HTTP headers to include in the request (values are masked for security)
        pub headers: HashMap<String, masking::Secret<String>>,
        /// Optional proxy URL for routing the request through a proxy server
        pub proxy_url: Option<Url>,
        /// Optional client certificate for mutual TLS authentication (masked)
        pub client_cert: Option<masking::Secret<String>>,
        /// Optional client private key for mutual TLS authentication (masked)
        pub client_key: Option<masking::Secret<String>>,
        /// Optional CA certificate for verifying the server certificate (masked)
        pub ca_cert: Option<masking::Secret<String>>,
        /// Whether to skip certificate verification (should only be true for testing)
        pub insecure: Option<bool>,
        /// Optional password for encrypted client certificate (masked)
        pub cert_password: Option<masking::Secret<String>>,
        /// Format of the client certificate (e.g., "PEM", "DER")
        pub cert_format: Option<String>,
    }

    impl From<api_models::injector::ConnectionConfig> for ConnectionConfig {
        fn from(config: api_models::injector::ConnectionConfig) -> Self {
            Self {
                base_url: config.base_url,
                endpoint_path: config.endpoint_path,
                http_method: config.http_method,
                headers: config.headers,
                proxy_url: config.proxy_url,
                client_cert: config.client_cert,
                client_key: config.client_key,
                ca_cert: config.ca_cert,
                insecure: config.insecure,
                cert_password: config.cert_password,
                cert_format: config.cert_format,
            }
        }
    }

    /// Complete domain request structure for the injector service
    #[derive(Clone, Debug)]
    pub struct InjectorRequest {
        /// Token data retrieved from the vault for replacement
        pub token_data: TokenData,
        /// Payload template containing token references to be processed
        pub connector_payload: ConnectorPayload,
        /// HTTP connection configuration for making the external request
        pub connection_config: ConnectionConfig,
    }

    impl From<api_models::injector::InjectorRequest> for InjectorRequest {
        fn from(request: api_models::injector::InjectorRequest) -> Self {
            Self {
                token_data: request.token_data.into(),
                connector_payload: request.connector_payload.into(),
                connection_config: request.connection_config.into(),
            }
        }
    }

    pub type InjectorResponse = serde_json::Value;
}

// Re-export all types when v2 feature is enabled
#[cfg(feature = "v2")]
pub use types::*;
