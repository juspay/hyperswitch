use common_utils::errors::CustomResult;
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
        secret_management_client: Box<dyn SecretManagementInterface>,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let db_password = secret_management_client
            .get_secret(value.get_inner().password.clone())
            .await?;

        Ok(value.transition_state(|db| Self {
            username: db.username,
            password: db_password,
            host: db.host,
            port: db.port,
            dbname: db.dbname,
            pool_size: db.pool_size,
            connection_timeout: db.connection_timeout,
        }))
    }
}

/// # Panics
///
/// Will panic even if kms decryption fails for at least one field
#[allow(clippy::unwrap_used)]
pub async fn kms_decryption(
    conf: Settings<SecuredSecret>,
    secret_management_client: Box<dyn SecretManagementInterface>,
) -> Settings<RawSecret> {
    #[allow(clippy::expect_used)]
    let database = Database::convert_to_raw_secret(conf.master_database, secret_management_client)
        .await
        .expect("Failed to decrypt database password");

    Settings {
        server: conf.server,
        master_database: database,
        redis: conf.redis,
        log: conf.log,
        drainer: conf.drainer,
        encryption_management: conf.encryption_management,
        secrets_management: conf.secrets_management,
    }
}
