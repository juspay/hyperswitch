//! Trait implementations for gcp kms client

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_interfaces::{
    encryption_interface::{EncryptionError, EncryptionManagementInterface},
    secrets_interface::{SecretManagementInterface, SecretsManagementError},
};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::gcp_kms::core::GcpKmsClient;

#[async_trait::async_trait]
impl EncryptionManagementInterface for GcpKmsClient {
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
impl SecretManagementInterface for GcpKmsClient {
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
