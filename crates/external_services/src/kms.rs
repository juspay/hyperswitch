//! Encryption schemes

use masking::{PeekInterface, Secret};

#[cfg(feature = "aws_kms")]
use crate::kms::aws_kms::{AwsKmsClient, AwsKmsConfig};
use crate::kms::no_encryption::NoEncryption;

#[cfg(feature = "aws_kms")]
pub mod aws_kms;

pub mod no_encryption;

pub mod decrypt;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(tag = "encryption_scheme")]
#[serde(rename_all = "snake_case")]
pub enum KmsConfig {
    #[cfg(feature = "aws_kms")]
    AwsKms(AwsKmsConfig),
    #[default]
    NoEncryption,
}

#[async_trait::async_trait]
pub trait Encryption<I, O> {
    type ReturnType<T>;

    async fn encrypt(&self, input: I) -> Self::ReturnType<O>;

    async fn decrypt(&self, input: O) -> Self::ReturnType<I>;
}

pub async fn get_kms_client(config: &KmsConfig) -> EncryptionScheme {
    match config {
        #[cfg(feature = "aws_kms")]
        KmsConfig::AwsKms(aws_kms) => EncryptionScheme::AwsKms {
            client: aws_kms::get_aws_kms_client(aws_kms).await,
        },
        KmsConfig::NoEncryption => EncryptionScheme::None(NoEncryption),
    }
}

#[derive(Debug, Clone)]
pub enum EncryptionScheme {
    #[cfg(feature = "aws_kms")]
    AwsKms {
        client: &'static AwsKmsClient,
    },
    None(NoEncryption),
}

#[async_trait::async_trait]
impl Encryption<String, String> for EncryptionScheme {
    type ReturnType<T> = error_stack::Result<T, KmsError>;

    async fn encrypt(&self, input: String) -> Self::ReturnType<String> {
        match self {
            #[cfg(feature = "aws_kms")]
            Self::AwsKms { client } => client.encrypt(input).await,
            Self::None(no_encryption) => Ok(no_encryption.encrypt(input)),
        }
    }

    async fn decrypt(&self, input: String) -> Self::ReturnType<String> {
        match self {
            #[cfg(feature = "aws_kms")]
            Self::AwsKms { client } => client.decrypt(input).await,
            Self::None(no_encryption) => Ok(no_encryption.decrypt(input)),
        }
    }
}

/// Errors that could occur during KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum KmsError {
    /// An error occurred when base64 encoding input data.
    #[error("Failed to base64 encode input data")]
    Base64EncodingFailed,

    /// An error occurred when base64 decoding input data.
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,

    /// An error occurred when KMS decrypting input data.
    #[error("Failed to KMS decrypt input data")]
    DecryptionFailed,

    /// An error occurred when AWS KMS encrypting input data.
    #[error("Failed to KMS encrypt input data")]
    EncryptionFailed,

    /// The KMS decrypted output does not include a plaintext output.
    #[error("Missing plaintext KMS decryption output")]
    MissingPlaintextDecryptionOutput,

    /// The KMS encrypted output does not include a ciphertext output.
    #[error("Missing ciphertext KMS encryption output")]
    MissingCiphertextEncryptionOutput,

    /// An error occurred UTF-8 decoding KMS decrypted output.
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,

    /// The KMS client has not been initialized.
    #[error("The {encryption_scheme} client has not been initialized")]
    KmsClientNotInitialized { encryption_scheme: &'static str },
}

/// A wrapper around a AWS KMS value that can be decrypted.
#[derive(Clone, Debug, Default, serde::Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct KmsValue(Secret<String>);

impl common_utils::ext_traits::ConfigExt for KmsValue {
    fn is_empty_after_trim(&self) -> bool {
        self.0.peek().is_empty_after_trim()
    }
}
