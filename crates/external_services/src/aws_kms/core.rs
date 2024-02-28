//! Interactions with the AWS KMS SDK

use std::time::Instant;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_kms::{config::Region, primitives::Blob, Client};
use base64::Engine;
use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use router_env::logger;

use crate::{consts, metrics};

/// Configuration parameters required for constructing a [`AwsKmsClient`].
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct AwsKmsConfig {
    /// The AWS key identifier of the KMS key used to encrypt or decrypt data.
    pub key_id: String,

    /// The AWS region to send KMS requests to.
    pub region: String,
}

/// Client for AWS KMS operations.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    inner_client: Client,
    key_id: String,
}

impl AwsKmsClient {
    /// Constructs a new AWS KMS client.
    pub async fn new(config: &AwsKmsConfig) -> Self {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));
        let sdk_config = aws_config::from_env().region(region_provider).load().await;

        Self {
            inner_client: Client::new(&sdk_config),
            key_id: config.key_id.clone(),
        }
    }

    /// Decrypts the provided base64-encoded encrypted data using the AWS KMS SDK. We assume that
    /// the SDK has the values required to interact with the AWS KMS APIs (`AWS_ACCESS_KEY_ID` and
    /// `AWS_SECRET_ACCESS_KEY`) either set in environment variables, or that the SDK is running in
    /// a machine that is able to assume an IAM role.
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, AwsKmsError> {
        let start = Instant::now();
        let data = consts::BASE64_ENGINE
            .decode(data)
            .into_report()
            .change_context(AwsKmsError::Base64DecodingFailed)?;
        let ciphertext_blob = Blob::new(data);

        let decrypt_output = self
            .inner_client
            .decrypt()
            .key_id(&self.key_id)
            .ciphertext_blob(ciphertext_blob)
            .send()
            .await
            .map_err(|error| {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(aws_kms_sdk_error=?error, "Failed to AWS KMS decrypt data");
                metrics::AWS_KMS_DECRYPTION_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                error
            })
            .into_report()
            .change_context(AwsKmsError::DecryptionFailed)?;

        let output = decrypt_output
            .plaintext
            .ok_or(AwsKmsError::MissingPlaintextDecryptionOutput)
            .into_report()
            .and_then(|blob| {
                String::from_utf8(blob.into_inner())
                    .into_report()
                    .change_context(AwsKmsError::Utf8DecodingFailed)
            })?;

        let time_taken = start.elapsed();
        metrics::AWS_KMS_DECRYPT_TIME.record(&metrics::CONTEXT, time_taken.as_secs_f64(), &[]);

        Ok(output)
    }

    /// Encrypts the provided String data using the AWS KMS SDK. We assume that
    /// the SDK has the values required to interact with the AWS KMS APIs (`AWS_ACCESS_KEY_ID` and
    /// `AWS_SECRET_ACCESS_KEY`) either set in environment variables, or that the SDK is running in
    /// a machine that is able to assume an IAM role.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, AwsKmsError> {
        let start = Instant::now();
        let plaintext_blob = Blob::new(data.as_ref());

        let encrypted_output = self
            .inner_client
            .encrypt()
            .key_id(&self.key_id)
            .plaintext(plaintext_blob)
            .send()
            .await
            .map_err(|error| {
                // Logging using `Debug` representation of the error as the `Display`
                // representation does not hold sufficient information.
                logger::error!(aws_kms_sdk_error=?error, "Failed to AWS KMS encrypt data");
                metrics::AWS_KMS_ENCRYPTION_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                error
            })
            .into_report()
            .change_context(AwsKmsError::EncryptionFailed)?;

        let output = encrypted_output
            .ciphertext_blob
            .ok_or(AwsKmsError::MissingCiphertextEncryptionOutput)
            .into_report()
            .map(|blob| consts::BASE64_ENGINE.encode(blob.into_inner()))?;
        let time_taken = start.elapsed();
        metrics::AWS_KMS_ENCRYPT_TIME.record(&metrics::CONTEXT, time_taken.as_secs_f64(), &[]);

        Ok(output)
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
}

impl AwsKmsConfig {
    /// Verifies that the [`AwsKmsClient`] configuration is usable.
    pub fn validate(&self) -> Result<(), &'static str> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.key_id.is_default_or_empty(), || {
            Err("KMS AWS key ID must not be empty")
        })?;

        when(self.region.is_default_or_empty(), || {
            Err("KMS AWS region must not be empty")
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #[tokio::test]
    async fn check_aws_kms_encryption() {
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "YOUR SECRET ACCESS KEY");
        std::env::set_var("AWS_ACCESS_KEY_ID", "YOUR AWS ACCESS KEY ID");
        use super::*;
        let config = AwsKmsConfig {
            key_id: "YOUR AWS KMS KEY ID".to_string(),
            region: "AWS REGION".to_string(),
        };

        let data = "hello".to_string();
        let binding = data.as_bytes();
        let kms_encrypted_fingerprint = AwsKmsClient::new(&config)
            .await
            .encrypt(binding)
            .await
            .expect("aws kms encryption failed");

        println!("{}", kms_encrypted_fingerprint);
    }

    #[tokio::test]
    async fn check_aws_kms_decrypt() {
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "YOUR SECRET ACCESS KEY");
        std::env::set_var("AWS_ACCESS_KEY_ID", "YOUR AWS ACCESS KEY ID");
        use super::*;
        let config = AwsKmsConfig {
            key_id: "YOUR AWS KMS KEY ID".to_string(),
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

        println!("{}", kms_encrypted_fingerprint);
    }
}
