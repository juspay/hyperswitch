use super::MockDb;
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
    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError>;
}

#[async_trait::async_trait]
impl ConfigInterface for super::Store {
    async fn insert_config(
        &self,
        config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        config.insert(&conn).await
    }

    async fn find_config_by_key(
        &self,
        key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::Config::find_by_key(&conn, key).await
    }

    async fn update_config_by_key(
        &self,
        key: &str,
        config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        storage::Config::update_by_key(&conn, key, config_update).await
    }
}

#[async_trait::async_trait]
impl ConfigInterface for MockDb {
    async fn insert_config(
        &self,
        _config: storage::ConfigNew,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        todo!()
    }

    async fn find_config_by_key(
        &self,
        _key: &str,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        todo!()
    }

    async fn update_config_by_key(
        &self,
        _key: &str,
        _config_update: storage::ConfigUpdate,
    ) -> CustomResult<storage::Config, errors::StorageError> {
        todo!()
    }
}
