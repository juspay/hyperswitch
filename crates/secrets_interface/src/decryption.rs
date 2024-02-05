//! Module containing trait for raw secret retrieval

use common_utils::errors::CustomResult;

use crate::{
    type_state::{RawSecret, SecretStateContainer, SecuredSecret},
    {SecretManagementInterface, SecretsManagementError},
};

/// Trait defining the interface for retrieving a raw secret value, given a secured value
#[async_trait::async_trait]
pub trait SecretsHandler
where
    Self: Sized,
{
    /// Retrieve the raw value and transitions its type to `Decrypted`
    async fn decrypt(
        value: SecretStateContainer<Self, SecuredSecret>,
        kms_client: Box<dyn SecretManagementInterface>,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError>;
}
