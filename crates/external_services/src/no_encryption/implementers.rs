//! Trait implementations for No encryption client

use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use hyperswitch_interfaces::{
    encryption_interface::{EncryptionError, EncryptionManagementInterface},
    secrets_interface::{SecretManagementInterface, SecretsManagementError},
};
use masking::{ExposeInterface, Secret};

use crate::no_encryption::core::NoEncryption;

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
            .change_context(SecretsManagementError::FetchSecretFailed)
            .attach_printable("Failed to convert decrypted value to UTF-8")
    }
}
