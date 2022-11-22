use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::Store,
    types::storage::{
        MerchantConnectorAccount, MerchantConnectorAccountNew, MerchantConnectorAccountUpdate,
    },
};

#[async_trait::async_trait]
pub trait IMerchantConnectorAccount {
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError>;

    async fn insert_merchant_connector_account(
        &self,
        t: MerchantConnectorAccountNew,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError>;

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError>;

    async fn find_merchant_connector_account_by_merchant_id_list(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, errors::StorageError>;

    async fn update_merchant_connector_account(
        &self,
        this: MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdate,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError>;

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl IMerchantConnectorAccount for Store {
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        MerchantConnectorAccount::find_by_merchant_id_connector(&conn, merchant_id, connector).await
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
            &conn,
            merchant_id,
            merchant_connector_id,
        )
        .await
    }

    async fn insert_merchant_connector_account(
        &self,
        t: MerchantConnectorAccountNew,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        t.insert(&conn).await
    }

    async fn find_merchant_connector_account_by_merchant_id_list(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        MerchantConnectorAccount::find_by_merchant_id(&conn, merchant_id).await
    }

    async fn update_merchant_connector_account(
        &self,
        this: MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdate,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        this.update(&conn, merchant_connector_account).await
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool.conn).await;
        MerchantConnectorAccount::delete_by_merchant_id_merchant_connector_id(
            &conn,
            merchant_id,
            merchant_connector_id,
        )
        .await
    }
}
