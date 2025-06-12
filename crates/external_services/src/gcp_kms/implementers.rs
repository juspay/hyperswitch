//! Trait implementations for Google Cloud KMS client

use async_trait::async_trait;
use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_interfaces::{
    encryption_interface::{EncryptionManagementInterface, EncryptionError},
    secrets_interface::{SecretManagementInterface, SecretsManagementError},
};
use masking::{Secret, ExposeInterface};

use super::core::{GcpKmsClient, GcpKmsError};

#[async_trait]
impl EncryptionManagementInterface for GcpKmsClient {
    async fn encrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError> {
        let encrypted = self.encrypt(input).await.change_context(EncryptionError::EncryptionFailed)?;
        Ok(encrypted.into_bytes())
    }

    async fn decrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError> {
        let decrypted = self.decrypt(input).await.change_context(EncryptionError::DecryptionFailed)?;
        Ok(decrypted.into_bytes())
    }
}

#[async_trait]
impl SecretManagementInterface for GcpKmsClient {
    async fn get_secret(&self, secret_input: Secret<String>) -> CustomResult<Secret<String>, SecretsManagementError> {
        let decrypted = self.decrypt(secret_input.expose().as_bytes())
            .await
            .change_context(SecretsManagementError::FetchSecretFailed)?;
        Ok(Secret::new(decrypted))
    }
}

impl From<GcpKmsError> for EncryptionError {
    fn from(error: GcpKmsError) -> Self {
        match error {
            GcpKmsError::ClientInitializationFailed | GcpKmsError::InitializationFailed => EncryptionError::EncryptionFailed,
            GcpKmsError::EncryptionFailed => EncryptionError::EncryptionFailed,
            GcpKmsError::DecryptionFailed => EncryptionError::DecryptionFailed,
            _ => EncryptionError::EncryptionFailed,
        }
    }
}

impl From<GcpKmsError> for SecretsManagementError {
    fn from(error: GcpKmsError) -> Self {
        match error {
            GcpKmsError::ClientInitializationFailed | GcpKmsError::InitializationFailed => SecretsManagementError::ClientCreationFailed,
            _ => SecretsManagementError::FetchSecretFailed,
        }
    }
}
