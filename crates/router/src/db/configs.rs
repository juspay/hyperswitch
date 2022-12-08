use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{Config, ConfigNew, ConfigUpdate},
};

#[async_trait::async_trait]
pub trait ConfigInterface {
    async fn insert_config(&self, config: ConfigNew) -> CustomResult<Config, errors::StorageError>;
    async fn find_config_by_key(&self, key: &str) -> CustomResult<Config, errors::StorageError>;
    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError>;
}

#[async_trait::async_trait]
impl ConfigInterface for super::Store {
    async fn insert_config(&self, config: ConfigNew) -> CustomResult<Config, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        config.insert(&conn).await
    }

    async fn find_config_by_key(&self, key: &str) -> CustomResult<Config, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        Config::find_by_key(&conn, key).await
    }

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        Config::update_by_key(&conn, key, config_update).await
    }
}

#[async_trait::async_trait]
impl ConfigInterface for MockDb {
    async fn insert_config(
        &self,
        _config: ConfigNew,
    ) -> CustomResult<Config, errors::StorageError> {
        todo!()
    }

    async fn find_config_by_key(&self, _key: &str) -> CustomResult<Config, errors::StorageError> {
        todo!()
    }

    async fn update_config_by_key(
        &self,
        _key: &str,
        _config_update: ConfigUpdate,
    ) -> CustomResult<Config, errors::StorageError> {
        todo!()
    }
}
