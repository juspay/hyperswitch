//! Resolving secret values at boot.

use std::collections::HashMap;

use common_utils::errors::CustomResult;
use external_services::chat_service::{slack::SlackConfig, xyne::XyneConfig};
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::settings::{AuthSettings, ChatDestination, ChatSettings, DatabaseSettings, Settings};

#[async_trait::async_trait]
impl SecretsHandler for DatabaseSettings {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let secured_database_config = value.get_inner();
        let raw_password = secret_management_client
            .get_secret(secured_database_config.password.clone())
            .await?;

        Ok(value.transition_state(|database| Self {
            password: raw_password,
            ..database
        }))
    }
}

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

        let max_upload_bytes = secured.max_upload_bytes;
        Ok(value.transition_state(|_chat| Self {
            destinations,
            max_upload_bytes,
        }))
    }
}

/// Resolve every secret in the configuration.
pub async fn fetch_raw_secrets(
    conf: Settings<SecuredSecret>,
    secret_management_client: &dyn SecretManagementInterface,
) -> Settings<RawSecret> {
    #[allow(clippy::expect_used)]
    let auth = AuthSettings::convert_to_raw_secret(conf.auth, secret_management_client)
        .await
        .expect("Failed to decrypt auth internal api key");

    // Re-validate *after* decryption.
    #[allow(clippy::expect_used)]
    auth.get_inner()
        .validate()
        .expect("Decrypted auth internal api key is unusable");

    #[allow(clippy::expect_used)]
    let chat = ChatSettings::convert_to_raw_secret(conf.chat, secret_management_client)
        .await
        .expect("Failed to decrypt a chat destination credential");

    #[allow(clippy::expect_used)]
    let database = DatabaseSettings::convert_to_raw_secret(conf.database, secret_management_client)
        .await
        .expect("Failed to decrypt the database password");

    // Re-validate after decryption, for the reason given above:
    #[allow(clippy::expect_used)]
    database
        .get_inner()
        .validate()
        .expect("Decrypted database password is unusable");

    Settings {
        server: conf.server,
        log: conf.log,
        auth,
        database,
        secrets_management: conf.secrets_management,
        proxy: conf.proxy,
        chat,
        email: conf.email,
    }
}
