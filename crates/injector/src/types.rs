pub mod models {
    use std::collections::HashMap;

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
            logger::debug!(
                header_count = headers.len(),
                has_vault_metadata = headers.contains_key(vault_metadata::EXTERNAL_VAULT_METADATA_HEADER),
                "Processing injector request with headers"
            );
            let vault_applied = {
                use vault_metadata::VaultMetadataExtractorExt;
                connection_config.extract_and_apply_vault_metadata_with_fallback(&headers)
            };
            logger::debug!(
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

            logger::debug!(
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
                logger::debug!(
                    proxy_url = %self.proxy_url,
                    proxy_url_scheme = self.proxy_url.scheme(),
                    proxy_url_host = ?self.proxy_url.host(),
                    proxy_url_port = ?self.proxy_url.port(),
                    "Starting VGS metadata processing"
                );
                
                // Validate and set proxy URL from VGS metadata
                self.validate_proxy_url()?;
                let proxy_url_str = self.proxy_url.as_str().to_string();
                connection_config.proxy_url = Some(Secret::new(proxy_url_str.clone()));
                
                logger::info!(
                    original_proxy_url = %self.proxy_url,
                    processed_proxy_url = %proxy_url_str,
                    proxy_url_length = proxy_url_str.len(),
                    "Set proxy URL from VGS metadata"
                );
                
                // Validate and decode certificate from VGS metadata
                self.validate_certificate()?;
                let cert_content = self.certificate.clone().expose();
                
                logger::debug!(
                    cert_length = cert_content.len(),
                    cert_starts_with_pem = cert_content.starts_with("-----BEGIN"),
                    "Processing certificate from VGS metadata"
                );
                
                // Check if certificate is base64 encoded and decode if necessary
                let decoded_cert = if cert_content.starts_with("-----BEGIN") {
                    logger::debug!("Certificate already in PEM format, using as-is");
                    cert_content
                } else {
                    logger::debug!("Certificate appears to be base64 encoded, decoding...");
                    match BASE64_ENGINE.decode(&cert_content) {
                        Ok(decoded_bytes) => {
                            let decoded_string = String::from_utf8(decoded_bytes).map_err(|e| {
                                VaultMetadataError::CertificateValidationFailed(
                                    format!("Certificate is not valid UTF-8 after base64 decoding: {e}")
                                )
                            })?;
                            logger::debug!(
                                decoded_cert_length = decoded_string.len(),
                                "Successfully decoded base64 certificate"
                            );
                            decoded_string
                        }
                        Err(e) => {
                            logger::error!(
                                error = %e,
                                cert_length = cert_content.len(),
                                "Failed to decode base64 certificate"
                            );
                            return Err(VaultMetadataError::CertificateValidationFailed(
                                format!("Failed to decode base64 certificate: {e}")
                            ));
                        }
                    }
                };
                
                connection_config.ca_cert = Some(Secret::new(decoded_cert.clone()));
                
                logger::info!(
                    proxy_url = %self.proxy_url,
                    proxy_url_as_str = self.proxy_url.as_str(),
                    proxy_url_set = connection_config.proxy_url.is_some(),
                    ca_cert_set = connection_config.ca_cert.is_some(),
                    ca_cert_length = decoded_cert.len(),
                    "Successfully applied VGS vault metadata to connection config"
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
                    logger::warn!(
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

                logger::debug!("Certificate validation passed (non-empty check only)");
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
                logger::debug!(
                    header_length = base64_value.len(),
                    "Processing vault metadata from base64 header"
                );

                // Decode base64 with detailed error context
                let decoded_bytes = BASE64_ENGINE
                    .decode(base64_value.trim())
                    .map_err(|e| {
                        logger::error!(
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
                        logger::error!(
                            error = %e,
                            decoded_size = decoded_bytes.len(),
                            "Failed to parse vault metadata JSON"
                        );
                        VaultMetadataError::JsonParsingFailed(format!(
                            "Invalid JSON structure: {}. Size: {} bytes",
                            e, decoded_bytes.len()
                        ))
                    })?;

                logger::info!(
                    vault_connector = ?metadata.vault_connector(),
                    "Successfully parsed vault metadata from header"
                );

                Ok(Box::new(metadata))
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
                    logger::debug!(
                        header_length = vault_metadata_header.clone().expose().len(),
                        "Found vault metadata header, processing..."
                    );
                    
                    let processor = VaultMetadataFactory::from_base64_header(&vault_metadata_header.clone().expose())
                        .map_err(|e| {
                            logger::error!(
                                error = %e,
                                header_length = vault_metadata_header.clone().expose().len(),
                                "Failed to create vault metadata processor from header"
                            );
                            e
                        })?;
                    
                    logger::debug!(
                        vault_connector = ?processor.vault_connector(),
                        "Created vault metadata processor, applying to connection config..."
                    );
                    
                    processor.process_metadata(self)
                        .map_err(|e| {
                            logger::error!(
                                error = %e,
                                vault_connector = ?processor.vault_connector(),
                                "Failed to apply vault metadata to connection config"
                            );
                            e
                        })?;

                    logger::info!(
                        vault_connector = ?processor.vault_connector(),
                        proxy_url_applied = self.proxy_url.is_some(),
                        ca_cert_applied = self.ca_cert.is_some(),
                        client_cert_applied = self.client_cert.is_some(),
                        "Successfully applied vault metadata to connection configuration"
                    );
                } else {
                    logger::debug!(
                        available_headers = ?headers.keys().collect::<Vec<_>>(),
                        "No vault metadata header found, skipping vault configuration"
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
                match self.extract_and_apply_vault_metadata(headers) {
                    Ok(()) => {
                        logger::info!(
                            proxy_url_set = self.proxy_url.is_some(),
                            ca_cert_set = self.ca_cert.is_some(),
                            client_cert_set = self.client_cert.is_some(),
                            "Vault metadata processing completed successfully"
                        );
                        true
                    }
                    Err(e) => {
                        logger::warn!(
                            error = %e,
                            proxy_url_set = self.proxy_url.is_some(),
                            ca_cert_set = self.ca_cert.is_some(),
                            "Vault metadata processing failed, continuing without vault configuration"
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

                // Test processor creation was successful
                assert!(processor.vault_connector() == VaultConnectors::VGS);
            }
        }
    }
}

pub use models::*;
