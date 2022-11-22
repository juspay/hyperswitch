use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{MerchantAccount, MerchantAccountNew, MerchantAccountUpdate},
};

#[async_trait::async_trait]
pub trait IMerchantAccount {
    async fn insert_merchant(
        &self,
        merchant_account: MerchantAccountNew,
    ) -> CustomResult<MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError>;

    async fn update_merchant(
        &self,
        this: MerchantAccount,
        merchant_account: MerchantAccountUpdate,
    ) -> CustomResult<MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_api_key(
        &self,
        api_key: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError>;

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl IMerchantAccount for Store {
    async fn insert_merchant(
        &self,
        merchant_account: MerchantAccountNew,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        merchant_account.insert(&conn).await
    }

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        MerchantAccount::find_by_merchant_id(&conn, merchant_id).await
    }

    async fn update_merchant(
        &self,
        this: MerchantAccount,
        merchant_account: MerchantAccountUpdate,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        this.update(&conn, merchant_account).await
    }

    async fn find_merchant_account_by_api_key(
        &self,
        api_key: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        MerchantAccount::find_by_api_key(&conn, api_key).await
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        MerchantAccount::find_by_publishable_key(&conn, publishable_key).await
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        MerchantAccount::delete_by_merchant_id(&conn, merchant_id).await
    }
}
