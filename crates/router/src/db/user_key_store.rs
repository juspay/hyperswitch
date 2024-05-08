use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use masking::Secret;
use router_env::{instrument, tracing};
use storage_impl::MockDb;

use crate::{
    connection,
    core::errors,
    services::Store,
    types::domain::{
        self,
        behaviour::{Conversion, ReverseConversion},
    },
};

#[async_trait::async_trait]
pub trait UserKeyStoreInterface {
    async fn insert_user_key_store(
        &self,
        user_key_store: domain::UserKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError>;

    async fn get_user_key_store_by_user_id(
        &self,
        user_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError>;
}

#[async_trait::async_trait]
impl UserKeyStoreInterface for Store {
    #[instrument(skip_all)]
    async fn insert_user_key_store(
        &self,
        user_key_store: domain::UserKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        user_key_store
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(key)
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn get_user_key_store_by_user_id(
        &self,
        user_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        diesel_models::user_key_store::UserKeyStore::find_by_user_id(&conn, user_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(key)
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
}

#[async_trait::async_trait]
impl UserKeyStoreInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_user_key_store(
        &self,
        user_key_store: domain::UserKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
        let mut locked_user_key_store = self.user_key_store.lock().await;

        if locked_user_key_store
            .iter()
            .any(|user_key| user_key.user_id == user_key_store.user_id)
        {
            Err(errors::StorageError::DuplicateValue {
                entity: "user_key_store",
                key: Some(user_key_store.user_id.clone()),
            })?;
        }

        let user_key_store = Conversion::convert(user_key_store)
            .await
            .change_context(errors::StorageError::MockDbError)?;
        locked_user_key_store.push(user_key_store.clone());

        user_key_store
            .convert(key)
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn get_user_key_store_by_user_id(
        &self,
        user_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
        self.user_key_store
            .lock()
            .await
            .iter()
            .find(|user_key_store| user_key_store.user_id == user_id)
            .cloned()
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "No user_key_store is found for user_id={}",
                user_id
            )))?
            .convert(key)
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
}
