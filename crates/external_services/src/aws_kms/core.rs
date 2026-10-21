//! Interactions with the AWS KMS SDK
//!
//! Hyperswitch adapter over the shared [`cloud_services::kms::aws`] client: it adds logging,
//! metrics and `error-stack` reports on top of the plain client.

use std::time::Instant;

use cloud_services::kms::aws as shared;
pub use cloud_services::kms::aws::{AwsKmsConfig, AwsKmsError};
use common_utils::errors::CustomResult;
use router_env::logger;

use crate::{metrics, report_with_cause};

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
            if let AwsKmsError::DecryptionFailed(sdk_error) = &error {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(aws_kms_sdk_error=?sdk_error, "Failed to AWS KMS decrypt data");
                metrics::AWS_KMS_DECRYPTION_FAILURES.add(1, &[]);
            }
            report_with_cause(error)
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
            if let AwsKmsError::EncryptionFailed(sdk_error) = &error {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(aws_kms_sdk_error=?sdk_error, "Failed to AWS KMS encrypt data");
                metrics::AWS_KMS_ENCRYPTION_FAILURES.add(1, &[]);
            }
            report_with_cause(error)
        })?;

        let time_taken = start.elapsed();
        metrics::AWS_KMS_ENCRYPT_TIME.record(time_taken.as_secs_f64(), &[]);

        Ok(output)
    }
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
        use error_stack::{AttachmentKind, FrameKind};

        use super::super::*;

        #[tokio::test]
        async fn invalid_base64_report_keeps_the_decode_error() {
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
                AwsKmsError::Base64DecodingFailed(_)
            ));
            let printed = format!("{report:?}");
            assert!(
                printed.contains("Failed to base64 decode input data"),
                "{printed}"
            );
            assert!(printed.contains("Invalid symbol"), "{printed}");
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

            assert!(matches!(
                report.current_context(),
                AwsKmsError::MissingKeyId
            ));
            let printable_attachments = report
                .frames()
                .filter(|frame| {
                    matches!(
                        frame.kind(),
                        FrameKind::Attachment(AttachmentKind::Printable(_))
                    )
                })
                .count();
            assert_eq!(printable_attachments, 0);
        }
    }
}
