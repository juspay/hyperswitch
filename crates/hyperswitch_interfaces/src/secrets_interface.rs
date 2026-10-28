//! Secrets management interface

pub mod secret_handler;

pub mod secret_state;

use common_utils::errors::CustomResult;
use hyperswitch_masking::{PeekInterface, Secret};

/// Trait defining the interface for managing application secrets
#[async_trait::async_trait]
pub trait SecretManagementInterface: Send + Sync {
    #[allow(missing_docs)]
    fn supports_kms_decryption(&self) -> bool {
        false
    }

    #[allow(missing_docs)]
    async fn get_kms_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError> {
        self.get_secret(input).await
    }

    #[allow(missing_docs)]
    async fn decrypt_optional_secret(
        &self,
        encrypted_secret: Option<Secret<String>>,
        field: &'static str,
    ) -> CustomResult<Option<Secret<String>>, SecretsManagementError> {
        let Some(encrypted_secret) = encrypted_secret else {
            return Ok(None);
        };
        if !self.supports_kms_decryption() || encrypted_secret.peek().is_empty() {
            return Err(
                error_stack::Report::new(SecretsManagementError::FetchSecretFailed)
                    .attach_printable(field)
                    .attach_printable("A nonempty ciphertext and a KMS backend are required"),
            );
        }
        let decrypted_secret = self
            .get_kms_secret(encrypted_secret)
            .await
            .map_err(|error| error.attach_printable(field))?;
        if decrypted_secret.peek().is_empty() {
            return Err(
                error_stack::Report::new(SecretsManagementError::FetchSecretFailed)
                    .attach_printable(field)
                    .attach_printable("KMS returned an empty credential"),
            );
        }
        Ok(Some(decrypted_secret))
    }

    /*
    /// Given an input, encrypt/store the secret
    async fn store_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<String, SecretsManagementError>;
    */

    /// Given an input, decrypt/retrieve the secret
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError>;
}

/// Errors that may occur during secret management
#[derive(Debug, thiserror::Error)]
pub enum SecretsManagementError {
    /// An error occurred when retrieving raw data.
    #[error("Failed to fetch the raw data")]
    FetchSecretFailed,

    /// Failed while creating kms client
    #[error("Failed while creating a secrets management client")]
    ClientCreationFailed,
}
