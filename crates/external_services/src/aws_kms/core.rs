//! Interactions with the AWS KMS SDK
//!
//! Hyperswitch adapter over the shared [`cloud_services::kms::aws`] client: it adds logging,
//! metrics and `error-stack` reports on top of the plain client.

use std::time::Instant;

use cloud_services::kms::aws as shared;
pub use cloud_services::kms::aws::AwsKmsConfig;
use common_utils::errors::CustomResult;
use error_stack::{report, Report};
use router_env::logger;

use crate::metrics;

/// Client for AWS KMS operations.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    inner: shared::AwsKmsClient,
}

impl AwsKmsClient {
    /// Constructs a new AWS KMS client.
    pub async fn new(config: &AwsKmsConfig) -> Self {
        Self {
            inner: shared::AwsKmsClient::new(config).await,
        }
    }

    /// Decrypts the provided base64-encoded encrypted data using the AWS KMS SDK. We assume that
    /// the SDK has the values required to interact with the AWS KMS APIs (`AWS_ACCESS_KEY_ID` and
    /// `AWS_SECRET_ACCESS_KEY`) either set in environment variables, or that the SDK is running in
    /// a machine that is able to assume an IAM role.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, AwsKmsError> {
        let start = Instant::now();

        let output = self.inner.decrypt(data).await.map_err(|error| {
            if let shared::AwsKmsError::DecryptionFailed(sdk_error) = &error {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(aws_kms_sdk_error=?sdk_error, "Failed to AWS KMS decrypt data");
                metrics::AWS_KMS_DECRYPTION_FAILURES.add(1, &[]);
            }
            into_report(error)
        })?;

        let time_taken = start.elapsed();
        metrics::AWS_KMS_DECRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts the provided String data using the AWS KMS SDK. We assume that
    /// the SDK has the values required to interact with the AWS KMS APIs (`AWS_ACCESS_KEY_ID` and
    /// `AWS_SECRET_ACCESS_KEY`) either set in environment variables, or that the SDK is running in
    /// a machine that is able to assume an IAM role.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, AwsKmsError> {
        let start = Instant::now();

        let output = self.inner.encrypt(data).await.map_err(|error| {
            if let shared::AwsKmsError::EncryptionFailed(sdk_error) = &error {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(aws_kms_sdk_error=?sdk_error, "Failed to AWS KMS encrypt data");
                metrics::AWS_KMS_ENCRYPTION_FAILURES.add(1, &[]);
            }
            into_report(error)
        })?;

        let time_taken = start.elapsed();
        metrics::AWS_KMS_ENCRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }
}

/// Converts a shared client error into a report with the same frames as before: the underlying
/// error, if any, followed by the matching [`AwsKmsError`].
fn into_report(error: shared::AwsKmsError) -> Report<AwsKmsError> {
    match error {
        shared::AwsKmsError::Base64DecodingFailed(source) => {
            report!(source).change_context(AwsKmsError::Base64DecodingFailed)
        }
        shared::AwsKmsError::DecryptionFailed(source) => {
            report!(*source).change_context(AwsKmsError::DecryptionFailed)
        }
        shared::AwsKmsError::EncryptionFailed(source) => {
            report!(*source).change_context(AwsKmsError::EncryptionFailed)
        }
        shared::AwsKmsError::MissingPlaintextDecryptionOutput => {
            report!(AwsKmsError::MissingPlaintextDecryptionOutput)
        }
        shared::AwsKmsError::MissingCiphertextEncryptionOutput => {
            report!(AwsKmsError::MissingCiphertextEncryptionOutput)
        }
        shared::AwsKmsError::Utf8DecodingFailed(source) => {
            report!(source).change_context(AwsKmsError::Utf8DecodingFailed)
        }
        shared::AwsKmsError::MissingKeyId => report!(AwsKmsError::MissingKeyId),
    }
}

/// Errors that could occur during KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum AwsKmsError {
    /// An error occurred when base64 encoding input data.
    #[error("Failed to base64 encode input data")]
    Base64EncodingFailed,

    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when AWS KMS decrypting input data.
    #[error("Failed to AWS KMS decrypt input data")]
    DecryptionFailed,

    /// An error occurred when AWS KMS encrypting input data.
    #[error("Failed to AWS KMS encrypt input data")]
    EncryptionFailed,

    /// The AWS KMS decrypted output does not include a plaintext output.
    #[error("Missing plaintext AWS KMS decryption output")]
    MissingPlaintextDecryptionOutput,

    /// The AWS KMS encrypted output does not include a ciphertext output.
    #[error("Missing ciphertext AWS KMS encryption output")]
    MissingCiphertextEncryptionOutput,

    /// An error occurred UTF-8 decoding AWS KMS decrypted output.
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,

    /// The AWS KMS client has not been initialized.
    #[error("The AWS KMS client has not been initialized")]
    AwsKmsClientNotInitialized,

    /// AWS KMS key id not provided.
    #[error("AWS KMS key id not provided")]
    MissingKeyId,
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn check_aws_kms_encryption() {
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "YOUR SECRET ACCESS KEY");
        std::env::set_var("AWS_ACCESS_KEY_ID", "YOUR AWS ACCESS KEY ID");
        use super::*;
        let config = AwsKmsConfig {
            key_id: Some("YOUR AWS KMS KEY ID".to_string()),
            region: "AWS REGION".to_string(),
        };

        let data = "hello".to_string();
        let binding = data.as_bytes();
        let kms_encrypted_fingerprint = AwsKmsClient::new(&config)
            .await
            .encrypt(binding)
            .await
            .expect("aws kms encryption failed");

        println!("{kms_encrypted_fingerprint}");
    }

    #[tokio::test]
    async fn check_aws_kms_decrypt() {
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "YOUR SECRET ACCESS KEY");
        std::env::set_var("AWS_ACCESS_KEY_ID", "YOUR AWS ACCESS KEY ID");
        use super::*;
        let config = AwsKmsConfig {
            key_id: Some("YOUR AWS KMS KEY ID".to_string()),
            region: "AWS REGION".to_string(),
        };

        // Should decrypt to hello
        let data = "AWS KMS ENCRYPTED CIPHER".to_string();
        let binding = data.as_bytes();
        let kms_encrypted_fingerprint = AwsKmsClient::new(&config)
            .await
            .decrypt(binding)
            .await
            .expect("aws kms decryption failed");

        println!("{kms_encrypted_fingerprint}");
    }

    mod error_reports {
        use super::super::*;

        fn frames(report: &Report<AwsKmsError>) -> Vec<String> {
            report
                .frames()
                .filter_map(|frame| match frame.kind() {
                    error_stack::FrameKind::Context(context) => Some(context.to_string()),
                    error_stack::FrameKind::Attachment(_) => None,
                })
                .collect()
        }

        #[tokio::test]
        async fn invalid_base64_reports_decode_error_under_base64_decoding_failed() {
            let client = AwsKmsClient::new(&AwsKmsConfig {
                key_id: Some("key".to_owned()),
                region: "us-east-1".to_owned(),
            })
            .await;

            #[allow(clippy::expect_used)]
            let report = client
                .decrypt("not base64!")
                .await
                .expect_err("invalid base64 must fail");

            assert!(matches!(
                report.current_context(),
                AwsKmsError::Base64DecodingFailed
            ));
            let frames = frames(&report);
            assert_eq!(frames.len(), 2, "{frames:?}");
            assert_eq!(
                frames.first().map(String::as_str),
                Some("Failed to base64 decode input data")
            );
        }

        #[tokio::test]
        async fn missing_key_id_reports_only_missing_key_id() {
            let client = AwsKmsClient::new(&AwsKmsConfig {
                key_id: None,
                region: "us-east-1".to_owned(),
            })
            .await;

            #[allow(clippy::expect_used)]
            let report = client
                .encrypt("hello")
                .await
                .expect_err("encrypting without a key id must fail");

            assert_eq!(frames(&report), ["AWS KMS key id not provided"]);
        }

        #[test]
        fn errors_without_a_source_map_to_a_single_frame() {
            for (error, message) in [
                (
                    shared::AwsKmsError::MissingPlaintextDecryptionOutput,
                    "Missing plaintext AWS KMS decryption output",
                ),
                (
                    shared::AwsKmsError::MissingCiphertextEncryptionOutput,
                    "Missing ciphertext AWS KMS encryption output",
                ),
            ] {
                assert_eq!(frames(&into_report(error)), [message]);
            }
        }

        #[test]
        fn utf8_error_is_kept_under_utf8_decoding_failed() {
            #[allow(clippy::expect_used)]
            let utf8_error = String::from_utf8(vec![0xff]).expect_err("0xff is not UTF-8");

            let report = into_report(shared::AwsKmsError::Utf8DecodingFailed(utf8_error));

            let frames = frames(&report);
            assert_eq!(frames.len(), 2, "{frames:?}");
            assert_eq!(
                frames.first().map(String::as_str),
                Some("Failed to UTF-8 decode decryption output")
            );
        }
    }
}
