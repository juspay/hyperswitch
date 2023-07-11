use error_stack::IntoReport;
use storage_models::configs::ConfigUpdateInternal;

use super::{cache, MockDb, Store};
use crate::{
    cache::{CacheKind, CONFIG_CACHE},
    connection, consts,
    core::errors::{self, CustomResult},
    services::PubSubInterface,
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

    async fn find_config_by_key_cached(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn update_config_cached(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn delete_config_by_key(&self, key: &str) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl ConfigInterface for Store {
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        config.insert(&conn).await.map_err(Into::into).into_report()
    }

    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Config::find_by_key(&conn, key)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_config_by_key(
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
    async fn update_config_cached(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        cache::publish_and_redact(self, CacheKind::Config(key.into()), || {
            self.update_config_by_key(key, config_update)
        })
        .await
    }

    async fn find_config_by_key_cached(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        cache::get_or_populate_in_memory(self, key, || self.find_config_by_key(key), &CONFIG_CACHE)
            .await
    }

    async fn delete_config_by_key(&self, key: &str) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let deleted = storage::Config::delete_by_key(&conn, key)
            .await
            .map_err(Into::into)
            .into_report()?;

        self.redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .publish(consts::PUB_SUB_CHANNEL, CacheKind::Config(key.into()))
            .await
            .map_err(Into::<errors::StorageError>::into)?;

        Ok(deleted)
    }
}

#[async_trait::async_trait]
impl ConfigInterface for MockDb {
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let mut configs = self.configs.lock().await;

        let config_new = storage::Config {
            #[allow(clippy::as_conversions)]
            id: configs.len() as i32,
            key: config.key,
            config: config.config,
        };
        configs.push(config_new.clone());
        Ok(config_new)
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
    async fn update_config_cached(
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

    async fn find_config_by_key_cached(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let configs = self.configs.lock().await;
        let config = configs.iter().find(|c| c.key == key).cloned();

        config.ok_or_else(|| {
            errors::StorageError::ValueNotFound("cannot find config".to_string()).into()
        })
    }
}
