use common_utils::{errors::CustomResult, types::keymanager};
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

    async fn get_all_user_key_store(
        &self,
        key: &Secret<Vec<u8>>,
        from: u32,
        limit: u32,
    ) -> CustomResult<Vec<domain::UserKeyStore>, errors::StorageError>;
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
        let user_id = user_key_store.user_id.clone();
        user_key_store
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                keymanager::Identifier::User(user_id),
            )
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
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                keymanager::Identifier::User(user_id.to_owned()),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn get_all_user_key_store(
        &self,
        key: &Secret<Vec<u8>>,
        from: u32,
        limit: u32,
    ) -> CustomResult<Vec<domain::UserKeyStore>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        let key_stores = diesel_models::user_key_store::UserKeyStore::get_all_user_key_stores(
            &conn, from, limit,
        )
        .await
        .map_err(|err| report!(errors::StorageError::from(err)))?;
        futures::future::try_join_all(key_stores.into_iter().map(|key_store| async {
            let user_id = key_store.user_id.clone();
            key_store
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key,
                    keymanager::Identifier::User(user_id),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }))
        .await
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
        let user_id = user_key_store.user_id.clone();
        user_key_store
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                keymanager::Identifier::User(user_id),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn get_all_user_key_store(
        &self,
        key: &Secret<Vec<u8>>,
        _from: u32,
        _limit: u32,
    ) -> CustomResult<Vec<domain::UserKeyStore>, errors::StorageError> {
        let user_key_store = self.user_key_store.lock().await;

        futures::future::try_join_all(user_key_store.iter().map(|user_key| async {
            let user_id = user_key.user_id.clone();
            user_key
                .to_owned()
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key,
                    keymanager::Identifier::User(user_id),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }))
        .await
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
                "No user_key_store is found for user_id={user_id}",
            )))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                keymanager::Identifier::User(user_id.to_owned()),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
}
