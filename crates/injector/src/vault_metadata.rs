use std::collections::HashMap;

use base64::Engine;
use masking::{ExposeInterface, Secret};
use router_env::logger;
use url::Url;

use crate::{consts::EXTERNAL_VAULT_METADATA_HEADER, types::ConnectionConfig, VaultConnectors};

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Trait for different vault metadata processors
pub trait VaultMetadataProcessor: Send + Sync {
    /// Process vault metadata and return connection configuration updates
    fn process_metadata(
        &self,
        connection_config: &mut ConnectionConfig,
    ) -> Result<(), VaultMetadataError>;

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
    ProcessingFailed { connector: String, reason: String },
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
    fn process_metadata(
        &self,
        connection_config: &mut ConnectionConfig,
    ) -> Result<(), VaultMetadataError> {
        // Validate and set proxy URL from VGS metadata
        let proxy_url_str = self.proxy_url.as_str().to_string();
        connection_config.proxy_url = Some(Secret::new(proxy_url_str.clone()));

        // Validate and decode certificate from VGS metadata
        let cert_content = self.certificate.clone().expose();

        // Check if certificate is base64 encoded and decode if necessary
        let decoded_cert = if cert_content.starts_with("-----BEGIN") {
            cert_content
        } else {
            match BASE64_ENGINE.decode(&cert_content) {
                Ok(decoded_bytes) => String::from_utf8(decoded_bytes).map_err(|e| {
                    VaultMetadataError::CertificateValidationFailed(format!(
                        "Certificate is not valid UTF-8 after base64 decoding: {e}"
                    ))
                })?,
                Err(e) => {
                    logger::error!(
                        error = %e,
                        "Failed to decode base64 certificate"
                    );
                    return Err(VaultMetadataError::CertificateValidationFailed(format!(
                        "Failed to decode base64 certificate: {e}"
                    )));
                }
            }
        };

        connection_config.ca_cert = Some(Secret::new(decoded_cert.clone()));

        Ok(())
    }

    fn vault_connector(&self) -> VaultConnectors {
        VaultConnectors::VGS
    }
}

impl VaultMetadataProcessor for ExternalVaultProxyMetadata {
    fn process_metadata(
        &self,
        connection_config: &mut ConnectionConfig,
    ) -> Result<(), VaultMetadataError> {
        match self {
            Self::VgsMetadata(vgs_metadata) => vgs_metadata.process_metadata(connection_config),
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
    pub fn from_base64_header(
        base64_value: &str,
    ) -> Result<Box<dyn VaultMetadataProcessor>, VaultMetadataError> {
        // Validate input
        if base64_value.trim().is_empty() {
            return Err(VaultMetadataError::EmptyOrMalformedHeader);
        }

        // Decode base64 with detailed error context
        let decoded_bytes = BASE64_ENGINE.decode(base64_value.trim()).map_err(|e| {
            logger::error!(
                error = %e,
                "Failed to decode base64 vault metadata header"
            );
            VaultMetadataError::Base64DecodingFailed(format!("Invalid base64 encoding: {e}"))
        })?;

        // Validate decoded size
        if decoded_bytes.is_empty() {
            return Err(VaultMetadataError::EmptyOrMalformedHeader);
        }

        if decoded_bytes.len() > 1_000_000 {
            return Err(VaultMetadataError::JsonParsingFailed(
                "Decoded vault metadata is too large (>1MB)".to_string(),
            ));
        }

        // Parse JSON with detailed error context
        let metadata: ExternalVaultProxyMetadata =
            serde_json::from_slice(&decoded_bytes).map_err(|e| {
                logger::error!(
                    error = %e,
                    "Failed to parse vault metadata JSON"
                );
                VaultMetadataError::JsonParsingFailed(format!("Invalid JSON structure: {e}"))
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
    fn extract_and_apply_vault_metadata(
        &mut self,
        headers: &HashMap<String, Secret<String>>,
    ) -> Result<(), VaultMetadataError>;
}

impl VaultMetadataExtractor for ConnectionConfig {
    fn extract_and_apply_vault_metadata(
        &mut self,
        headers: &HashMap<String, Secret<String>>,
    ) -> Result<(), VaultMetadataError> {
        if let Some(vault_metadata_header) = headers.get(EXTERNAL_VAULT_METADATA_HEADER) {
            let processor =
                VaultMetadataFactory::from_base64_header(&vault_metadata_header.clone().expose())
                    .map_err(|e| {
                    logger::error!(
                        error = %e,
                        "Failed to create vault metadata processor from header"
                    );
                    e
                })?;

            processor.process_metadata(self).map_err(|e| {
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
        }
        Ok(())
    }
}

/// Extended trait for graceful fallback handling
pub trait VaultMetadataExtractorExt {
    /// Extract vault metadata with graceful fallback (doesn't fail the entire request)
    fn extract_and_apply_vault_metadata_with_fallback(
        &mut self,
        headers: &HashMap<String, Secret<String>>,
    ) -> bool;

    /// Extract vault metadata from a single header value with graceful fallback
    fn extract_and_apply_vault_metadata_with_fallback_from_header(
        &mut self,
        header_value: &str,
    ) -> bool;
}

impl VaultMetadataExtractorExt for ConnectionConfig {
    fn extract_and_apply_vault_metadata_with_fallback(
        &mut self,
        headers: &HashMap<String, Secret<String>>,
    ) -> bool {
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
            Err(error) => {
                logger::warn!(
                    error = %error,
                    proxy_url_set = self.proxy_url.is_some(),
                    ca_cert_set = self.ca_cert.is_some(),
                    "Vault metadata processing failed, continuing without vault configuration"
                );
                false
            }
        }
    }

    fn extract_and_apply_vault_metadata_with_fallback_from_header(
        &mut self,
        header_value: &str,
    ) -> bool {
        let mut temp_headers = HashMap::new();
        temp_headers.insert(
            EXTERNAL_VAULT_METADATA_HEADER.to_string(),
            Secret::new(header_value.to_string()),
        );
        self.extract_and_apply_vault_metadata_with_fallback(&temp_headers)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use base64::Engine;
    use common_utils::pii::SecretSerdeValue;

    use super::*;
    use crate::types::{HttpMethod, InjectorRequest, TokenData, VaultConnectors};

    #[test]
    fn test_vault_metadata_processing() {
        // Create test VGS metadata with base64 encoded certificate
        let vgs_metadata = VgsMetadata {
            proxy_url: "https://vgs-proxy.example.com:8443"
                .parse()
                .expect("Valid test URL"),
            certificate: Secret::new("cert".to_string()),
        };

        let metadata = ExternalVaultProxyMetadata::VgsMetadata(vgs_metadata);

        // Serialize and base64 encode (as it would come from the header)
        let metadata_json =
            serde_json::to_vec(&metadata).expect("Metadata serialization should succeed");
        let base64_metadata = BASE64_ENGINE.encode(&metadata_json);

        // Create headers with vault metadata
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            Secret::new("application/json".to_string()),
        );
        headers.insert(
            "Authorization".to_string(),
            Secret::new("Bearer token123".to_string()),
        );
        headers.insert(
            EXTERNAL_VAULT_METADATA_HEADER.to_string(),
            Secret::new(base64_metadata),
        );

        // Test the amazing automatic processing with the unified API!
        let injector_request = InjectorRequest::new(
            "https://api.example.com/v1/payments".to_string(),
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
            injector_request
                .connection_config
                .proxy_url
                .as_ref()
                .expect("Proxy URL should be set")
                .clone()
                .expose(),
            "https://vgs-proxy.example.com:8443/"
        );

        // Verify vault metadata header was removed from regular headers
        assert!(!injector_request
            .connection_config
            .headers
            .contains_key(EXTERNAL_VAULT_METADATA_HEADER));

        // Verify other headers are preserved
        assert!(injector_request
            .connection_config
            .headers
            .contains_key("Content-Type"));
        assert!(injector_request
            .connection_config
            .headers
            .contains_key("Authorization"));
    }

    #[test]
    fn test_vault_metadata_factory() {
        let vgs_metadata = VgsMetadata {
            proxy_url: "https://vgs-proxy.example.com:8443"
                .parse()
                .expect("Valid test URL"),
            certificate: Secret::new("cert".to_string()),
        };

        let metadata = ExternalVaultProxyMetadata::VgsMetadata(vgs_metadata);
        let metadata_json =
            serde_json::to_vec(&metadata).expect("Metadata serialization should succeed");
        let base64_metadata = BASE64_ENGINE.encode(&metadata_json);

        // Test factory creation from base64
        let processor = VaultMetadataFactory::from_base64_header(&base64_metadata)
            .expect("Base64 decoding should succeed");
        assert_eq!(processor.vault_connector(), VaultConnectors::VGS);

        // Test processor creation was successful
        assert!(processor.vault_connector() == VaultConnectors::VGS);
    }
}
