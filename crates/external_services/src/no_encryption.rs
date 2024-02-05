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
    pub fn encrypt(&self, data: impl AsRef<[u8]>) -> Vec<u8> {
        data.as_ref().into()
    }

    /// Decryption functionality
    pub fn decrypt(&self, data: impl AsRef<[u8]>) -> Vec<u8> {
        data.as_ref().into()
    }
}

#[async_trait::async_trait]
impl EncryptionManagementInterface for NoEncryption {
    async fn encrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError> {
        Ok(self.encrypt(input))
    }

    async fn decrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError> {
        Ok(self.decrypt(input))
    }
}

#[async_trait::async_trait]
impl SecretManagementInterface for NoEncryption {
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError> {
        String::from_utf8(self.decrypt(input.expose()))
            .map(Into::into)
            .into_report()
            .change_context(SecretsManagementError::DecryptionFailed)
    }
}
