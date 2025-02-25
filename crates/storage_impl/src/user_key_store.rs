
use common_utils::{types::keymanager, errors::CustomResult};
use router_env::{instrument, tracing};
use error_stack::{report, ResultExt};
use masking::Secret;
use hyperswitch_domain_models::behaviour::{Conversion, ReverseConversion};
use sample::{domain::user_key_store as domain, user_key_store::UserKeyStoreInterface};

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> UserKeyStoreInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_user_key_store(
        &self,
        state: &keymanager::KeyManagerState,
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
            .convert(state, key, keymanager::Identifier::User(user_id))
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn get_user_key_store_by_user_id(
        &self,
        state: &keymanager::KeyManagerState,
        user_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        diesel_models::user_key_store::UserKeyStore::find_by_user_id(&conn, user_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(state, key, keymanager::Identifier::User(user_id.to_owned()))
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn get_all_user_key_store(
        &self,
        state: &keymanager::KeyManagerState,
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
                .convert(state, key, keymanager::Identifier::User(user_id))
                .await
                .change_context(errors::StorageError::DecryptionError)
        }))
        .await
    }
}