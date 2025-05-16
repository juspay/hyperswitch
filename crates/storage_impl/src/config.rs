use common_utils::{id_type, DbConnectionParams};
use masking::Secret;
use crate::CustomResult;
pub use diesel_models::configs::*;
use crate::errors;
use hyperswitch_domain_models::configs::ConfigInterface;
use crate::redis::cache::CacheKind;
use crate::MockDb;
use crate::redis::cache;
use crate::connection;
use crate::redis::cache::CONFIG_CACHE;
use error_stack::report;
use router_env::{instrument, tracing};
use crate::DatabaseStore;
use crate::kv_router_store::KVRouterStore;
use crate::RouterStore;

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

impl DbConnectionParams for Database {
    fn get_username(&self) -> &str {
        &self.username
    }
    fn get_password(&self) -> Secret<String> {
        self.password.clone()
    }
    fn get_host(&self) -> &str {
        &self.host
    }
    fn get_port(&self) -> u16 {
        self.port
    }
    fn get_dbname(&self) -> &str {
        &self.dbname
    }
}

pub trait TenantConfig: Send + Sync {
    fn get_tenant_id(&self) -> &id_type::TenantId;
    fn get_schema(&self) -> &str;
    fn get_accounts_schema(&self) -> &str;
    fn get_redis_key_prefix(&self) -> &str;
    fn get_clickhouse_database(&self) -> &str;
}

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

#[async_trait::async_trait]
impl<T: DatabaseStore> ConfigInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_config(
        &self,
        config: ConfigNew,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let inserted = config
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        cache::redact_from_redis_and_publish(
            self,
            [CacheKind::Config((&inserted.key).into())],
        )
        .await?;

        Ok(inserted)
    }

    #[instrument(skip_all)]
    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        Config::update_by_key(&conn, key, config_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //update in DB and remove in redis and cache
    #[instrument(skip_all)]
    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError> {
        cache::publish_and_redact(self, CacheKind::Config(key.into()), || {
            self.update_config_in_database(key, config_update)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        Config::find_by_key(&conn, key)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //check in cache, then redis then finally DB, and on the way back populate redis and cache
    #[instrument(skip_all)]
    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        let find_config_by_key_from_db = || async {
            let conn = connection::pg_connection_write(self).await?;
            Config::find_by_key(&conn, key)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };
        cache::get_or_populate_in_memory(self, key, find_config_by_key_from_db, &CONFIG_CACHE).await
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        // If the config is not found it will be cached with the default value.
        default_config: Option<String>,
    ) -> CustomResult<Config, errors::StorageError> {
        let find_else_unwrap_or = || async {
            let conn = connection::pg_connection_write(self).await?;
            match Config::find_by_key(&conn, key)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            {
                Ok(a) => Ok(a),
                Err(err) => {
                    if err.current_context().is_db_not_found() {
                        default_config
                            .map(|c| {
                                ConfigNew {
                                    key: key.to_string(),
                                    config: c,
                                }
                                .into()
                            })
                            .ok_or(err)
                    } else {
                        Err(err)
                    }
                }
            }
        };

        cache::get_or_populate_in_memory(self, key, find_else_unwrap_or, &CONFIG_CACHE).await
    }

    #[instrument(skip_all)]
    async fn delete_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let deleted = Config::delete_by_key(&conn, key)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        cache::redact_from_redis_and_publish(
            self,
            [CacheKind::Config((&deleted.key).into())],
        )
        .await?;

        Ok(deleted)
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ConfigInterface for KVRouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_config(
        &self,
        config: ConfigNew,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let inserted = config
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        cache::redact_from_redis_and_publish(
            &self.router_store,
            [CacheKind::Config((&inserted.key).into())],
        )
        .await?;

        Ok(inserted)
    }

    #[instrument(skip_all)]
    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        Config::update_by_key(&conn, key, config_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //update in DB and remove in redis and cache
    #[instrument(skip_all)]
    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError> {
        cache::publish_and_redact(self, CacheKind::Config(key.into()), || {
            self.update_config_in_database(key, config_update)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        Config::find_by_key(&conn, key)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //check in cache, then redis then finally DB, and on the way back populate redis and cache
    #[instrument(skip_all)]
    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        let find_config_by_key_from_db = || async {
            let conn = connection::pg_connection_write(self).await?;
            Config::find_by_key(&conn, key)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };
        cache::get_or_populate_in_memory(self, key, find_config_by_key_from_db, &CONFIG_CACHE).await
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        // If the config is not found it will be cached with the default value.
        default_config: Option<String>,
    ) -> CustomResult<Config, errors::StorageError> {
        let find_else_unwrap_or = || async {
            let conn = connection::pg_connection_write(self).await?;
            match Config::find_by_key(&conn, key)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            {
                Ok(a) => Ok(a),
                Err(err) => {
                    if err.current_context().is_db_not_found() {
                        default_config
                            .map(|c| {
                                ConfigNew {
                                    key: key.to_string(),
                                    config: c,
                                }
                                .into()
                            })
                            .ok_or(err)
                    } else {
                        Err(err)
                    }
                }
            }
        };

        cache::get_or_populate_in_memory(self, key, find_else_unwrap_or, &CONFIG_CACHE).await
    }

    #[instrument(skip_all)]
    async fn delete_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let deleted = Config::delete_by_key(&conn, key)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        cache::redact_from_redis_and_publish(
            &self.router_store,
            [CacheKind::Config((&deleted.key).into())],
        )
        .await?;

        Ok(deleted)
    }
}

#[async_trait::async_trait]
impl ConfigInterface for MockDb {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn insert_config(
        &self,
        config: ConfigNew,
    ) -> CustomResult<Config, errors::StorageError> {
        let mut configs = self.configs.lock().await;

        let config_new = Config {
            key: config.key,
            config: config.config,
        };
        configs.push(config_new.clone());
        Ok(config_new)
    }

    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError> {
        self.update_config_by_key(key, config_update).await
    }

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError> {
        let result = self
            .configs
            .lock()
            .await
            .iter_mut()
            .find(|c| c.key == key)
            .ok_or_else(|| {
                errors::StorageError::ValueNotFound("cannot find config to update".to_string())
                    .into()
            })
            .map(|c| {
                let config_updated =
                    ConfigUpdateInternal::from(config_update).create_config(c.clone());
                *c = config_updated.clone();
                config_updated
            });

        result
    }

    async fn delete_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        let mut configs = self.configs.lock().await;
        let result = configs
            .iter()
            .position(|c| c.key == key)
            .map(|index| configs.remove(index))
            .ok_or_else(|| {
                errors::StorageError::ValueNotFound("cannot find config to delete".to_string())
                    .into()
            });

        result
    }

    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        let configs = self.configs.lock().await;
        let config = configs.iter().find(|c| c.key == key).cloned();

        config.ok_or_else(|| {
            errors::StorageError::ValueNotFound("cannot find config".to_string()).into()
        })
    }

    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        _default_config: Option<String>,
    ) -> CustomResult<Config, errors::StorageError> {
        self.find_config_by_key(key).await
    }

    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<Config, errors::StorageError> {
        self.find_config_by_key(key).await
    }
}
