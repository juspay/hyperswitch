use error_stack::{report, IntoReport, ResultExt};

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
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

    async fn find_config_by_key_cached(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError>;

    async fn update_config_by_key(
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
        let conn = pg_connection(&self.master_pool).await;
        config.insert(&conn).await.map_err(Into::into).into_report()
    }

    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
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
        let conn = pg_connection(&self.master_pool).await;
        storage::Config::update_by_key(&conn, key, config_update)
            .await
            .map_err(Into::into)
            .into_report()
    }
    async fn find_config_by_key_cached(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let redis = &self.redis_conn;
        let redis_val = redis
            .get_and_deserialize_key::<storage::Config>(key, "Config")
            .await;
        Ok(match redis_val {
            Err(err) => match err.current_context() {
                errors::RedisError::NotFound => {
                    let config = self.find_config_by_key(key).await?;
                    redis
                        .serialize_and_set_key(&config.key, &config)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    config
                }
                err => Err(report!(errors::StorageError::KVError)
                    .attach_printable(format!("Error while fetching config {err}")))?,
            },
            Ok(val) => val,
        })
    }

    async fn delete_config_by_key(&self, key: &str) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::Config::delete_by_key(&conn, key)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl ConfigInterface for MockDb {
    async fn insert_config(
        &self,
        _config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_config_by_key(
        &self,
        _key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_config_by_key(
        &self,
        _key: &str,
        _config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_config_by_key(&self, _key: &str) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_config_by_key_cached(
        &self,
        _key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
