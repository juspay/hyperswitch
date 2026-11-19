//! Interactions with the GCP Cloud KMS SDK
//!
//! Hyperswitch adapter over the shared [`cloud_services::kms::gcp`] client: it adds logging,
//! metrics and `error-stack` reports on top of the plain client.

use std::time::Instant;

use cloud_services::kms::gcp as shared;
pub use cloud_services::kms::gcp::{GcpKmsConfig, GcpKmsError};
use common_utils::errors::CustomResult;
use router_env::logger;

use crate::{metrics, report_with_cause};

/// Client for GCP Cloud KMS operations.
#[derive(Clone, Debug)]
pub struct GcpKmsClient {
    inner: shared::GcpKmsClient,
}

impl GcpKmsClient {
    /// Constructs a new GCP KMS client with ambient credentials, targeting the KMS key
    /// identified by the provided [`GcpKmsConfig`].
    pub async fn new(config: &GcpKmsConfig) -> CustomResult<Self, GcpKmsError> {
        let inner = shared::GcpKmsClient::new(config)
            .await
            .map_err(report_with_cause)?;
        Ok(Self { inner })
    }

    /// Decrypts base64-encoded ciphertext via GCP Cloud KMS.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();

        let output = self.inner.decrypt(data).await.map_err(|error| {
            if let GcpKmsError::DecryptionFailed(status) = &error {
                logger::error!(gcp_kms_error=?status, "Failed to GCP KMS decrypt data");
                metrics::GCP_KMS_DECRYPTION_FAILURES.add(1, &[]);
            }
            report_with_cause(error)
        })?;

        let time_taken = start.elapsed();
        metrics::GCP_KMS_DECRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts data via GCP Cloud KMS, returning base64-encoded ciphertext.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();

        let output = self.inner.encrypt(data).await.map_err(|error| {
            if let GcpKmsError::EncryptionFailed(status) = &error {
                logger::error!(gcp_kms_error=?status, "Failed to GCP KMS encrypt data");
                metrics::GCP_KMS_ENCRYPTION_FAILURES.add(1, &[]);
            }
            report_with_cause(error)
        })?;

        let time_taken = start.elapsed();
        metrics::GCP_KMS_ENCRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_fails_when_project_id_is_empty() {
        let config = GcpKmsConfig {
            project_id: String::new(),
            location_id: "global".to_string(),
            key_ring_id: "key-ring".to_string(),
            key_id: "key".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_location_id_is_empty() {
        let config = GcpKmsConfig {
            project_id: "project".to_string(),
            location_id: String::new(),
            key_ring_id: "key-ring".to_string(),
            key_id: "key".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_key_ring_id_is_empty() {
        let config = GcpKmsConfig {
            project_id: "project".to_string(),
            location_id: "global".to_string(),
            key_ring_id: String::new(),
            key_id: "key".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_key_id_is_empty() {
        let config = GcpKmsConfig {
            project_id: "project".to_string(),
            location_id: "global".to_string(),
            key_ring_id: "key-ring".to_string(),
            key_id: String::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_succeeds_when_all_fields_are_populated() {
        let config = GcpKmsConfig {
            project_id: "project".to_string(),
            location_id: "global".to_string(),
            key_ring_id: "key-ring".to_string(),
            key_id: "key".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn reports_keep_the_underlying_cause() {
        #[allow(clippy::expect_used)]
        let utf8_error = String::from_utf8(vec![0xff]).expect_err("0xff is not UTF-8");
        let printed = format!(
            "{:?}",
            report_with_cause(GcpKmsError::Utf8DecodingFailed(utf8_error))
        );
        assert!(
            printed.contains("Failed UTF-8 decode of GCP KMS decrypted output"),
            "{printed}"
        );
        assert!(printed.contains("invalid utf-8 sequence"), "{printed}");
    }

    #[tokio::test]
    async fn check_gcp_kms_encrypt() {
        let config = GcpKmsConfig {
            project_id: "YOUR GCP PROJECT ID".to_string(),
            location_id: "YOUR GCP KMS LOCATION ID".to_string(),
            key_ring_id: "YOUR GCP KMS KEY RING ID".to_string(),
            key_id: "YOUR GCP KMS KEY ID".to_string(),
        };

        let data = "hello".to_string();
        let gcp_kms_encrypted_fingerprint = GcpKmsClient::new(&config)
            .await
            .expect("gcp kms client creation failed")
            .encrypt(data.as_bytes())
            .await
            .expect("gcp kms encryption failed");

        println!("{gcp_kms_encrypted_fingerprint}");
    }

    #[tokio::test]
    async fn check_gcp_kms_decrypt() {
        let config = GcpKmsConfig {
            project_id: "YOUR GCP PROJECT ID".to_string(),
            location_id: "YOUR GCP KMS LOCATION ID".to_string(),
            key_ring_id: "YOUR GCP KMS KEY RING ID".to_string(),
            key_id: "YOUR GCP KMS KEY ID".to_string(),
        };

        // Should decrypt to hello
        let data = "GCP KMS ENCRYPTED CIPHER".to_string();
        let gcp_kms_decrypted_fingerprint = GcpKmsClient::new(&config)
            .await
            .expect("gcp kms client creation failed")
            .decrypt(data.as_bytes())
            .await
            .expect("gcp kms decryption failed");

        println!("{gcp_kms_decrypted_fingerprint}");
    }
}
