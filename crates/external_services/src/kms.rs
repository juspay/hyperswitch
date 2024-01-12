//! Interactions with the AWS KMS SDK

use std::marker::PhantomData;

use masking::{PeekInterface, Secret};

#[cfg(feature = "aws_kms")]
use crate::kms::aws_kms::{AwsKmsClient, AwsKmsConfig};
use crate::kms::no_encryption::NoEncryption;
use serde::{Deserialize, Deserializer};

/// decrypting data using the AWS KMS SDK.
pub mod decrypt;

#[cfg(feature = "aws_kms")]
pub mod aws_kms;

pub mod no_encryption;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(tag = "encryption_scheme")]
#[serde(rename_all = "snake_case")]
pub enum KmsConfig {
    #[cfg(feature = "aws_kms")]
    AwsKms { aws_kms: AwsKmsConfig },
    #[default]
    NoEncryption,
}

#[async_trait::async_trait]
pub trait Encryption<I, O> {
    type ReturnType<T>;

    async fn encrypt(&self, input: I) -> Self::ReturnType<O>;

    async fn decrypt(&self, input: O) -> Self::ReturnType<I>;
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
impl Encryption<Secret<String>, Secret<String>> for EncryptionScheme {
    type ReturnType<T> = error_stack::Result<T, KmsError>;

    async fn encrypt(&self, input: Secret<String>) -> Self::ReturnType<Secret<String>> {
        match self {
            #[cfg(feature = "aws_kms")]
            Self::AwsKms { client } => client.encrypt(input.peek()).await.map(Into::into),
            Self::None(no_encryption) => Ok(no_encryption.encrypt(input.peek().clone()).into()),
        }
    }

    async fn decrypt(&self, input: Secret<String>) -> Self::ReturnType<Secret<String>> {
        match self {
            #[cfg(feature = "aws_kms")]
            Self::AwsKms { client } => client.decrypt(input.peek()).await.map(Into::into),
            Self::None(no_encryption) => Ok(no_encryption.decrypt(input.peek().clone()).into()),
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

/// A wrapper around a KMS value that can be decrypted.
#[derive(Clone, Debug, Default, serde::Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct KmsValue(Secret<String>);

impl common_utils::ext_traits::ConfigExt for KmsValue {
    fn is_empty_after_trim(&self) -> bool {
        self.0.peek().is_empty_after_trim()
    }
}

pub trait EncryptionState {}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Decrypted {}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Encrypted {}

impl EncryptionState for Decrypted {}
impl EncryptionState for Encrypted {}

#[derive(Debug, Clone, Default)]
pub struct Decryptable<T, S: EncryptionState> {
    pub inner: T,
    marker: PhantomData<S>,
}

impl<T: Clone, S: EncryptionState> Decryptable<T, S> {
    pub fn into_inner(&self) -> T {
        self.inner.clone()
    }
}

impl<'de, T: Deserialize<'de>, S: EncryptionState> Deserialize<'de> for Decryptable<T, S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = Deserialize::deserialize(deserializer)?;
        Ok(Self {
            inner: val,
            marker: PhantomData,
        })
    }
}

impl<T> Decryptable<T, Encrypted> {
    pub fn decrypt(mut self, decryptor_fn: impl FnOnce(T) -> T) -> Decryptable<T, Decrypted> {
        self.inner = decryptor_fn(self.inner);
        Decryptable {
            inner: self.inner,
            marker: PhantomData,
        }
    }
}

#[async_trait::async_trait]
pub trait Decryption
where
    Self: Sized,
{
    async fn decrypt(
        value: Decryptable<Self, Encrypted>,
        kms_client: &EncryptionScheme,
    ) -> error_stack::Result<Decryptable<Self, Decrypted>, KmsError>;
}
