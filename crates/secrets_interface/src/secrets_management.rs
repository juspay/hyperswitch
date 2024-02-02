use common_utils::errors::CustomResult;
use masking::Secret;

/// Trait defining the interface for managing application secrets
#[async_trait::async_trait]
pub trait SecretManagementInterface: Send + Sync {
    /// Given an input, encrypt/store the secret
    async fn store_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<String, SecretsManagementError>;

    /// Given an input, decrypt/retrieve the secret
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<String, SecretsManagementError>;
}

/// Errors that may occur during secret management
#[derive(Debug, thiserror::Error)]
pub enum SecretsManagementError {
    /// An error occurred when decrypting input data.
    #[error("Failed to decrypt input data")]
    DecryptionFailed,

    /// An error occurred when encrypting input data.
    #[error("Failed to encrypt input data")]
    EncryptionFailed,

    /// Failed while creating kms client
    #[error("Failed while creating a new client")]
    ClientCreationFailed,
}
