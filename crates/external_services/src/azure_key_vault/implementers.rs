//! Trait implementations for azure key vault

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_interfaces::{
    encryption_interface::{EncryptionError, EncryptionManagementInterface},
    secrets_interface::{SecretManagementInterface, SecretsManagementError},
};
use masking::{PeekInterface, Secret};

use crate::azure_key_vault::core::AzureKeyVaultClient;

#[async_trait::async_trait]
impl EncryptionManagementInterface for AzureKeyVaultClient {
    async fn encrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError> {
        self.encrypt(input)
            .await
            .change_context(EncryptionError::EncryptionFailed)
            .map(|val| val.into_bytes())
    }

    async fn decrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError> {
        self.decrypt(input)
            .await
            .change_context(EncryptionError::DecryptionFailed)
            .map(|val| val.into_bytes())
    }
}

#[async_trait::async_trait]
impl SecretManagementInterface for AzureKeyVaultClient {
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError> {
        self.decrypt(input.peek())
            .await
            .change_context(SecretsManagementError::FetchSecretFailed)
            .map(Into::into)
    }
}
