use common_utils::ext_traits::AsyncExt;
use diesel_models::configs::ConfigUpdateInternal;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
use storage_impl::redis::{
    cache::{CacheKind, CONFIG_CACHE},
    kv_store::RedisConnInterface,
    pub_sub::PubSubInterface,
};

use super::{cache, MockDb, Store};
use crate::{
    connection, consts,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait ConfigInterface {
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        // If the config is not found it will be created with the default value.
        default_config: Option<String>,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn delete_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError>;
}

#[async_trait::async_trait]
impl ConfigInterface for Store {
    #[instrument(skip_all)]
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let inserted = config
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        self.get_redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .publish(consts::PUB_SUB_CHANNEL, CacheKind::Config((&inserted.key).into()))
            .await
            .map_err(Into::<errors::StorageError>::into)?;

        Ok(inserted)
    }

    #[instrument(skip_all)]
    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Config::update_by_key(&conn, key, config_update)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //update in DB and remove in redis and cache
    #[instrument(skip_all)]
    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        cache::publish_and_redact(self, CacheKind::Config(key.into()), || {
            self.update_config_in_database(key, config_update)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Config::find_by_key(&conn, key)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //check in cache, then redis then finally DB, and on the way back populate redis and cache
    #[instrument(skip_all)]
    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let find_config_by_key_from_db = || async {
            let conn = connection::pg_connection_write(self).await?;
            storage::Config::find_by_key(&conn, key)
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
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let find_else_unwrap_or = || async {
            let conn = connection::pg_connection_write(self).await?;
            match storage::Config::find_by_key(&conn, key)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            {
                Ok(a) => Ok(a),
                Err(err) => {
                    if err.current_context().is_db_not_found() {
                        default_config.map(|c| {
                            storage::ConfigNew{
                                key : key.to_string(),
                                config: c
                            }.into()
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
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let deleted = storage::Config::delete_by_key(&conn, key)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        self.get_redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .publish(consts::PUB_SUB_CHANNEL, CacheKind::Config(key.into()))
            .await
            .map_err(Into::<errors::StorageError>::into)?;

        Ok(deleted)
    }
}

#[async_trait::async_trait]
impl ConfigInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let mut configs = self.configs.lock().await;

        let config_new = storage::Config {
            id: i32::try_from(configs.len()).change_context(errors::StorageError::MockDbError)?,
            key: config.key,
            config: config.config,
        };
        configs.push(config_new.clone());
        Ok(config_new)
    }

    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.update_config_by_key(key, config_update).await
    }

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
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
    ) -> CustomResult<storage::Config, errors::StorageError> {
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
    ) -> CustomResult<storage::Config, errors::StorageError> {
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
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.find_config_by_key(key).await
    }

    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.find_config_by_key(key).await
    }
}
