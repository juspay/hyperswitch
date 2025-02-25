use common_utils::{errors::CustomResult, types::keymanager::KeyManagerState};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store as domain,
};
use masking::Secret;
use router_env::{instrument, tracing};
use sample::merchant_key_store::MerchantKeyStoreInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> MerchantKeyStoreInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_merchant_key_store(
        &self,
        state: &KeyManagerState,
        merchant_key_store: domain::MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let merchant_id = merchant_key_store.merchant_id.clone();
        merchant_key_store
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(state, key, merchant_id.into())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn get_merchant_key_store_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        let fetch_func = || async {
            let conn = connection::pg_connection_read(self).await?;

            diesel_models::merchant_key_store::MerchantKeyStore::find_by_merchant_id(
                &conn,
                merchant_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            fetch_func()
                .await?
                .convert(state, key, merchant_id.clone().into())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(feature = "accounts_cache")]
        {
            let key_store_cache_key =
                format!("merchant_key_store_{}", merchant_id.get_string_repr());
            cache::get_or_populate_in_memory(
                self,
                &key_store_cache_key,
                fetch_func,
                &ACCOUNTS_CACHE,
            )
            .await?
            .convert(state, key, merchant_id.clone().into())
            .await
            .change_context(errors::StorageError::DecryptionError)
        }
    }

    #[instrument(skip_all)]
    async fn delete_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        let delete_func = || async {
            let conn = connection::pg_connection_write(self).await?;
            diesel_models::merchant_key_store::MerchantKeyStore::delete_by_merchant_id(
                &conn,
                merchant_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            delete_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            let key_store_cache_key =
                format!("merchant_key_store_{}", merchant_id.get_string_repr());
            cache::publish_and_redact(
                self,
                CacheKind::Accounts(key_store_cache_key.into()),
                delete_func,
            )
            .await
        }
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn list_multiple_key_stores(
        &self,
        state: &KeyManagerState,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, errors::StorageError> {
        let fetch_func = || async {
            let conn = connection::pg_connection_read(self).await?;

            diesel_models::merchant_key_store::MerchantKeyStore::list_multiple_key_stores(
                &conn,
                merchant_ids,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        futures::future::try_join_all(fetch_func().await?.into_iter().map(|key_store| async {
            let merchant_id = key_store.merchant_id.clone();
            key_store
                .convert(state, key, merchant_id.into())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }))
        .await
    }

    async fn get_all_key_stores(
        &self,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        from: u32,
        to: u32,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let stores = diesel_models::merchant_key_store::MerchantKeyStore::list_all_key_stores(
            &conn, from, to,
        )
        .await
        .map_err(|err| report!(errors::StorageError::from(err)))?;

        futures::future::try_join_all(stores.into_iter().map(|key_store| async {
            let merchant_id = key_store.merchant_id.clone();
            key_store
                .convert(state, key, merchant_id.into())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }))
        .await
    }
}

use sample::MasterKeyInterface;
use masking::PeekInterface;

// TODO(jarnura): MOve the below to respective place
impl<T: DatabaseStore> MasterKeyInterface for RouterStore<T> {
    fn get_master_key(&self) -> &[u8] {
        self.master_key().peek()
    }
}
