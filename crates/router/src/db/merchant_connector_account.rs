use masking::ExposeInterface;

use super::MockDb;
#[cfg(feature = "diesel")]
use crate::connection::pg_connection;
use crate::{
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

#[cfg(feature = "diesel")]
#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for super::Store {
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
        t.insert_diesel(&conn).await
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

#[cfg(feature = "sqlx")]
#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for super::Sqlx {
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        #[allow(clippy::panic)]
        let val = sqlx::query_as!(
            MerchantConnectorAccount,
            r#"
          SELECT
            "merchant_connector_account"."id",
            "merchant_connector_account"."merchant_id",
            "merchant_connector_account"."connector_name",
            "merchant_connector_account"."connector_account_details",
            "merchant_connector_account"."test_mode",
            "merchant_connector_account"."disabled",
            "merchant_connector_account"."merchant_connector_id",
            "merchant_connector_account"."payment_methods_enabled",
            "merchant_connector_account"."connector_type" "connector_type: _"
          FROM
            "merchant_connector_account"
          WHERE
            (
              (
                "merchant_connector_account"."merchant_id" = $1
              )
              AND (
                "merchant_connector_account"."connector_name" = $2
              )
            )"#,
            merchant_id,
            connector,
        )
        .fetch_one(&self.pool)
        .await;

        #[allow(clippy::panic)]
        match val {
            Ok(val) => Ok(val),
            Err(err) => {
                panic!("{err}");
            }
        }
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        todo!()
    }

    #[allow(clippy::panic)]
    async fn insert_merchant_connector_account(
        &self,
        t: MerchantConnectorAccountNew,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let val = t
            .insert::<MerchantConnectorAccount>(&self.pool, "merchant_connector_account")
            .await;

        match val {
            Ok(val) => Ok(val),
            Err(err) => panic!("{err}"),
        }
    }

    async fn find_merchant_connector_account_by_merchant_id_list(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, errors::StorageError> {
        todo!()
    }

    async fn update_merchant_connector_account(
        &self,
        this: MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdate,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        todo!()
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<bool, errors::StorageError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for MockDb {
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let accounts = self.merchant_connector_accounts().await;

        Ok(accounts
            .iter()
            .find(|account| {
                account.merchant_id == merchant_id && account.connector_name == connector
            })
            .cloned()
            .unwrap())
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        todo!()
    }

    #[allow(clippy::panic)]
    async fn insert_merchant_connector_account(
        &self,
        t: MerchantConnectorAccountNew,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts().await;
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
        merchant_id: &str,
    ) -> CustomResult<Vec<MerchantConnectorAccount>, errors::StorageError> {
        todo!()
    }

    async fn update_merchant_connector_account(
        &self,
        this: MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdate,
    ) -> CustomResult<MerchantConnectorAccount, errors::StorageError> {
        todo!()
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &i32,
    ) -> CustomResult<bool, errors::StorageError> {
        todo!()
    }
}
