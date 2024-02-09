//! Encryption related interface and error types

#![warn(missing_docs, missing_debug_implementations)]

use common_utils::errors::CustomResult;

/// Trait defining the interface for encryption management
#[async_trait::async_trait]
pub trait EncryptionManagementInterface: Sync + Send + dyn_clone::DynClone {
    /// Encrypt the given input data
    async fn encrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError>;

    /// Decrypt the given input data
    async fn decrypt(&self, input: &[u8]) -> CustomResult<Vec<u8>, EncryptionError>;
}

dyn_clone::clone_trait_object!(EncryptionManagementInterface);

/// Errors that may occur during above encryption functionalities
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    /// An error occurred when encrypting input data.
    #[error("Failed to encrypt input data")]
    EncryptionFailed,

    /// An error occurred when decrypting input data.
    #[error("Failed to decrypt input data")]
    DecryptionFailed,
}
