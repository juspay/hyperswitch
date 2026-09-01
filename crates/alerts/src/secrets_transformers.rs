//! Resolving secret values at boot.
//!
//! In release builds the values in `alerts.toml` may be KMS handles rather than the secrets
//! themselves. This module performs the one-time transition from `Settings<SecuredSecret>` to
//! `Settings<RawSecret>`, so that everything downstream is statically known to hold resolved
//! values. Mirrors `drainer::secrets_transformers`.

use common_utils::errors::CustomResult;
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::settings::{AuthSettings, Settings};

#[async_trait::async_trait]
impl SecretsHandler for AuthSettings {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let secured_auth_config = value.get_inner();
        let raw_internal_api_key = secret_management_client
            .get_secret(secured_auth_config.internal_api_key.clone())
            .await?;

        Ok(value.transition_state(|_auth| Self {
            internal_api_key: raw_internal_api_key,
        }))
    }
}

/// Resolve every secret in the configuration.
///
/// # Panics
///
/// Panics if any secret fails to resolve. This is deliberate: a service that cannot read its own
/// API key must not start, and there is no partially-configured state worth serving traffic from.
pub async fn fetch_raw_secrets(
    conf: Settings<SecuredSecret>,
    secret_management_client: &dyn SecretManagementInterface,
) -> Settings<RawSecret> {
    #[allow(clippy::expect_used)]
    let auth = AuthSettings::convert_to_raw_secret(conf.auth, secret_management_client)
        .await
        .expect("Failed to decrypt auth internal api key");

    Settings {
        server: conf.server,
        log: conf.log,
        auth,
        secrets_management: conf.secrets_management,
    }
}
