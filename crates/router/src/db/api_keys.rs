use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait ApiKeyInterface {
    async fn insert_api_key(
        &self,
        api_key: storage::ApiKeyNew,
    ) -> CustomResult<storage::ApiKey, errors::StorageError>;

    async fn update_api_key(
        &self,
        key_id: String,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError>;

    async fn revoke_api_key(&self, key_id: &str) -> CustomResult<bool, errors::StorageError>;

    async fn find_api_key_by_key_id_optional(
        &self,
        key_id: &str,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError>;

    async fn find_api_key_by_hash_optional(
        &self,
        hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError>;

    async fn list_api_keys_by_merchant_id(
        &self,
        merchant_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError>;
}

#[async_trait::async_trait]
impl ApiKeyInterface for Store {
    async fn insert_api_key(
        &self,
        api_key: storage::ApiKeyNew,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        api_key
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_api_key(
        &self,
        key_id: String,
        api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::ApiKey::update_by_key_id(&conn, key_id, api_key)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn revoke_api_key(&self, key_id: &str) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::ApiKey::revoke_by_key_id(&conn, key_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_api_key_by_key_id_optional(
        &self,
        key_id: &str,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::ApiKey::find_optional_by_key_id(&conn, key_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_api_key_by_hash_optional(
        &self,
        hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::ApiKey::find_optional_by_hashed_api_key(&conn, hashed_api_key)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn list_api_keys_by_merchant_id(
        &self,
        merchant_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::ApiKey::find_by_merchant_id(&conn, merchant_id, limit, offset)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl ApiKeyInterface for MockDb {
    async fn insert_api_key(
        &self,
        _api_key: storage::ApiKeyNew,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_api_key(
        &self,
        _key_id: String,
        _api_key: storage::ApiKeyUpdate,
    ) -> CustomResult<storage::ApiKey, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn revoke_api_key(&self, _key_id: &str) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_api_key_by_key_id_optional(
        &self,
        _key_id: &str,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_api_key_by_hash_optional(
        &self,
        _hashed_api_key: storage::HashedApiKey,
    ) -> CustomResult<Option<storage::ApiKey>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn list_api_keys_by_merchant_id(
        &self,
        _merchant_id: &str,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> CustomResult<Vec<storage::ApiKey>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
