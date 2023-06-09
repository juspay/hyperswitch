use error_stack::{IntoReport, ResultExt};

use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    services::Store,
    types::domain::{
        behaviour::{Conversion, ReverseConversion},
        merchant_key_store,
    },
};

#[async_trait::async_trait]
pub trait MerchantKeyStoreInterface {
    async fn insert_merchant_key_store(
        &self,
        merchant_key_store: merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<merchant_key_store::MerchantKeyStore, errors::StorageError>;

    async fn get_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<merchant_key_store::MerchantKeyStore, errors::StorageError>;
}

#[async_trait::async_trait]
impl MerchantKeyStoreInterface for Store {
    async fn insert_merchant_key_store(
        &self,
        merchant_key_store: merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<merchant_key_store::MerchantKeyStore, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let merchant_id = merchant_key_store.merchant_id.clone();
        merchant_key_store
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()?
            .convert(self, &merchant_id)
            .await
            .change_context(errors::StorageError::DecryptionError)
    }
    async fn get_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<merchant_key_store::MerchantKeyStore, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage_models::merchant_key_store::MerchantKeyStore::find_by_merchant_id(
            &conn,
            merchant_id,
        )
        .await
        .map_err(Into::into)
        .into_report()?
        .convert(self, merchant_id)
        .await
        .change_context(errors::StorageError::DecryptionError)
    }
}

#[async_trait::async_trait]
impl MerchantKeyStoreInterface for MockDb {
    async fn insert_merchant_key_store(
        &self,
        _merchant_key_store: merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<merchant_key_store::MerchantKeyStore, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError.into())
    }
    async fn get_merchant_key_store_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<merchant_key_store::MerchantKeyStore, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError.into())
    }
}
