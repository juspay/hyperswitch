use diesel_models::configs as storage;
use diesel_models::configs::ConfigUpdateInternal;
use error_stack::report;
use hyperswitch_domain_models::configs::{ConfigInterface, ConfigNew, ConfigUpdate, Config as DomainConfig};
use router_env::{instrument, tracing};

use crate::{
    connection,
    errors::StorageError,
    kv_router_store,
    redis::{
        cache,
        cache::{CacheKind, CONFIG_CACHE},
    },
    transformers::ForeignFrom,
    CustomResult, DatabaseStore, MockDb, RouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> ConfigInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_config(
        &self,
        config: ConfigNew,
    ) -> CustomResult<DomainConfig, Self::Error> {
        self.router_store.insert_config(config).await
    }

    #[instrument(skip_all)]
    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<DomainConfig, StorageError> {
        self.router_store.update_config_in_database(key, config_update).await
    }

    //update in DB and remove in redis and cache
    #[instrument(skip_all)]
    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<DomainConfig, StorageError> {
        self.router_store.update_config_by_key(key, config_update).await
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<DomainConfig, StorageError> {
        self.router_store.find_config_by_key_from_db(key).await
    }

    //check in cache, then redis then finally DB, and on the way back populate redis and cache
    #[instrument(skip_all)]
    async fn find_config_by_key(&self, key: &str) -> CustomResult<DomainConfig, StorageError> {
        self.router_store.find_config_by_key(key).await
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        // If the config is not found it will be cached with the default value.
        default_config: Option<String>,
    ) -> CustomResult<DomainConfig, StorageError> {
        self.router_store.find_config_by_key_unwrap_or(key, default_config).await
    }

    #[instrument(skip_all)]
    async fn delete_config_by_key(&self, key: &str) -> CustomResult<DomainConfig, StorageError> {
        self.router_store.delete_config_by_key(key).await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ConfigInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_config(
        &self,
        config: ConfigNew,
    ) -> CustomResult<DomainConfig, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let diesel_config = storage::ConfigNew::foreign_from(config);
        let inserted = diesel_config
            .insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;

        cache::redact_from_redis_and_publish(self, [CacheKind::Config((&inserted.key).into())])
            .await?;

        Ok(DomainConfig::foreign_from(inserted))
    }

    #[instrument(skip_all)]
    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<DomainConfig, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let diesel_update = storage::ConfigUpdate::foreign_from(config_update);
        let updated = storage::Config::update_by_key(&conn, key, diesel_update)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainConfig::foreign_from(updated))
    }

    //update in DB and remove in redis and cache
    #[instrument(skip_all)]
    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<DomainConfig, StorageError> {
        cache::publish_and_redact(self, CacheKind::Config(key.into()), || {
            self.update_config_in_database(key, config_update)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<DomainConfig, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let result = storage::Config::find_by_key(&conn, key)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;
        Ok(DomainConfig::foreign_from(result))
    }

    //check in cache, then redis then finally DB, and on the way back populate redis and cache
    #[instrument(skip_all)]
    async fn find_config_by_key(&self, key: &str) -> CustomResult<DomainConfig, StorageError> {
        let find_config_by_key_from_db = || async {
            let conn = connection::pg_connection_write(self).await?;
            storage::Config::find_by_key(&conn, key)
                .await
                .map_err(|error| report!(StorageError::from(error)))
        };
        let result = cache::get_or_populate_in_memory(self, key, find_config_by_key_from_db, &CONFIG_CACHE).await?;
        Ok(DomainConfig::foreign_from(result))
    }

    #[instrument(skip_all)]
    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        // If the config is not found it will be cached with the default value.
        default_config: Option<String>,
    ) -> CustomResult<DomainConfig, StorageError> {
        let find_else_unwrap_or = || async {
            let conn = connection::pg_connection_write(self).await?;
            match storage::Config::find_by_key(&conn, key)
                .await
                .map_err(|error| report!(StorageError::from(error)))
            {
                Ok(a) => Ok(a),
                Err(err) => {
                    if err.current_context().is_db_not_found() {
                        default_config
                            .map(|c| {
                                storage::Config {
                                    key: key.to_string(),
                                    config: c,
                                }
                            })
                            .ok_or(err)
                    } else {
                        Err(err)
                    }
                }
            }
        };

        let result = cache::get_or_populate_in_memory(self, key, find_else_unwrap_or, &CONFIG_CACHE).await?;
        Ok(DomainConfig::foreign_from(result))
    }

    #[instrument(skip_all)]
    async fn delete_config_by_key(&self, key: &str) -> CustomResult<DomainConfig, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let deleted = storage::Config::delete_by_key(&conn, key)
            .await
            .map_err(|error| report!(StorageError::from(error)))?;

        cache::redact_from_redis_and_publish(self, [CacheKind::Config((&deleted.key).into())])
            .await?;

        Ok(DomainConfig::foreign_from(deleted))
    }
}

#[async_trait::async_trait]
impl ConfigInterface for MockDb {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_config(
        &self,
        config: ConfigNew,
    ) -> CustomResult<DomainConfig, Self::Error> {
        let mut configs = self.configs.lock().await;

        let config_new = DomainConfig {
            key: config.key,
            config: config.config,
        };
        configs.push(storage::Config::foreign_from(config_new.clone()));
        Ok(config_new)
    }

    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<DomainConfig, Self::Error> {
        self.update_config_by_key(key, config_update).await
    }

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<DomainConfig, Self::Error> {
        let result = self
            .configs
            .lock()
            .await
            .iter_mut()
            .find(|c| c.key == key)
            .ok_or_else(|| {
                StorageError::ValueNotFound("cannot find config to update".to_string()).into()
            })
            .map(|c| {
                let config_updated =
                    ConfigUpdateInternal::foreign_from(config_update).create_config(c.clone());
                *c = config_updated.clone();
                DomainConfig::foreign_from(config_updated)
            });

        result
    }

    async fn delete_config_by_key(&self, key: &str) -> CustomResult<DomainConfig, Self::Error> {
        let mut configs = self.configs.lock().await;
        let result = configs
            .iter()
            .position(|c| c.key == key)
            .map(|index| {
                let deleted = configs.remove(index);
                DomainConfig::foreign_from(deleted)
            })
            .ok_or_else(|| {
                StorageError::ValueNotFound("cannot find config to delete".to_string()).into()
            });

        result
    }

    async fn find_config_by_key(&self, key: &str) -> CustomResult<DomainConfig, Self::Error> {
        let configs = self.configs.lock().await;
        let config = configs.iter().find(|c| c.key == key).cloned();

        config
            .map(|c| DomainConfig::foreign_from(c))
            .ok_or_else(|| StorageError::ValueNotFound("cannot find config".to_string()).into())
    }

    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        _default_config: Option<String>,
    ) -> CustomResult<DomainConfig, Self::Error> {
        self.find_config_by_key(key).await
    }

    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<DomainConfig, Self::Error> {
        self.find_config_by_key(key).await
    }
}
