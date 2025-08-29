pub mod models {
    use std::collections::HashMap;

    use common_utils::pii::SecretSerdeValue;
    use masking::Secret;
    use serde::{Deserialize, Serialize};
    use url::Url;

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

    /// Accept types supported by the injector for HTTP requests
    #[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum AcceptType {
        ApplicationJson,
        ApplicationXml,
        TextXml,
        TextPlain,
        Any,
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
        /// Base URL of the connector endpoint
        pub base_url: Url,
        /// Path to append to the base URL for the specific endpoint
        pub endpoint_path: String,
        /// HTTP method to use for the request
        pub http_method: HttpMethod,
        /// HTTP headers to include in the request
        pub headers: HashMap<String, Secret<String>>,
        /// Optional proxy URL for routing the request through a proxy server
        pub proxy_url: Option<Url>,
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

    pub type InjectorResponse = serde_json::Value;

    // Domain models for internal use

    /// Domain model for token data containing vault-specific information
    #[derive(Clone, Debug)]
    pub struct DomainTokenData {
        /// The specific token data retrieved from the vault, containing sensitive PII
        pub specific_token_data: SecretSerdeValue,
        /// The type of vault connector being used for token retrieval
        pub vault_connector: VaultConnectors,
    }

    impl From<TokenData> for DomainTokenData {
        fn from(token_data: TokenData) -> Self {
            Self {
                specific_token_data: token_data.specific_token_data,
                vault_connector: token_data.vault_connector,
            }
        }
    }

    /// Domain model for connector payload containing the template to be processed
    #[derive(Clone, Debug)]
    pub struct DomainConnectorPayload {
        /// Template string containing token references in the format {{$field_name}}
        pub template: String,
    }

    impl From<ConnectorPayload> for DomainConnectorPayload {
        fn from(payload: ConnectorPayload) -> Self {
            Self {
                template: payload.template,
            }
        }
    }

    /// Domain model for HTTP connection configuration to external connectors
    #[derive(Clone, Debug)]
    pub struct DomainConnectionConfig {
        /// Base URL of the connector endpoint
        pub base_url: Url,
        /// Path to append to the base URL for the specific endpoint
        pub endpoint_path: String,
        /// HTTP method to use for the request
        pub http_method: HttpMethod,
        /// HTTP headers to include in the request (values are masked for security)
        pub headers: HashMap<String, Secret<String>>,
        /// Optional proxy URL for routing the request through a proxy server
        pub proxy_url: Option<Url>,
        /// Optional client certificate for mutual TLS authentication (masked)
        pub client_cert: Option<Secret<String>>,
        /// Optional client private key for mutual TLS authentication (masked)
        pub client_key: Option<Secret<String>>,
        /// Optional CA certificate for verifying the server certificate (masked)
        pub ca_cert: Option<Secret<String>>,
        /// Whether to skip certificate verification (should only be true for testing)
        pub insecure: Option<bool>,
        /// Optional password for encrypted client certificate (masked)
        pub cert_password: Option<Secret<String>>,
        /// Format of the client certificate (e.g., "PEM", "DER")
        pub cert_format: Option<String>,
        /// Maximum response size in bytes (defaults to 10MB if not specified)
        pub max_response_size: Option<usize>,
    }

    impl From<ConnectionConfig> for DomainConnectionConfig {
        fn from(config: ConnectionConfig) -> Self {
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
                max_response_size: config.max_response_size,
            }
        }
    }

    /// Complete domain request structure for the injector service
    #[derive(Clone, Debug)]
    pub struct DomainInjectorRequest {
        /// Token data retrieved from the vault for replacement
        pub token_data: DomainTokenData,
        /// Payload template containing token references to be processed
        pub connector_payload: DomainConnectorPayload,
        /// HTTP connection configuration for making the external request
        pub connection_config: DomainConnectionConfig,
    }

    impl From<InjectorRequest> for DomainInjectorRequest {
        fn from(request: InjectorRequest) -> Self {
            Self {
                token_data: request.token_data.into(),
                connector_payload: request.connector_payload.into(),
                connection_config: request.connection_config.into(),
            }
        }
    }
}

pub use models::*;
