use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store as domain,
    merchant_key_store::MerchantKeyStoreInterface,
};
use masking::Secret;
use router_env::{instrument, tracing};

#[cfg(feature = "accounts_cache")]
use crate::redis::{
    cache,
    cache::{CacheKind, ACCOUNTS_CACHE},
};
use crate::{
    kv_router_store,
    utils::{pg_accounts_connection_read, pg_accounts_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore, StorageError,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> MerchantKeyStoreInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_merchant_key_store(
        &self,
        merchant_key_store: domain::MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, Self::Error> {
        self.router_store
            .insert_merchant_key_store(merchant_key_store, key)
            .await
    }

    #[instrument(skip_all)]
    async fn get_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, Self::Error> {
        self.router_store
            .get_merchant_key_store_by_merchant_id(merchant_id, key)
            .await
    }

    #[instrument(skip_all)]
    async fn delete_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error> {
        self.router_store
            .delete_merchant_key_store_by_merchant_id(merchant_id)
            .await
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn list_multiple_key_stores(
        &self,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, Self::Error> {
        self.router_store
            .list_multiple_key_stores(merchant_ids, key)
            .await
    }

    async fn get_all_key_stores(
        &self,
        key: &Secret<Vec<u8>>,
        from: u32,
        to: u32,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, Self::Error> {
        self.router_store.get_all_key_stores(key, from, to).await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> MerchantKeyStoreInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_merchant_key_store(
        &self,
        merchant_key_store: domain::MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;
        let merchant_id = merchant_key_store.merchant_id.clone();
        merchant_key_store
            .construct_new()
            .await
            .change_context(Self::Error::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(Self::Error::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                merchant_id.into(),
            )
            .await
            .change_context(Self::Error::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn get_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, Self::Error> {
        let fetch_func = || async {
            let conn = pg_accounts_connection_read(self).await?;

            diesel_models::merchant_key_store::MerchantKeyStore::find_by_merchant_id(
                &conn,
                merchant_id,
            )
            .await
            .map_err(|error| report!(Self::Error::from(error)))
        };
        let state = self
            .get_keymanager_state()
            .attach_printable("Missing KeyManagerState")?;

        #[cfg(not(feature = "accounts_cache"))]
        {
            fetch_func()
                .await?
                .convert(state, key, merchant_id.clone().into())
                .await
                .change_context(Self::Error::DecryptionError)
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
            .change_context(Self::Error::DecryptionError)
        }
    }

    #[instrument(skip_all)]
    async fn delete_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error> {
        let delete_func = || async {
            let conn = pg_accounts_connection_write(self).await?;
            diesel_models::merchant_key_store::MerchantKeyStore::delete_by_merchant_id(
                &conn,
                merchant_id,
            )
            .await
            .map_err(|error| report!(Self::Error::from(error)))
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
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, Self::Error> {
        let fetch_func = || async {
            let conn = pg_accounts_connection_read(self).await?;

            diesel_models::merchant_key_store::MerchantKeyStore::list_multiple_key_stores(
                &conn,
                merchant_ids,
            )
            .await
            .map_err(|error| report!(Self::Error::from(error)))
        };

        futures::future::try_join_all(fetch_func().await?.into_iter().map(|key_store| async {
            let merchant_id = key_store.merchant_id.clone();
            key_store
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key,
                    merchant_id.into(),
                )
                .await
                .change_context(Self::Error::DecryptionError)
        }))
        .await
    }

    async fn get_all_key_stores(
        &self,
        key: &Secret<Vec<u8>>,
        from: u32,
        to: u32,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        let stores = diesel_models::merchant_key_store::MerchantKeyStore::list_all_key_stores(
            &conn, from, to,
        )
        .await
        .map_err(|err| report!(Self::Error::from(err)))?;

        futures::future::try_join_all(stores.into_iter().map(|key_store| async {
            let merchant_id = key_store.merchant_id.clone();
            key_store
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key,
                    merchant_id.into(),
                )
                .await
                .change_context(Self::Error::DecryptionError)
        }))
        .await
    }
}

#[async_trait::async_trait]
impl MerchantKeyStoreInterface for MockDb {
    type Error = StorageError;
    async fn insert_merchant_key_store(
        &self,
        merchant_key_store: domain::MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, Self::Error> {
        let mut locked_merchant_key_store = self.merchant_key_store.lock().await;

        if locked_merchant_key_store
            .iter()
            .any(|merchant_key| merchant_key.merchant_id == merchant_key_store.merchant_id)
        {
            Err(StorageError::DuplicateValue {
                entity: "merchant_key_store",
                key: Some(merchant_key_store.merchant_id.get_string_repr().to_owned()),
            })?;
        }

        let merchant_key = Conversion::convert(merchant_key_store)
            .await
            .change_context(StorageError::MockDbError)?;
        locked_merchant_key_store.push(merchant_key.clone());
        let merchant_id = merchant_key.merchant_id.clone();
        merchant_key
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                merchant_id.into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn get_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, StorageError> {
        self.merchant_key_store
            .lock()
            .await
            .iter()
            .find(|merchant_key| merchant_key.merchant_id == *merchant_id)
            .cloned()
            .ok_or(StorageError::ValueNotFound(String::from(
                "merchant_key_store",
            )))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn delete_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        let mut merchant_key_stores = self.merchant_key_store.lock().await;
        let index = merchant_key_stores
            .iter()
            .position(|mks| mks.merchant_id == *merchant_id)
            .ok_or(StorageError::ValueNotFound(format!(
                "No merchant key store found for merchant_id = {merchant_id:?}",
            )))?;
        merchant_key_stores.remove(index);
        Ok(true)
    }

    #[cfg(feature = "olap")]
    async fn list_multiple_key_stores(
        &self,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, StorageError> {
        let merchant_key_stores = self.merchant_key_store.lock().await;
        futures::future::try_join_all(
            merchant_key_stores
                .iter()
                .filter(|merchant_key| merchant_ids.contains(&merchant_key.merchant_id))
                .map(|merchant_key| async {
                    merchant_key
                        .to_owned()
                        .convert(
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
                            key,
                            merchant_key.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(StorageError::DecryptionError)
                }),
        )
        .await
    }
    async fn get_all_key_stores(
        &self,
        key: &Secret<Vec<u8>>,
        _from: u32,
        _to: u32,
    ) -> CustomResult<Vec<domain::MerchantKeyStore>, StorageError> {
        let merchant_key_stores = self.merchant_key_store.lock().await;

        futures::future::try_join_all(merchant_key_stores.iter().map(|merchant_key| async {
            merchant_key
                .to_owned()
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key,
                    merchant_key.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)
        }))
        .await
    }
}
