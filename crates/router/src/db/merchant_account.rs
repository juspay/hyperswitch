use error_stack::{IntoReport, Report};
use masking::PeekInterface;

use super::MockDb;
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{enums, MerchantAccount, MerchantAccountNew, MerchantAccountUpdate},
};

#[async_trait::async_trait]
pub trait MerchantAccountInterface {
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
impl MerchantAccountInterface for super::Store {
    async fn insert_merchant(
        &self,
        merchant_account: MerchantAccountNew,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        merchant_account
            .insert_diesel(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        MerchantAccount::find_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_merchant(
        &self,
        this: MerchantAccount,
        merchant_account: MerchantAccountUpdate,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        this.update(&conn, merchant_account)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_merchant_account_by_api_key(
        &self,
        api_key: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        MerchantAccount::find_by_api_key(&conn, api_key)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        MerchantAccount::find_by_publishable_key(&conn, publishable_key)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await;
        MerchantAccount::delete_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl MerchantAccountInterface for MockDb {
    #[allow(clippy::panic)]
    async fn insert_merchant(
        &self,
        merchant_account: MerchantAccountNew,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        let account = MerchantAccount {
            id: accounts.len() as i32,
            merchant_id: merchant_account.merchant_id,
            api_key: merchant_account.api_key,
            return_url: merchant_account.return_url,
            enable_payment_response_hash: merchant_account
                .enable_payment_response_hash
                .unwrap_or_default(),
            payment_response_hash_key: merchant_account.payment_response_hash_key,
            redirect_to_merchant_with_http_post: merchant_account
                .redirect_to_merchant_with_http_post
                .unwrap_or_default(),
            merchant_name: merchant_account.merchant_name,
            merchant_details: merchant_account.merchant_details,
            webhook_details: merchant_account.webhook_details,
            routing_algorithm: merchant_account.routing_algorithm,
            custom_routing_rules: merchant_account.custom_routing_rules,
            sub_merchants_enabled: merchant_account.sub_merchants_enabled,
            parent_merchant_id: merchant_account.parent_merchant_id,
            publishable_key: merchant_account.publishable_key,
            storage_scheme: enums::MerchantStorageScheme::PostgresOnly,
        };
        accounts.push(account.clone());
        Ok(account)
    }

    #[allow(clippy::panic)]
    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let accounts = self.merchant_accounts.lock().await;
        let account = accounts
            .iter()
            .find(|account| account.merchant_id == merchant_id);

        match account {
            Some(account) => Ok(account.clone()),
            None => todo!(),
        }
    }

    async fn update_merchant(
        &self,
        _this: MerchantAccount,
        _merchant_account: MerchantAccountUpdate,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        todo!()
    }

    #[allow(clippy::panic)]
    async fn find_merchant_account_by_api_key(
        &self,
        api_key: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        let accounts = self.merchant_accounts.lock().await;

        accounts
            .iter()
            .find(|account| account.api_key.as_ref().map(|s| s.peek()) == Some(&api_key.into()))
            .cloned()
            .ok_or_else(|| Report::from(storage_models::errors::DatabaseError::NotFound).into())
            .into_report()
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        _publishable_key: &str,
    ) -> CustomResult<MerchantAccount, errors::StorageError> {
        todo!()
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        todo!()
    }
}
