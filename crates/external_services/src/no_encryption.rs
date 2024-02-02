//!
//! No encryption functionalities
//!

use common_utils::errors::CustomResult;
use encryption_interface::encryption_management::{EncryptionError, EncryptionManagementInterface};
use masking::{ExposeInterface, Secret};
use secrets_interface::secrets_management::{SecretManagementInterface, SecretsManagementError};

/// No encryption type
#[derive(Debug, Clone)]
pub struct NoEncryption;

impl NoEncryption {
    /// Encryption functionality
    pub fn encrypt(&self, data: String) -> String {
        data
    }

    /// Decryption functionality
    pub fn decrypt(&self, data: String) -> String {
        data
    }
}

#[async_trait::async_trait]
impl EncryptionManagementInterface for NoEncryption {
    async fn encrypt(&self, input: String) -> CustomResult<String, EncryptionError> {
        Ok(self.encrypt(input))
    }

    async fn decrypt(&self, input: String) -> CustomResult<String, EncryptionError> {
        Ok(self.decrypt(input))
    }
}

#[async_trait::async_trait]
impl SecretManagementInterface for NoEncryption {
    async fn store_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<String, SecretsManagementError> {
        Ok(self.encrypt(input.expose()))
    }

    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<String, SecretsManagementError> {
        Ok(self.decrypt(input.expose()))
    }
}
