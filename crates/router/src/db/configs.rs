use common_utils::ext_traits::AsyncExt;
use diesel_models::configs::ConfigUpdateInternal;
use error_stack::{IntoReport, ResultExt};
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

    async fn delete_config_by_key(&self, key: &str) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl ConfigInterface for Store {
    #[instrument(skip_all)]
        /// Asynchronously inserts a new configuration into the storage, returning the inserted configuration
    /// if successful, or a `StorageError` if an error occurs.
    ///
    /// # Arguments
    ///
    /// * `config` - The new configuration to be inserted into the storage.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the inserted configuration if successful, or a `StorageError` if an
    /// error occurs.
    ///
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        config.insert(&conn).await.map_err(Into::into).into_report()
    }

        /// Asynchronously updates the configuration in the database with the specified key and configuration update,
    /// returning a Result containing the updated storage::Config or a StorageError if the update fails.
    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Config::update_by_key(&conn, key, config_update)
            .await
            .map_err(Into::into)
            .into_report()
    }

    //update in DB and remove in redis and cache
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

        /// Asynchronously finds a configuration by the given key from the database.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to search for in the database
    ///
    /// # Returns
    ///
    /// * `CustomResult<storage::Config, errors::StorageError>` - A result containing the found configuration or a storage error
    ///
    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Config::find_by_key(&conn, key)
            .await
            .map_err(Into::into)
            .into_report()
    }

    //check in cache, then redis then finally DB, and on the way back populate redis and cache
    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let find_config_by_key_from_db = || async {
            let conn = connection::pg_connection_write(self).await?;
            storage::Config::find_by_key(&conn, key)
                .await
                .map_err(Into::into)
                .into_report()
        };
        cache::get_or_populate_in_memory(self, key, find_config_by_key_from_db, &CONFIG_CACHE).await
    }

        /// Asynchronously finds a configuration by its key and unwraps it if found, or creates a new configuration with the provided default value if not found. Returns a `CustomResult` with the found or newly created configuration, or a `StorageError` if an error occurs during the process.
    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        // If the config is not found it will be created with the default value.
        default_config: Option<String>,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let find_else_unwrap_or = || async {
            let conn = connection::pg_connection_write(self).await?;
            match storage::Config::find_by_key(&conn, key)
                .await
                .map_err(Into::<errors::StorageError>::into)
                .into_report()
            {
                Ok(a) => Ok(a),
                Err(err) => {
                    if err.current_context().is_db_not_found() {
                        default_config
                            .ok_or(err)
                            .async_and_then(|c| async {
                                storage::ConfigNew {
                                    key: key.to_string(),
                                    config: c,
                                }
                                .insert(&conn)
                                .await
                                .map_err(Into::into)
                                .into_report()
                            })
                            .await
                    } else {
                        Err(err)
                    }
                }
            }
        };

        cache::get_or_populate_in_memory(self, key, find_else_unwrap_or, &CONFIG_CACHE).await
    }

        /// Asynchronously deletes a configuration by its key from the storage. Returns a CustomResult containing a boolean indicating whether the configuration was deleted successfully or not, and an errors::StorageError if an error occurs during the deletion process. 
    async fn delete_config_by_key(&self, key: &str) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let deleted = storage::Config::delete_by_key(&conn, key)
            .await
            .map_err(Into::into)
            .into_report()?;

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
        /// Asynchronously inserts a new configuration into the storage. It takes a `ConfigNew` object as input and returns a `CustomResult` containing the inserted `Config` object or a `StorageError` if an error occurs during the insertion process.
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let mut configs = self.configs.lock().await;

        let config_new = storage::Config {
            id: configs
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            key: config.key,
            config: config.config,
        };
        configs.push(config_new.clone());
        Ok(config_new)
    }

        /// Asynchronously updates the configuration in the database for the given key with the provided configuration update.
    ///
    /// # Arguments
    ///
    /// * `key` - A reference to a string representing the key of the configuration to be updated.
    /// * `config_update` - A storage::ConfigUpdate struct representing the update to be applied to the configuration.
    ///
    /// # Returns
    ///
    /// A CustomResult containing the updated storage::Config if the update was successful, otherwise an errors::StorageError.
    ///
    async fn update_config_in_database(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.update_config_by_key(key, config_update).await
    }
        /// Asynchronously updates the configuration with the given key using the provided configuration update. 
    /// Returns a `CustomResult` containing the updated configuration if successful, otherwise returns a `StorageError`.
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

        /// Asynchronously deletes a configuration by its key from the storage. 
    /// Returns a CustomResult indicating whether the deletion was successful or an error occurred.
    async fn delete_config_by_key(&self, key: &str) -> CustomResult<bool, errors::StorageError> {
        let mut configs = self.configs.lock().await;
        let result = configs
            .iter()
            .position(|c| c.key == key)
            .map(|index| {
                configs.remove(index);
                true
            })
            .ok_or_else(|| {
                errors::StorageError::ValueNotFound("cannot find config to delete".to_string())
                    .into()
            });

        result
    }

        /// Asynchronously finds a configuration by key in the storage. Returns a Result containing the found configuration or a StorageError if the configuration is not found.
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

        /// Asynchronously finds a configuration by key and unwraps it if found, otherwise returns the default configuration provided.
    ///
    /// # Arguments
    ///
    /// * `key` - The key used to search for the configuration.
    /// * `_default_config` - An optional default configuration to return if the configuration is not found.
    ///
    /// # Returns
    ///
    /// * `CustomResult<storage::Config, errors::StorageError>` - A custom result type containing the found configuration or a storage error.
    ///
    async fn find_config_by_key_unwrap_or(
        &self,
        key: &str,
        _default_config: Option<String>,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.find_config_by_key(key).await
    }


        /// Asynchronously finds a configuration by its key from the database and returns a `CustomResult` containing the found `storage::Config` or an `errors::StorageError` in case of failure.
    async fn find_config_by_key_from_db(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        self.find_config_by_key(key).await
    }
}
