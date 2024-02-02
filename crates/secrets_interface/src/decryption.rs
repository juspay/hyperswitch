use common_utils::errors::CustomResult;

use crate::{
    secrets_management::{SecretManagementInterface, SecretsManagementError},
    type_state::{Decryptable, Decrypted, Encrypted},
};

/// Trait defining the interface for decrypting a Decryptable value
#[async_trait::async_trait]
pub trait Decryption
where
    Self: Sized,
{
    /// Decrypt the given value and transitions its type to `Decrypted`
    async fn decrypt(
        value: Decryptable<Self, Encrypted>,
        kms_client: Box<dyn SecretManagementInterface>,
    ) -> CustomResult<Decryptable<Self, Decrypted>, SecretsManagementError>;
}
