use common_utils::errors::{CustomResult};
use common_utils::pii::REDACTED;
use masking::Secret;
use crate::services::{Store, MockDb};
use crate::cache::Cacheable;
use crate::db::cache::publish_and_redact;
use crate::domain::MerchantAccountUpdate;
use crate::{self as storage, cache, CardInfo, enums, EphemeralKeyNew, EphemeralKey};
use crate::{domain::behaviour::Conversion, connection};
use crate::AddressNew;
use crate::address::AddressUpdateInternal;
use error_stack::{IntoReport, ResultExt};
use crate::merchant_key_store;
use crate::{domain, errors};
use crate::domain::CustomerUpdate;

#[async_trait::async_trait]
pub trait MerchantKeyStoreInterface {
    async fn insert_merchant_key_store(
        &self,
        merchant_key_store: domain::MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError>;

    async fn get_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError>;
}

#[async_trait::async_trait]
impl MerchantKeyStoreInterface for Store {
    async fn insert_merchant_key_store(
        &self,
        merchant_key_store: domain::MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        merchant_key_store
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()?
            .convert(key)
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
    async fn get_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        let fetch_func = || async {
            let conn = connection::pg_connection_read(self).await?;

            merchant_key_store::MerchantKeyStore::find_by_merchant_id(
                &conn,
                merchant_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        };
        #[cfg(not(feature = "accounts_cache"))]
        {
            fetch_func()
                .await?
                .convert(key)
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(feature = "accounts_cache")]
        {
            let key_store_cache_key = format!("merchant_key_store_{}", merchant_id);
            super::cache::get_or_populate_in_memory(
                self,
                &key_store_cache_key,
                fetch_func,
                &ACCOUNTS_CACHE,
            )
            .await?
            .convert(key)
            .await
            .change_context(errors::StorageError::DecryptionError)
        }
    }
}

#[async_trait::async_trait]
impl MerchantKeyStoreInterface for MockDb {
    async fn insert_merchant_key_store(
        &self,
        _merchant_key_store: domain::MerchantKeyStore,
        _key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError.into())
    }
    async fn get_merchant_key_store_by_merchant_id(
        &self,
        _merchant_id: &str,
        _key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::MerchantKeyStore, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError.into())
    }
}
