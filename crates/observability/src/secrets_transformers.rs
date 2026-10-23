//! Resolving secret values at boot.
//!
//! In release builds the values in `observability.toml` may be KMS handles rather than the secrets
//! themselves. This module performs the one-time transition from `Settings<SecuredSecret>` to
//! `Settings<RawSecret>`, so that everything downstream is statically known to hold resolved
//! values. Mirrors `drainer::secrets_transformers`.

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::chat_service::{slack::SlackConfig, xyne::XyneConfig};
use futures::{future::try_join_all, TryFutureExt};
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};
use hyperswitch_masking::PeekInterface;

use crate::settings::{
    evaluator::AlertSettings, AuthSettings, ChatDestination, ChatSettings, Database, Settings,
};

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

/// Resolve independent destination credentials concurrently, retaining destination error context.
#[async_trait::async_trait]
impl SecretsHandler for ChatSettings {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let secured = value.get_inner();
        let destinations = try_join_all(secured.destinations.iter().map(
            |(id, destination)| async move {
                let resolved = match destination {
                    ChatDestination::Xyne(config) => ChatDestination::Xyne(XyneConfig {
                        app_jwt: secret_management_client
                            .get_secret(config.app_jwt.clone())
                            .await
                            .attach_printable_lazy(|| {
                                format!("Failed to decrypt chat destination `{id}`")
                            })?,
                        ..config.clone()
                    }),
                    ChatDestination::Slack(config) => ChatDestination::Slack(SlackConfig {
                        bot_token: secret_management_client
                            .get_secret(config.bot_token.clone())
                            .await
                            .attach_printable_lazy(|| {
                                format!("Failed to decrypt chat destination `{id}`")
                            })?,
                        ..config.clone()
                    }),
                    ChatDestination::Log => ChatDestination::Log,
                };
                Ok::<_, error_stack::Report<SecretsManagementError>>((id.clone(), resolved))
            },
        ))
        .await?
        .into_iter()
        .collect();

        let max_upload_bytes = secured.max_upload_bytes;
        Ok(value.transition_state(|_chat| Self {
            destinations,
            max_upload_bytes,
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for AlertSettings {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        if value.get_inner().grafana.api_token.peek().is_empty() {
            return Ok(value.transition_state(|alert| alert));
        }
        let api_token = secret_management_client
            .get_secret(value.get_inner().grafana.api_token.clone())
            .await?;
        Ok(value.transition_state(|mut alert| {
            alert.grafana.api_token = api_token;
            alert
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for Database {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let secured_database = value.get_inner();
        let raw_password = secret_management_client
            .get_secret(secured_database.password.clone())
            .await?;

        Ok(value.transition_state(|database| Self {
            password: raw_password,
            ..database
        }))
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
    let (auth, chat, alert, database) = futures::try_join!(
        AuthSettings::convert_to_raw_secret(conf.auth, secret_management_client)
            .map_err(|error| error.attach_printable("Failed to decrypt auth internal api key")),
        ChatSettings::convert_to_raw_secret(conf.chat, secret_management_client).map_err(|error| {
            error.attach_printable("Failed to decrypt a chat destination credential")
        }),
        AlertSettings::convert_to_raw_secret(conf.alert, secret_management_client)
            .map_err(|error| error.attach_printable("Failed to decrypt the alert API token")),
        Database::convert_to_raw_secret(conf.database, secret_management_client)
            .map_err(|error| error.attach_printable("Failed to decrypt the database password")),
    )
    .expect("Failed to decrypt observability credentials");

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
    alert
        .get_inner()
        .validate(chat.get_inner())
        .expect("Decrypted alert configuration is unusable");

    Settings {
        server: conf.server,
        log: conf.log,
        auth,
        router: conf.router,
        monitoring: conf.monitoring,
        secrets_management: conf.secrets_management,
        proxy: conf.proxy,
        chat,
        alert,
        email: conf.email,
        database,
    }
}

#[cfg(test)]
mod tests {
    use hyperswitch_masking::Secret;
    use tokio::{
        sync::Barrier,
        time::{timeout, Duration},
    };

    use super::*;

    struct ConcurrentSecrets {
        barrier: Barrier,
    }

    #[async_trait::async_trait]
    impl SecretManagementInterface for ConcurrentSecrets {
        async fn get_secret(
            &self,
            input: Secret<String>,
        ) -> CustomResult<Secret<String>, SecretsManagementError> {
            self.barrier.wait().await;
            Ok(input)
        }
    }

    #[tokio::test]
    async fn independent_boot_and_destination_credentials_are_resolved_concurrently() {
        let conf: Settings<SecuredSecret> = serde_json::from_value(serde_json::json!({
            "auth": { "internal_api_key": "test_internal_key" },
            "chat": { "destinations": {
                "xyne": { "type": "xyne", "app_jwt": "xyne-handle", "channel": "C123" },
                "slack": { "type": "slack", "bot_token": "slack-handle", "channel": "C456" }
            }},
            "alert": { "grafana": {
                "base_url": "http://127.0.0.1:1",
                "api_token": "grafana-handle"
            } },
            "database": { "password": "database-handle" }
        }))
        .expect("valid secured configuration");
        let client = ConcurrentSecrets {
            barrier: Barrier::new(5),
        };
        let raw = timeout(Duration::from_secs(2), fetch_raw_secrets(conf, &client))
            .await
            .expect("all five independent lookups must overlap before any can complete");
        assert_eq!(
            raw.auth.get_inner().internal_api_key.peek(),
            "test_internal_key"
        );
        assert_eq!(
            raw.alert.get_inner().grafana.api_token.peek(),
            "grafana-handle"
        );
        assert_eq!(raw.database.get_inner().password.peek(), "database-handle");
        match raw.chat.get_inner().destinations.get("xyne") {
            Some(ChatDestination::Xyne(config)) => assert_eq!(config.app_jwt.peek(), "xyne-handle"),
            _ => panic!("resolved Xyne destination is missing"),
        }
        match raw.chat.get_inner().destinations.get("slack") {
            Some(ChatDestination::Slack(config)) => {
                assert_eq!(config.bot_token.peek(), "slack-handle")
            }
            _ => panic!("resolved Slack destination is missing"),
        }
    }
}
