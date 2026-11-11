//! Interactions with the AWS KMS SDK

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_kms::{
    config::Region,
    error::SdkError,
    operation::{decrypt::DecryptError, encrypt::EncryptError},
    primitives::Blob,
    Client,
};
use base64::Engine;

/// Base64 engine used to encode ciphertext returned to, and decode ciphertext received from, callers.
const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Configuration parameters required for constructing a [`AwsKmsClient`].
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct AwsKmsConfig {
    /// The AWS key identifier of the KMS key used to encrypt or decrypt data.
    pub key_id: Option<String>,

    /// The AWS region to send KMS requests to.
    pub region: String,
}

impl AwsKmsConfig {
    /// Verifies that the [`AwsKmsClient`] configuration is usable.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.region.trim().is_empty() {
            return Err("KMS AWS region must not be empty");
        }

        Ok(())
    }
}

/// Client for AWS KMS operations.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    inner_client: Client,
    key_id: Option<String>,
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
    pub async fn decrypt(&self, data: impl AsRef<[u8]>) -> Result<String, AwsKmsError> {
        let data = BASE64_ENGINE
            .decode(data)
            .map_err(AwsKmsError::Base64DecodingFailed)?;
        let ciphertext_blob = Blob::new(data);

        let mut decryption_builder = self.inner_client.decrypt();

        if let Some(key_id) = &self.key_id {
            decryption_builder = decryption_builder.key_id(key_id);
        }

        let decrypt_output = decryption_builder
            .ciphertext_blob(ciphertext_blob)
            .send()
            .await
            .map_err(|error| AwsKmsError::DecryptionFailed(Box::new(error)))?;

        decrypt_output
            .plaintext
            .ok_or(AwsKmsError::MissingPlaintextDecryptionOutput)
            .and_then(|blob| {
                String::from_utf8(blob.into_inner()).map_err(AwsKmsError::Utf8DecodingFailed)
            })
    }

    /// Encrypts the provided String data using the AWS KMS SDK. We assume that
    /// the SDK has the values required to interact with the AWS KMS APIs (`AWS_ACCESS_KEY_ID` and
    /// `AWS_SECRET_ACCESS_KEY`) either set in environment variables, or that the SDK is running in
    /// a machine that is able to assume an IAM role.
    pub async fn encrypt(&self, data: impl AsRef<[u8]>) -> Result<String, AwsKmsError> {
        let plaintext_blob = Blob::new(data.as_ref());

        let mut encryption_builder = self.inner_client.encrypt();

        match &self.key_id {
            Some(key_id) => encryption_builder = encryption_builder.key_id(key_id),
            None => return Err(AwsKmsError::MissingKeyId),
        };
        let encrypted_output = encryption_builder
            .plaintext(plaintext_blob)
            .send()
            .await
            .map_err(|error| AwsKmsError::EncryptionFailed(Box::new(error)))?;

        encrypted_output
            .ciphertext_blob
            .ok_or(AwsKmsError::MissingCiphertextEncryptionOutput)
            .map(|blob| BASE64_ENGINE.encode(blob.into_inner()))
    }
}

/// Errors that could occur during KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum AwsKmsError {
    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed(#[source] base64::DecodeError),

    /// An error occurred when AWS KMS decrypting input data.
    #[error("Failed to AWS KMS decrypt input data")]
    DecryptionFailed(#[source] Box<SdkError<DecryptError>>),

    /// An error occurred when AWS KMS encrypting input data.
    #[error("Failed to AWS KMS encrypt input data")]
    EncryptionFailed(#[source] Box<SdkError<EncryptError>>),

    /// The AWS KMS decrypted output does not include a plaintext output.
    #[error("Missing plaintext AWS KMS decryption output")]
    MissingPlaintextDecryptionOutput,

    /// The AWS KMS encrypted output does not include a ciphertext output.
    #[error("Missing ciphertext AWS KMS encryption output")]
    MissingCiphertextEncryptionOutput,

    /// An error occurred UTF-8 decoding AWS KMS decrypted output.
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed(#[source] std::string::FromUtf8Error),

    /// AWS KMS key id not provided.
    #[error("AWS KMS key id not provided")]
    MissingKeyId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(key_id: Option<&str>) -> AwsKmsConfig {
        AwsKmsConfig {
            key_id: key_id.map(ToOwned::to_owned),
            region: "us-east-1".to_owned(),
        }
    }

    #[test]
    fn validate_requires_region() {
        assert!(config(None).validate().is_ok());
        for region in ["", "   "] {
            let config = AwsKmsConfig {
                key_id: None,
                region: region.to_owned(),
            };
            assert_eq!(config.validate(), Err("KMS AWS region must not be empty"));
        }
    }

    #[tokio::test]
    async fn decrypt_rejects_invalid_base64_before_calling_kms() {
        let client = AwsKmsClient::new(&config(Some("key"))).await;

        let error = client.decrypt("not base64!").await.err();

        assert!(
            matches!(error, Some(AwsKmsError::Base64DecodingFailed(_))),
            "{error:?}"
        );
    }

    #[tokio::test]
    async fn encrypt_requires_key_id_before_calling_kms() {
        let client = AwsKmsClient::new(&config(None)).await;

        let error = client.encrypt("hello").await.err();

        assert!(
            matches!(error, Some(AwsKmsError::MissingKeyId)),
            "{error:?}"
        );
    }
}
