pub mod models {
    use std::collections::HashMap;

    use common_utils::pii::SecretSerdeValue;
    use masking::Secret;
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
        pub base_url: String,
        /// Path to append to the base URL for the specific endpoint
        pub endpoint_path: String,
        /// HTTP method to use for the request
        pub http_method: HttpMethod,
        /// HTTP headers to include in the request
        pub headers: HashMap<String, Secret<String>>,
        /// Optional proxy URL for routing the request through a proxy server
        pub proxy_url: Option<Secret<String>>,
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
        pub base_url: String,
        /// Path to append to the base URL for the specific endpoint
        pub endpoint_path: String,
        /// HTTP method to use for the request
        pub http_method: HttpMethod,
        /// HTTP headers to include in the request (values are masked for security)
        pub headers: HashMap<String, Secret<String>>,
        /// Optional proxy URL for routing the request through a proxy server
        pub proxy_url: Option<Secret<String>>,
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

    impl InjectorRequest {
        /// Creates a new InjectorRequest with intelligent processing
        /// 
        /// This single function handles everything:
        /// - Automatically processes vault metadata from headers (graceful fallback)
        /// - Applies vault configuration (proxy URL, certificates) when available
        /// - Uses fallback configurations when vault metadata is not present or invalid
        /// - Removes vault metadata headers from regular headers
        /// - Supports all connection types (simple, vault-enabled, certificate-based)
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            base_url: String,
            endpoint_path: String,
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
            
            // Create base configuration
            let mut connection_config = ConnectionConfig::new(base_url.clone(), endpoint_path.clone(), http_method);
            
            // Try to apply vault metadata with graceful fallback
            tracing::debug!(
                header_count = headers.len(),
                has_vault_metadata = headers.contains_key(vault_metadata::EXTERNAL_VAULT_METADATA_HEADER),
                "Processing injector request with headers"
            );
            let vault_applied = {
                use vault_metadata::VaultMetadataExtractorExt;
                connection_config.extract_and_apply_vault_metadata_with_fallback(&headers)
            };
            tracing::debug!(
                vault_applied = vault_applied,
                proxy_url_set = connection_config.proxy_url.is_some(),
                ca_cert_set = connection_config.ca_cert.is_some(),
                "Vault metadata processing result"
            );
            
            // Apply fallback configurations only if vault didn't provide them
            if !vault_applied || connection_config.proxy_url.is_none() {
                connection_config.proxy_url = proxy_url;
            }
            
            if !vault_applied || connection_config.client_cert.is_none() {
                connection_config.client_cert = client_cert;
            }
            
            if !vault_applied || connection_config.client_key.is_none() {
                connection_config.client_key = client_key;
            }
            
            if !vault_applied || connection_config.ca_cert.is_none() {
                connection_config.ca_cert = ca_cert;
            }
            
            // Set headers (excluding vault metadata header)
            let mut filtered_headers = headers;
            filtered_headers.remove(vault_metadata::EXTERNAL_VAULT_METADATA_HEADER);
            connection_config.headers = filtered_headers;

            tracing::debug!(
                base_url = %base_url,
                endpoint_path = %endpoint_path,
                vault_configured = vault_applied,
                has_proxy = connection_config.proxy_url.is_some(),
                has_client_cert = connection_config.client_cert.is_some(),
                has_ca_cert = connection_config.ca_cert.is_some(),
                "Created injector request with unified processing"
            );

            Self {
                token_data,
                connector_payload: ConnectorPayload { template },
                connection_config,
            }
        }
    }

    impl ConnectionConfig {
        /// Creates a new ConnectionConfig from basic parameters
        pub fn new(
            base_url: String,
            endpoint_path: String,
            http_method: HttpMethod,
        ) -> Self {
            use std::collections::HashMap;
            
            Self {
                base_url,
                endpoint_path,
                http_method,
                headers: HashMap::new(),
                proxy_url: None,
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

    /// External vault metadata processing module
    pub mod vault_metadata {
        use super::*;
        use masking::{ExposeInterface, Secret};
        use url::Url;
        use base64::Engine;

        const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;
        pub const EXTERNAL_VAULT_METADATA_HEADER: &str = "x-external-vault-metadata";

        /// Trait for different vault metadata processors
        pub trait VaultMetadataProcessor: Send + Sync {
            /// Process vault metadata and return connection configuration updates
            fn process_metadata(&self, connection_config: &mut ConnectionConfig) -> Result<(), VaultMetadataError>;
            
            /// Get the vault connector type
            fn vault_connector(&self) -> VaultConnectors;
        }

        /// Comprehensive errors related to vault metadata processing
        #[derive(Debug, thiserror::Error)]
        pub enum VaultMetadataError {
            #[error("Failed to decode base64 vault metadata: {0}")]
            Base64DecodingFailed(String),
            #[error("Failed to parse vault metadata JSON: {0}")]
            JsonParsingFailed(String),
            #[error("Unsupported vault connector: {0}")]
            UnsupportedVaultConnector(String),
            #[error("Invalid URL in vault metadata: {0}")]
            InvalidUrl(String),
            #[error("Missing required field in vault metadata: {0}")]
            MissingRequiredField(String),
            #[error("Invalid certificate format: {0}")]
            InvalidCertificateFormat(String),
            #[error("Vault metadata header is empty or malformed")]
            EmptyOrMalformedHeader,
            #[error("URL validation failed for {field}: {url} - {reason}")]
            UrlValidationFailed {
                field: String,
                url: String,
                reason: String,
            },
            #[error("Certificate validation failed: {0}")]
            CertificateValidationFailed(String),
            #[error("Vault metadata processing failed for connector {connector}: {reason}")]
            ProcessingFailed {
                connector: String,
                reason: String,
            },
        }

        impl VaultMetadataError {
            /// Create a processing failed error with context
            pub fn processing_failed(connector: &str, reason: impl Into<String>) -> Self {
                Self::ProcessingFailed {
                    connector: connector.to_string(),
                    reason: reason.into(),
                }
            }

            /// Create a URL validation error with context
            pub fn url_validation_failed(field: &str, url: &str, reason: impl Into<String>) -> Self {
                Self::UrlValidationFailed {
                    field: field.to_string(),
                    url: url.to_string(),
                    reason: reason.into(),
                }
            }
        }

        /// External vault proxy metadata (moved from external_services)
        #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
        #[serde(untagged)]
        pub enum ExternalVaultProxyMetadata {
            /// VGS proxy data variant
            VgsMetadata(VgsMetadata),
        }

        /// VGS proxy data (moved from external_services)
        #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
        pub struct VgsMetadata {
            /// External vault url
            pub proxy_url: Url,
            /// CA certificates to verify the vault server
            pub certificate: Secret<String>,
        }

        impl VaultMetadataProcessor for VgsMetadata {
            fn process_metadata(&self, connection_config: &mut ConnectionConfig) -> Result<(), VaultMetadataError> {
                println!("VGS DEBUG: Starting VGS metadata processing - proxy_url={}, scheme={}, host={:?}, port={:?}", 
                    self.proxy_url,
                    self.proxy_url.scheme(),
                    self.proxy_url.host(),
                    self.proxy_url.port()
                );
                
                // Validate and set proxy URL from VGS metadata
                self.validate_proxy_url()?;
                let proxy_url_str = self.proxy_url.as_str().to_string();
                connection_config.proxy_url = Some(Secret::new(proxy_url_str.clone()));
                
                println!("VGS PROXY: Set proxy URL from VGS metadata - original_proxy_url={}, processed_proxy_url={}, proxy_url_length={}", 
                    self.proxy_url,
                    proxy_url_str,
                    proxy_url_str.len()
                );
                
                // Validate and decode certificate from VGS metadata
                self.validate_certificate()?;
                let cert_content = self.certificate.clone().expose();
                
                println!("VGS CERT: Processing certificate from VGS metadata - cert_length={}, cert_starts_with_pem={}", 
                    cert_content.len(),
                    cert_content.starts_with("-----BEGIN")
                );
                
                // Check if certificate is base64 encoded and decode if necessary
                let decoded_cert = if cert_content.starts_with("-----BEGIN") {
                    println!("VGS CERT: Certificate already in PEM format, using as-is");
                    cert_content
                } else {
                    println!("VGS CERT: Certificate appears to be base64 encoded, decoding...");
                    match BASE64_ENGINE.decode(&cert_content) {
                        Ok(decoded_bytes) => {
                            let decoded_string = String::from_utf8(decoded_bytes).map_err(|e| {
                                VaultMetadataError::CertificateValidationFailed(
                                    format!("Certificate is not valid UTF-8 after base64 decoding: {e}")
                                )
                            })?;
                            println!("VGS CERT: Successfully decoded base64 certificate - decoded_cert_length={}", 
                                decoded_string.len()
                            );
                            decoded_string
                        }
                        Err(e) => {
                            println!("VGS CERT ERROR: Failed to decode base64 certificate: {}, cert_length={}", 
                                e,
                                cert_content.len()
                            );
                            return Err(VaultMetadataError::CertificateValidationFailed(
                                format!("Failed to decode base64 certificate: {e}")
                            ));
                        }
                    }
                };
                
                connection_config.ca_cert = Some(Secret::new(decoded_cert.clone()));
                
                println!("VGS COMPLETE: Successfully applied VGS vault metadata to connection config - proxy_url={}, proxy_url_as_str={}, proxy_url_set={}, ca_cert_set={}, ca_cert_length={}", 
                    self.proxy_url,
                    self.proxy_url.as_str(),
                    connection_config.proxy_url.is_some(),
                    connection_config.ca_cert.is_some(),
                    decoded_cert.len()
                );
                
                Ok(())
            }

            fn vault_connector(&self) -> VaultConnectors {
                VaultConnectors::VGS
            }
        }

        impl VgsMetadata {
            /// Validate the proxy URL
            fn validate_proxy_url(&self) -> Result<(), VaultMetadataError> {
                let url_str = self.proxy_url.as_str();
                
                // Check if URL has HTTPS scheme for security
                if self.proxy_url.scheme() != "https" {
                    return Err(VaultMetadataError::url_validation_failed(
                        "proxy_url",
                        url_str,
                        "VGS proxy URL must use HTTPS scheme for security"
                    ));
                }

                // Check if URL has a host
                if self.proxy_url.host().is_none() {
                    return Err(VaultMetadataError::url_validation_failed(
                        "proxy_url",
                        url_str,
                        "Proxy URL must have a valid host"
                    ));
                }

                // Check if URL has a port (VGS typically uses specific ports)
                if self.proxy_url.port().is_none() {
                    tracing::warn!(
                        proxy_url = %self.proxy_url,
                        "VGS proxy URL does not specify a port, using default HTTPS port 443"
                    );
                }

                Ok(())
            }

            /// Validate the certificate format and content
            fn validate_certificate(&self) -> Result<(), VaultMetadataError> {
                let cert_content = self.certificate.clone().expose();
                
                // Only check that certificate is not empty - let the HTTP client handle the rest
                if cert_content.trim().is_empty() {
                    return Err(VaultMetadataError::CertificateValidationFailed(
                        "Certificate content is empty".to_string()
                    ));
                }

                tracing::debug!("Certificate validation passed (non-empty check only)");
                Ok(())
            }
        }

        impl VaultMetadataProcessor for ExternalVaultProxyMetadata {
            fn process_metadata(&self, connection_config: &mut ConnectionConfig) -> Result<(), VaultMetadataError> {
                match self {
                    Self::VgsMetadata(vgs_metadata) => {
                        vgs_metadata.process_metadata(connection_config)
                    }
                }
            }

            fn vault_connector(&self) -> VaultConnectors {
                match self {
                    Self::VgsMetadata(vgs_metadata) => vgs_metadata.vault_connector(),
                }
            }
        }

        /// Factory for creating vault metadata processors from different sources
        pub struct VaultMetadataFactory;

        impl VaultMetadataFactory {
            /// Create a vault metadata processor from base64 encoded header value with comprehensive validation
            pub fn from_base64_header(base64_value: &str) -> Result<Box<dyn VaultMetadataProcessor>, VaultMetadataError> {
                // Validate input
                if base64_value.trim().is_empty() {
                    return Err(VaultMetadataError::EmptyOrMalformedHeader);
                }

                // Log the attempt (without exposing sensitive data)
                tracing::debug!(
                    header_length = base64_value.len(),
                    "Processing vault metadata from base64 header"
                );

                // Decode base64 with detailed error context
                let decoded_bytes = BASE64_ENGINE
                    .decode(base64_value.trim())
                    .map_err(|e| {
                        tracing::error!(
                            error = %e,
                            header_length = base64_value.len(),
                            "Failed to decode base64 vault metadata header"
                        );
                        VaultMetadataError::Base64DecodingFailed(format!(
                            "Invalid base64 encoding: {}. Header length: {}",
                            e, base64_value.len()
                        ))
                    })?;

                // Validate decoded size
                if decoded_bytes.is_empty() {
                    return Err(VaultMetadataError::EmptyOrMalformedHeader);
                }

                if decoded_bytes.len() > 1_000_000 {
                    return Err(VaultMetadataError::JsonParsingFailed(
                        "Decoded vault metadata is too large (>1MB)".to_string()
                    ));
                }

                // Parse JSON with detailed error context
                let metadata: ExternalVaultProxyMetadata = serde_json::from_slice(&decoded_bytes)
                    .map_err(|e| {
                        tracing::error!(
                            error = %e,
                            decoded_size = decoded_bytes.len(),
                            "Failed to parse vault metadata JSON"
                        );
                        VaultMetadataError::JsonParsingFailed(format!(
                            "Invalid JSON structure: {}. Size: {} bytes",
                            e, decoded_bytes.len()
                        ))
                    })?;

                tracing::info!(
                    vault_connector = ?metadata.vault_connector(),
                    "Successfully parsed vault metadata from header"
                );

                Ok(Box::new(metadata))
            }

            /// Create a vault metadata processor from URL and certificate with validation
            pub fn create_vgs_metadata(proxy_url: Url, certificate: Secret<String>) -> Result<Box<dyn VaultMetadataProcessor>, VaultMetadataError> {
                let vgs_metadata = VgsMetadata {
                    proxy_url,
                    certificate,
                };

                // Validate the created metadata
                vgs_metadata.validate_proxy_url()?;
                vgs_metadata.validate_certificate()?;

                tracing::debug!(
                    proxy_url = %vgs_metadata.proxy_url,
                    "Created and validated VGS metadata"
                );

                Ok(Box::new(vgs_metadata))
            }

            /// Create a vault metadata processor with explicit validation
            pub fn create_and_validate_vgs_metadata(
                proxy_url_str: &str,
                certificate: Secret<String>,
            ) -> Result<Box<dyn VaultMetadataProcessor>, VaultMetadataError> {
                // Parse and validate URL
                let proxy_url = Url::parse(proxy_url_str)
                    .map_err(|e| VaultMetadataError::url_validation_failed(
                        "proxy_url",
                        proxy_url_str,
                        format!("URL parsing failed: {e}")
                    ))?;

                Self::create_vgs_metadata(proxy_url, certificate)
            }
        }

        /// Trait for extracting vault metadata from various sources
        pub trait VaultMetadataExtractor {
            /// Extract vault metadata from headers and apply to connection config
            fn extract_and_apply_vault_metadata(&mut self, headers: &HashMap<String, Secret<String>>) -> Result<(), VaultMetadataError>;
        }

        impl VaultMetadataExtractor for ConnectionConfig {
            fn extract_and_apply_vault_metadata(&mut self, headers: &HashMap<String, Secret<String>>) -> Result<(), VaultMetadataError> {
                if let Some(vault_metadata_header) = headers.get(EXTERNAL_VAULT_METADATA_HEADER) {
                    println!("VAULT DEBUG: Found vault metadata header, processing... header_length={}", 
                        vault_metadata_header.clone().expose().len()
                    );
                    
                    let processor = VaultMetadataFactory::from_base64_header(&vault_metadata_header.clone().expose())
                        .map_err(|e| {
                            println!("VAULT ERROR: Failed to create vault metadata processor from header: {}, header_length={}", 
                                e,
                                vault_metadata_header.clone().expose().len()
                            );
                            e
                        })?;
                    
                    println!("VAULT DEBUG: Created vault metadata processor {:?}, applying to connection config...", 
                        processor.vault_connector()
                    );
                    
                    processor.process_metadata(self)
                        .map_err(|e| {
                            println!("VAULT ERROR: Failed to apply vault metadata to connection config: {}, vault_connector={:?}", 
                                e,
                                processor.vault_connector()
                            );
                            e
                        })?;

                    println!("VAULT SUCCESS: Successfully applied vault metadata to connection configuration - vault_connector={:?}, proxy_url_applied={}, ca_cert_applied={}, client_cert_applied={}", 
                        processor.vault_connector(),
                        self.proxy_url.is_some(),
                        self.ca_cert.is_some(),
                        self.client_cert.is_some()
                    );
                } else {
                    println!("VAULT DEBUG: No vault metadata header found, available_headers={:?}", 
                        headers.keys().collect::<Vec<_>>()
                    );
                }
                Ok(())
            }

        }

        /// Extended trait for graceful fallback handling
        pub trait VaultMetadataExtractorExt {
            /// Extract vault metadata with graceful fallback (doesn't fail the entire request)
            fn extract_and_apply_vault_metadata_with_fallback(&mut self, headers: &HashMap<String, Secret<String>>) -> bool;
        }

        impl VaultMetadataExtractorExt for ConnectionConfig {
            fn extract_and_apply_vault_metadata_with_fallback(&mut self, headers: &HashMap<String, Secret<String>>) -> bool {
                println!("VAULT DEBUG: Starting vault metadata extraction with fallback, header_count={}, has_vault_metadata={}", 
                    headers.len(), 
                    headers.contains_key(EXTERNAL_VAULT_METADATA_HEADER)
                );
                
                match self.extract_and_apply_vault_metadata(headers) {
                    Ok(()) => {
                        println!("VAULT SUCCESS: Vault metadata processing completed successfully - proxy_url_set={}, ca_cert_set={}, client_cert_set={}", 
                            self.proxy_url.is_some(),
                            self.ca_cert.is_some(),
                            self.client_cert.is_some()
                        );
                        true
                    }
                    Err(e) => {
                        println!("VAULT ERROR: Vault metadata processing failed: {}, proxy_url_set={}, ca_cert_set={}", 
                            e,
                            self.proxy_url.is_some(),
                            self.ca_cert.is_some()
                        );
                        false
                    }
                }
            }
        }


        #[cfg(test)]
        mod tests {
            use super::*;
            use base64::Engine;
            use masking::ExposeInterface;
            use common_utils::pii::SecretSerdeValue;

            #[test]
            fn test_vault_metadata_processing() {
                // Create test VGS metadata with base64 encoded certificate
                let vgs_metadata = VgsMetadata {
                    proxy_url: "https://vgs-proxy.example.com:8443".parse().expect("Valid test URL"),
                    certificate: Secret::new("LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0tCk1JSUQyVENDQXNHZ0F3SUJBZ0lIQU40R3MvTEdoekFOQmdrcWhraUc5dzBCQVEwRkFEQjVNU1F3SWdZRFZRUUQKREJzcUxuTmhibVJpYjNndWRtVnllV2R2YjJSd2NtOTRlUzVqYjIweElUQWZCZ05WQkFvTUdGWmxjbmtnUjI5dgpaQ0JUWldOMWNtbDBlU3dnU1c1akxqRXVNQ3dHQTFVRUN3d2xWbVZ5ZVNCSGIyOWtJRk5sWTNWeWFYUjVJQzBnClJXNW5hVzVsWlhKcGJtY2dWR1ZoYlRBZ0Z3MHhOakF5TURreU16VXpNelphR0E4eU1URTNNREV4TlRJek5UTXoKTmxvd2VURWtNQ0lHQTFVRUF3d2JLaTV6WVc1a1ltOTRMblpsY25sbmIyOWtjSEp2ZUhrdVkyOXRNU0V3SHdZRApWUVFLREJoV1pYSjVJRWR2YjJRZ1UyVmpkWEpwZEhrc0lFbHVZeTR4TGpBc0JnTlZCQXNNSlZabGNua2dSMjl2ClpDQlRaV04xY21sMGVTQXRJRVZ1WjJsdVpXVnlhVzVuSUZSbFlXMHdnZ0VpTUEwR0NTcUdTSWIzRFFFQkFRVUEKQTRJQkR3QXdnZ0VLQW9JQkFRREkzdWtIcHhJbERDdkZqcHFuNGdBa3JRVmRXbGwvdUkwS3Yzd2lyd1ozUXJwZwpCVmVYakluSityVjlyMG91QklvWThJZ1JMYWs1SHkvdFNlVjZuQVZIdjB0NDFCN1Z5b2VUQXNaWVNXVTExZGVSCkRCU0JYSFdIOXpLRXZYa2tQZHk5dGdIbnZMSXp1aTJINTlPUGxqVjd6M3NDTGd1Ukl2SUl3OGRqYVY5ejdGUm0KS1JzZm1ZSEtPQmxTTzRUbHBmWFFnN2pRNWRzNjVxOEZGR3ZUQjVxQWdMWFM4VzhwdmRrOGpjY211elFYRlVZKwpadEhnalRoZzdCSFdXVW4rN202aFE2aUhIQ2ozNFF1NjlGOG5MYW1kK0tKLy8xNGx1a2R5S3MzQU1yWXNGYWJ5CmsrVUdlbU0vczJxM0IrMzlCNllLYUhhbzBTUnpTSkM3cUR3YldQeTNBZ01CQUFHalpEQmlNQjBHQTFVZERnUVcKQkJSV2xJUnJFMnAyUDAxOFZUelRiNkJhZU9GaEF6QVBCZ05WSFJNQkFmOEVCVEFEQVFIL01Bc0dBMVVkRHdRRQpBd0lCdGpBakJnTlZIU1VFSERBYUJnZ3JCZ0VGQlFjREFRWUlLd1lCQlFVSEF3SUdCRlVkSlFBd0RRWUpLb1pJCmh2Y05BUUVOQlFBRGdnRUJBR1d4TEZscjBiOWxXa09MY1p0UjlJRFZ4REw5eitVUEZFazcwRDNOUGFxWGtvRS8KVE5OVWtYZ1M2K1ZCQTJHOG5pZ3EyWWo4cW9JTStrVFhQYjhUeld2K2xyY0xtK2krNEFTaEtWa25wQjE1Y0MxQwovTkpmeVlHUlc2NnMvdzdITlMyMFJtcmROK2JXUzBQQTRDVkxYZEd6VUpuMFBDc2ZzUys2QWNuN1JQQUUrMEE4CldCN0p6WFdpOHg5bU9Kd2lPaG9kcDRqNDFtdis1ZUhNMHJlTWg2eWN1WWJqcXVETnBpTm5zTHp0azZNR3NnQVAKNUM1OWRyUVdKVTQ3NzM4QmNmYkJ5dVNUWUZvZzZ6TllDbTdBQ3FidGl3dkZUd2puZU5lYk9oc09sYUVBSGp1cApkNFFCcVlWczdwemtoTk5wOW9VdnY0d0dmL0tKY3c1QjlFNlRwZms9Ci0tLS0tRU5EIENFUlRJRklDQVRFLS0tLS0=".to_string()),
                };

                let metadata = ExternalVaultProxyMetadata::VgsMetadata(vgs_metadata);
                
                // Serialize and base64 encode (as it would come from the header)
                let metadata_json = serde_json::to_vec(&metadata).expect("Metadata serialization should succeed");
                let base64_metadata = BASE64_ENGINE.encode(&metadata_json);

                // Create headers with vault metadata
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), Secret::new("application/json".to_string()));
                headers.insert("Authorization".to_string(), Secret::new("Bearer token123".to_string()));
                headers.insert(EXTERNAL_VAULT_METADATA_HEADER.to_string(), Secret::new(base64_metadata));

                // Test the amazing automatic processing with the unified API!
                let injector_request = InjectorRequest::new(
                    "https://api.example.com".to_string(),
                    "/v1/payments".to_string(),
                    HttpMethod::POST,
                    "amount={{$amount}}&currency={{$currency}}".to_string(),
                    TokenData {
                        vault_connector: VaultConnectors::VGS,
                        specific_token_data: SecretSerdeValue::new(serde_json::json!({
                            "amount": "1000",
                            "currency": "USD"
                        })),
                    },
                    Some(headers),
                    None, // No fallback proxy needed - vault metadata provides it
                    None, // No fallback client cert
                    None, // No fallback client key  
                    None, // No fallback CA cert
                );

                // Verify vault metadata was automatically applied!
                assert!(injector_request.connection_config.proxy_url.is_some());
                assert!(injector_request.connection_config.ca_cert.is_some());
                assert_eq!(
                    injector_request.connection_config.proxy_url.as_ref().expect("Proxy URL should be set").clone().expose(),
                    "https://vgs-proxy.example.com:8443/"
                );

                // Verify vault metadata header was removed from regular headers
                assert!(!injector_request.connection_config.headers.contains_key(EXTERNAL_VAULT_METADATA_HEADER));
                
                // Verify other headers are preserved
                assert!(injector_request.connection_config.headers.contains_key("Content-Type"));
                assert!(injector_request.connection_config.headers.contains_key("Authorization"));
            }

            #[test]
            fn test_vault_metadata_factory() {
                let vgs_metadata = VgsMetadata {
                    proxy_url: "https://vgs-proxy.example.com:8443".parse().expect("Valid test URL"),
                    certificate: Secret::new("LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0tCk1JSUQyVENDQXNHZ0F3SUJBZ0lIQU40R3MvTEdoekFOQmdrcWhraUc5dzBCQVEwRkFEQjVNU1F3SWdZRFZRUUQKREJzcUxuTmhibVJpYjNndWRtVnllV2R2YjJSd2NtOTRlUzVqYjIweElUQWZCZ05WQkFvTUdGWmxjbmtnUjI5dgpaQ0JUWldOMWNtbDBlU3dnU1c1akxqRXVNQ3dHQTFVRUN3d2xWbVZ5ZVNCSGIyOWtJRk5sWTNWeWFYUjVJQzBnClJXNW5hVzVsWlhKcGJtY2dWR1ZoYlRBZ0Z3MHhOakF5TURreU16VXpNelphR0E4eU1URTNNREV4TlRJek5UTXoKTmxvd2VURWtNQ0lHQTFVRUF3d2JLaTV6WVc1a1ltOTRMblpsY25sbmIyOWtjSEp2ZUhrdVkyOXRNU0V3SHdZRApWUVFLREJoV1pYSjVJRWR2YjJRZ1UyVmpkWEpwZEhrc0lFbHVZeTR4TGpBc0JnTlZCQXNNSlZabGNua2dSMjl2ClpDQlRaV04xY21sMGVTQXRJRVZ1WjJsdVpXVnlhVzVuSUZSbFlXMHdnZ0VpTUEwR0NTcUdTSWIzRFFFQkFRVUEKQTRJQkR3QXdnZ0VLQW9JQkFRREkzdWtIcHhJbERDdkZqcHFuNGdBa3JRVmRXbGwvdUkwS3Yzd2lyd1ozUXJwZwpCVmVYakluSityVjlyMG91QklvWThJZ1JMYWs1SHkvdFNlVjZuQVZIdjB0NDFCN1Z5b2VUQXNaWVNXVTExZGVSCkRCU0JYSFdIOXpLRXZYa2tQZHk5dGdIbnZMSXp1aTJINTlPUGxqVjd6M3NDTGd1Ukl2SUl3OGRqYVY5ejdGUm0KS1JzZm1ZSEtPQmxTTzRUbHBmWFFnN2pRNWRzNjVxOEZGR3ZUQjVxQWdMWFM4VzhwdmRrOGpjY211elFYRlVZKwpadEhnalRoZzdCSFdXVW4rN202aFE2aUhIQ2ozNFF1NjlGOG5MYW1kK0tKLy8xNGx1a2R5S3MzQU1yWXNGYWJ5CmsrVUdlbU0vczJxM0IrMzlCNllLYUhhbzBTUnpTSkM3cUR3YldQeTNBZ01CQUFHalpEQmlNQjBHQTFVZERnUVcKQkJSV2xJUnJFMnAyUDAxOFZUelRiNkJhZU9GaEF6QVBCZ05WSFJNQkFmOEVCVEFEQVFIL01Bc0dBMVVkRHdRRQpBd0lCdGpBakJnTlZIU1VFSERBYUJnZ3JCZ0VGQlFjREFRWUlLd1lCQlFVSEF3SUdCRlVkSlFBd0RRWUpLb1pJCmh2Y05BUUVOQlFBRGdnRUJBR1d4TEZscjBiOWxXa09MY1p0UjlJRFZ4REw5eitVUEZFazcwRDNOUGFxWGtvRS8KVE5OVWtYZ1M2K1ZCQTJHOG5pZ3EyWWo4cW9JTStrVFhQYjhUeld2K2xyY0xtK2krNEFTaEtWa25wQjE1Y0MxQwovTkpmeVlHUlc2NnMvdzdITlMyMFJtcmROK2JXUzBQQTRDVkxYZEd6VUpuMFBDc2ZzUys2QWNuN1JQQUUrMEE4CldCN0p6WFdpOHg5bU9Kd2lPaG9kcDRqNDFtdis1ZUhNMHJlTWg2eWN1WWJqcXVETnBpTm5zTHp0azZNR3NnQVAKNUM1OWRyUVdKVTQ3NzM4QmNmYkJ5dVNUWUZvZzZ6TllDbTdBQ3FidGl3dkZUd2puZU5lYk9oc09sYUVBSGp1cApkNFFCcVlWczdwemtoTk5wOW9VdnY0d0dmL0tKY3c1QjlFNlRwZms9Ci0tLS0tRU5EIENFUlRJRklDQVRFLS0tLS0=".to_string()),
                };

                let metadata = ExternalVaultProxyMetadata::VgsMetadata(vgs_metadata);
                let metadata_json = serde_json::to_vec(&metadata).expect("Metadata serialization should succeed");
                let base64_metadata = BASE64_ENGINE.encode(&metadata_json);

                // Test factory creation from base64
                let processor = VaultMetadataFactory::from_base64_header(&base64_metadata).expect("Base64 decoding should succeed");
                assert_eq!(processor.vault_connector(), VaultConnectors::VGS);

                // Test direct VGS creation
                let direct_processor = VaultMetadataFactory::create_vgs_metadata(
                    "https://direct.vgs.com".parse().expect("Valid test URL"),
                    Secret::new("direct-cert".to_string()),
                ).expect("VGS metadata creation should succeed");
                assert_eq!(direct_processor.vault_connector(), VaultConnectors::VGS);
            }
        }
    }
}

pub use models::*;
