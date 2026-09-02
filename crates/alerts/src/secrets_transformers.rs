//! Resolving secret values at boot.
//!
//! In release builds the values in `alerts.toml` may be KMS handles rather than the secrets
//! themselves. This module performs the one-time transition from `Settings<SecuredSecret>` to
//! `Settings<RawSecret>`, so that everything downstream is statically known to hold resolved
//! values. Mirrors `drainer::secrets_transformers`.

use std::collections::HashMap;

use common_utils::errors::CustomResult;
use external_services::chat_service::{slack::SlackConfig, xyne::XyneConfig};
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::settings::{AuthSettings, ChatDestination, ChatSettings, Settings};

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

/// Chat credentials are resolved one destination at a time.
///
/// Sequentially rather than concurrently: this runs once, at boot, against a handful of
/// destinations, and a failure that names the destination it came from is worth more here than the
/// milliseconds a join would save.
#[async_trait::async_trait]
impl SecretsHandler for ChatSettings {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let secured = value.get_inner();
        let mut destinations = HashMap::with_capacity(secured.destinations.len());

        for (id, destination) in &secured.destinations {
            let resolved = match destination {
                ChatDestination::Xyne(config) => ChatDestination::Xyne(XyneConfig {
                    app_jwt: secret_management_client
                        .get_secret(config.app_jwt.clone())
                        .await?,
                    ..config.clone()
                }),
                ChatDestination::Slack(config) => ChatDestination::Slack(SlackConfig {
                    bot_token: secret_management_client
                        .get_secret(config.bot_token.clone())
                        .await?,
                    ..config.clone()
                }),
                // Holds no credential, so there is nothing to resolve.
                ChatDestination::Log => ChatDestination::Log,
            };

            destinations.insert(id.clone(), resolved);
        }

        Ok(value.transition_state(|_chat| Self { destinations }))
    }
}

/// Resolve every secret in the configuration.
///
/// # Panics
///
/// Panics if any secret fails to resolve, or if a resolved secret is unusable. This is
/// deliberate: a service that cannot read its own API key must not start, and there is no
/// partially-configured state worth serving traffic from.
pub async fn fetch_raw_secrets(
    conf: Settings<SecuredSecret>,
    secret_management_client: &dyn SecretManagementInterface,
) -> Settings<RawSecret> {
    #[allow(clippy::expect_used)]
    let auth = AuthSettings::convert_to_raw_secret(conf.auth, secret_management_client)
        .await
        .expect("Failed to decrypt auth internal api key");

    // Re-validate *after* decryption. The check in `main` ran against the `SecuredSecret` value,
    // which under a KMS backend is a handle, not the key — and a perfectly well-formed handle can
    // resolve to an empty string. Without this, the service would start with an empty configured
    // key, and an empty `X-Internal-Api-Key` header would compare equal to it: every request
    // authenticated. The boot-time check must therefore happen on both sides of the transition.
    #[allow(clippy::expect_used)]
    auth.get_inner()
        .validate()
        .expect("Decrypted auth internal api key is unusable");

    #[allow(clippy::expect_used)]
    let chat = ChatSettings::convert_to_raw_secret(conf.chat, secret_management_client)
        .await
        .expect("Failed to decrypt a chat destination credential");

    Settings {
        server: conf.server,
        log: conf.log,
        auth,
        secrets_management: conf.secrets_management,
        proxy: conf.proxy,
        chat,
        email: conf.email,
    }
}
