//!
//! No encryption functionalities
//!

use common_utils::errors::CustomResult;
use encryption_interface::{EncryptionError, EncryptionManagementInterface};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, Secret};
use secrets_interface::{SecretManagementInterface, SecretsManagementError};

/// No encryption type
#[derive(Debug, Clone)]
pub struct NoEncryption;

impl NoEncryption {
    /// Encryption functionality
    pub fn encrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, NoEncryptionError> {
        String::from_utf8(data.as_ref().into())
            .into_report()
            .change_context(NoEncryptionError::Utf8DecodingFailed)
    }

    /// Decryption functionality
    pub fn decrypt(&self, data: impl AsRef<[u8]>) -> CustomResult<String, NoEncryptionError> {
        String::from_utf8(data.as_ref().into())
            .into_report()
            .change_context(NoEncryptionError::Utf8DecodingFailed)
    }
}

#[async_trait::async_trait]
impl EncryptionManagementInterface for NoEncryption {
    async fn encrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError> {
        self.encrypt(input)
            .change_context(EncryptionError::EncryptionFailed)
            .map(|val| val.into_bytes())
    }

    async fn decrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError> {
        self.decrypt(input)
            .change_context(EncryptionError::DecryptionFailed)
            .map(|val| val.into_bytes())
    }
}

#[async_trait::async_trait]
impl SecretManagementInterface for NoEncryption {
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError> {
        self.decrypt(input.expose())
            .map(Into::into)
            .change_context(SecretsManagementError::DecryptionFailed)
    }
}

/// Errors that could occur during KMS operations.
#[derive(Debug, thiserror::Error)]
pub enum NoEncryptionError {
    /// An error occurred UTF-8 decoding AWS KMS decrypted output.
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,
}
