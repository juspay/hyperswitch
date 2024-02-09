//! Module containing trait for raw secret retrieval

use common_utils::errors::CustomResult;

use crate::secrets_interface::{
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

/// Trait defining the interface for retrieving a raw secret value, given a secured value
#[async_trait::async_trait]
pub trait SecretsHandler
where
    Self: Sized,
{
    /// Construct `Self` with raw secret value and transitions its type from `SecuredSecret` to `RawSecret`
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        kms_client: Box<dyn SecretManagementInterface>,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError>;
}
