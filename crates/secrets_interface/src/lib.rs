//! Secrets management interface

#![warn(missing_docs, missing_debug_implementations)]

pub mod secret_handler;

pub mod secret_state;

use common_utils::errors::CustomResult;
use masking::Secret;

/// Trait defining the interface for managing application secrets
#[async_trait::async_trait]
pub trait SecretManagementInterface: Send + Sync {
    /// Given an input, encrypt/store the secret
    // async fn store_secret(
    //     &self,
    //     input: Secret<String>,
    // ) -> CustomResult<String, SecretsManagementError>;

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
