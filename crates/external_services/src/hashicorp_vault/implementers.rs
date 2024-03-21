//! Trait implementations for Hashicorp vault client

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_interfaces::secrets_interface::{
    SecretManagementInterface, SecretsManagementError,
};
use masking::{ExposeInterface, Secret};

use crate::hashicorp_vault::core::{HashiCorpVault, Kv2};

#[async_trait::async_trait]
impl SecretManagementInterface for HashiCorpVault {
    async fn get_secret(
        &self,
        input: Secret<String>,
    ) -> CustomResult<Secret<String>, SecretsManagementError> {
        self.fetch::<Kv2, Secret<String>>(input.expose())
            .await
            .map(|val| val.expose())
            .change_context(SecretsManagementError::FetchSecretFailed)
            .map(Into::into)
    }
}
