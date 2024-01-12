use external_services::kms::{
    Decryptable, Decrypted, Decryption, Encrypted, Encryption, EncryptionScheme, KmsError,
};

use crate::settings::{Database, Settings};

#[async_trait::async_trait]
impl Decryption for Database {
    async fn decrypt(
        value: Decryptable<Self, Encrypted>,
        kms_client: &EncryptionScheme,
    ) -> error_stack::Result<Decryptable<Self, Decrypted>, KmsError> {
        let db_password = kms_client.decrypt(value.into_inner().password).await?;

        Ok(value.decrypt(|db| Self {
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

pub async fn kms_decryption(
    conf: Settings<Encrypted>,
    kms_client: &EncryptionScheme,
) -> Settings<Decrypted> {
    let database = Database::decrypt(conf.master_database, kms_client)
        .await
        .unwrap();

    Settings {
        master_database: database,
        redis: conf.redis,
        log: conf.log,
        drainer: conf.drainer,
        kms: conf.kms,
    }
}
