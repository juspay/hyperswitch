use common_utils::{errors::CustomResult, ext_traits::AsyncExt};
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};

use crate::settings::{Database, Settings};

#[async_trait::async_trait]
impl SecretsHandler for Database {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let secured_db_config = value.get_inner();
        let raw_db_password = secret_management_client
            .get_secret(secured_db_config.password.clone())
            .await?;

        Ok(value.transition_state(|db| Self {
            password: raw_db_password,
            ..db
        }))
    }
}

/// # Panics
///
/// Will panic even if fetching raw secret fails for at least one config value
pub async fn fetch_raw_secrets(
    conf: Settings<SecuredSecret>,
    secret_management_client: &dyn SecretManagementInterface,
) -> CustomResult<Settings<RawSecret>, SecretsManagementError> {
    let redis = conf.redis;
    let primary_password = redis
        .primary_password
        .async_map(|secret| async move {
            secret_management_client
                .get_secret(secret)
                .await
                .map_err(|error| error.attach_printable("redis.primary_password"))
        })
        .await
        .transpose()?;
    let redis = redis_interface::RedisSettings {
        primary_password,
        ..redis
    };
    #[allow(clippy::expect_used)]
    let database = Database::convert_to_raw_secret(conf.master_database, secret_management_client)
        .await
        .expect("Failed to decrypt database password");

    Ok(Settings {
        server: conf.server,
        master_database: database,
        redis,
        log: conf.log,
        drainer: conf.drainer,
        encryption_management: conf.encryption_management,
        secrets_management: conf.secrets_management,
        multitenancy: conf.multitenancy,
    })
}
