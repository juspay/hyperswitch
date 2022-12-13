use error_stack::IntoReport;
use masking::ExposeInterface;

use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{
        MerchantConnectorAccount, MerchantConnectorAccountNew, MerchantConnectorAccountUpdate,
    },
};

#[async_trait::async_trait]
pub trait MerchantConnectorAccountInterface {
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
impl MerchantConnectorAccountInterface for super::Store {
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        MerchantConnectorAccount::find_by_merchant_id_connector(&conn, merchant_id, connector)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
            &conn,
            merchant_id,
            merchant_connector_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn insert_merchant_connector_account(
        &self,
        t: MerchantConnectorAccountNew,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        t.insert_diesel(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_merchant_connector_account_by_merchant_id_list(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        MerchantConnectorAccount::find_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_merchant_connector_account(
        &self,
        this: MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdate,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        this.update(&conn, merchant_connector_account)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        MerchantConnectorAccount::delete_by_merchant_id_merchant_connector_id(
            &conn,
            merchant_id,
            merchant_connector_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for MockDb {
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let accounts = self.merchant_connector_accounts.lock().await;
        let account = accounts
            .iter()
            .find(|account| {
                account.merchant_id == merchant_id && account.connector_name == connector
            })
            .cloned()
            .unwrap();
        Ok(account)
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        _merchant_id: &str,
        _merchant_connector_id: &i32,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        todo!()
    }

    #[allow(clippy::panic)]
    async fn insert_merchant_connector_account(
        &self,
        t: MerchantConnectorAccountNew,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        let account = MerchantConnectorAccount {
            id: accounts.len() as i32,
            merchant_id: t.merchant_id.unwrap_or_default(),
            connector_name: t.connector_name.unwrap_or_default(),
            connector_account_details: t.connector_account_details.unwrap_or_default().expose(),
            test_mode: t.test_mode,
            disabled: t.disabled,
            merchant_connector_id: t.merchant_connector_id.unwrap_or_default(),
            payment_methods_enabled: t.payment_methods_enabled,
            connector_type: t
                .connector_type
                .unwrap_or(crate::types::storage::enums::ConnectorType::FinOperations),
        };
        accounts.push(account.clone());
        Ok(account)
    }

    async fn find_merchant_connector_account_by_merchant_id_list(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, errors::StorageError> {
        todo!()
    }

    async fn update_merchant_connector_account(
        &self,
        _this: MerchantConnectorAccount,
        _merchant_connector_account: MerchantConnectorAccountUpdate,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        todo!()
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        _merchant_id: &str,
        _merchant_connector_id: &i32,
    ) -> CustomResult<bool, errors::StorageError> {
        todo!()
    }
}
