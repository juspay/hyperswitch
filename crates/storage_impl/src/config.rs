use common_utils::ext_traits::ConfigExt;
use external_services::kms::{
    Decryptable, Decrypted, Decryption, Encrypted, Encryption, EncryptionScheme, KmsError,
};
use masking::Secret;

use crate::errors::ApplicationError;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Database {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub queue_strategy: QueueStrategy,
    pub min_idle: Option<u32>,
    pub max_lifetime: Option<u64>,
}

impl Database {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database host must not be empty".into(),
            ))
        })?;

        when(self.dbname.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database name must not be empty".into(),
            ))
        })?;

        when(self.username.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database user username must not be empty".into(),
            ))
        })?;

        when(self.password.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "database user password must not be empty".into(),
            ))
        })
    }
}

#[async_trait::async_trait]
impl Decryption for Database {
    async fn decrypt(
        value: Decryptable<Self, Encrypted>,
        kms_client: &EncryptionScheme,
    ) -> error_stack::Result<Decryptable<Self, Decrypted>, KmsError> {
        let db_password = kms_client.decrypt(value.inner.password.clone()).await?;
        Ok(value.decrypt(|db| Self {
            username: db.username,
            password: db_password,
            host: db.host,
            port: db.port,
            dbname: db.dbname,
            pool_size: db.pool_size,
            connection_timeout: db.connection_timeout,
            queue_strategy: db.queue_strategy,
            min_idle: db.min_idle,
            max_lifetime: db.max_lifetime,
        }))
    }
}
// #[async_trait::async_trait]
// impl KmsDecrypt for Database {
//     type Output = Database;

//     async fn decrypt_inner(
//         mut self,
//         kms_client: &EncryptionScheme,
//     ) -> CustomResult<Self::Output, KmsError> {
//         Ok(Self {
//             host: self.host,
//             port: self.port,
//             dbname: self.dbname,
//             username: self.username,
//             password: self.password.decrypt_inner(kms_client).await?.into(),
//             pool_size: self.pool_size,
//             connection_timeout: self.connection_timeout,
//             queue_strategy: self.queue_strategy,
//             min_idle: self.min_idle,
//             max_lifetime: self.max_lifetime,
//         })
//     }
// }

#[derive(Debug, serde::Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "PascalCase")]
pub enum QueueStrategy {
    #[default]
    Fifo,
    Lifo,
}

impl From<QueueStrategy> for bb8::QueueStrategy {
    fn from(value: QueueStrategy) -> Self {
        match value {
            QueueStrategy::Fifo => Self::Fifo,
            QueueStrategy::Lifo => Self::Lifo,
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: Secret::<String>::default(),
            host: "localhost".into(),
            port: 5432,
            dbname: String::new(),
            pool_size: 5,
            connection_timeout: 10,
            queue_strategy: QueueStrategy::default(),
            min_idle: None,
            max_lifetime: None,
        }
    }
}
