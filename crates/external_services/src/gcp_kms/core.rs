//! Interactions with the GCP Cloud KMS SDK
//!
//! Hyperswitch adapter over the shared [`cloud_services::kms::gcp`] client: it adds logging,
//! metrics and `error-stack` reports on top of the plain client.

use std::time::Instant;

use cloud_services::kms::gcp as shared;
pub use cloud_services::kms::gcp::GcpKmsConfig;
use common_utils::errors::CustomResult;
use error_stack::{report, Report};
use router_env::logger;

use crate::metrics;

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
            .map_err(into_report)?;
        Ok(Self { inner })
    }

    /// Decrypts base64-encoded ciphertext via GCP Cloud KMS.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();

        let output = self.inner.decrypt(data).await.map_err(|error| {
            if let shared::GcpKmsError::DecryptionFailed(status) = &error {
                logger::error!(gcp_kms_error=?status, "Failed to GCP KMS decrypt data");
                metrics::GCP_KMS_DECRYPTION_FAILURES.add(1, &[]);
            }
            into_report(error)
        })?;

        let time_taken = start.elapsed();
        metrics::GCP_KMS_DECRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts data via GCP Cloud KMS, returning base64-encoded ciphertext.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, GcpKmsError> {
        let start = Instant::now();

        let output = self.inner.encrypt(data).await.map_err(|error| {
            if let shared::GcpKmsError::EncryptionFailed(status) = &error {
                logger::error!(gcp_kms_error=?status, "Failed to GCP KMS encrypt data");
                metrics::GCP_KMS_ENCRYPTION_FAILURES.add(1, &[]);
            }
            into_report(error)
        })?;

        let time_taken = start.elapsed();
        metrics::GCP_KMS_ENCRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }
}

/// Converts a shared client error into a report with the same frames as before: the underlying
/// error followed by the matching [`GcpKmsError`].
fn into_report(error: shared::GcpKmsError) -> Report<GcpKmsError> {
    match error {
        shared::GcpKmsError::Base64DecodingFailed(source) => {
            report!(source).change_context(GcpKmsError::Base64DecodingFailed)
        }
        shared::GcpKmsError::DecryptionFailed(source) => {
            report!(source).change_context(GcpKmsError::DecryptionFailed)
        }
        shared::GcpKmsError::EncryptionFailed(source) => {
            report!(source).change_context(GcpKmsError::EncryptionFailed)
        }
        shared::GcpKmsError::Utf8DecodingFailed(source) => {
            report!(source).change_context(GcpKmsError::Utf8DecodingFailed)
        }
        shared::GcpKmsError::CredentialsUnavailable(source) => {
            report!(source).change_context(GcpKmsError::ClientCreationFailed)
        }
        shared::GcpKmsError::ClientCreationFailed(source) => {
            report!(source).change_context(GcpKmsError::ClientCreationFailed)
        }
    }
}

/// Errors that could occur during GCP KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum GcpKmsError {
    /// An error occurred when base64 decoding the input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when GCP KMS decrypting the input data.
    #[error("Failed to GCP KMS decrypt input data")]
    DecryptionFailed,

    /// An error occurred when GCP KMS encrypting the input data.
    #[error("Failed to GCP KMS encrypt input data")]
    EncryptionFailed,

    /// An error occurred UTF-8 decoding the GCP KMS decrypted output.
    #[error("Failed UTF-8 decode of GCP KMS decrypted output")]
    Utf8DecodingFailed,

    /// An error occurred when creating the GCP KMS client.
    #[error("Failed to create GCP KMS client")]
    ClientCreationFailed,
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
    fn errors_keep_their_source_under_the_matching_context() {
        fn frames(report: &Report<GcpKmsError>) -> Vec<String> {
            report
                .frames()
                .filter_map(|frame| match frame.kind() {
                    error_stack::FrameKind::Context(context) => Some(context.to_string()),
                    error_stack::FrameKind::Attachment(_) => None,
                })
                .collect()
        }

        #[allow(clippy::expect_used)]
        let utf8_error = String::from_utf8(vec![0xff]).expect_err("0xff is not UTF-8");
        let report = into_report(shared::GcpKmsError::Utf8DecodingFailed(utf8_error));
        let utf8_frames = frames(&report);
        assert_eq!(utf8_frames.len(), 2, "{utf8_frames:?}");
        assert_eq!(
            utf8_frames.first().map(String::as_str),
            Some("Failed UTF-8 decode of GCP KMS decrypted output")
        );

        #[allow(clippy::expect_used)]
        let base64_error = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode("not base64!")
                .expect_err("invalid base64")
        };
        let report = into_report(shared::GcpKmsError::Base64DecodingFailed(base64_error));
        let base64_frames = frames(&report);
        assert_eq!(base64_frames.len(), 2, "{base64_frames:?}");
        assert_eq!(
            base64_frames.first().map(String::as_str),
            Some("Failed to base64 decode input data")
        );
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
