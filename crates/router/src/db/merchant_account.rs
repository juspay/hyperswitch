use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    types::storage::{self, enums},
};

#[async_trait::async_trait]
pub trait MerchantAccountInterface {
    async fn insert_merchant(
        &self,
        merchant_account: storage::MerchantAccountNew,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError>;

    async fn update_merchant(
        &self,
        this: storage::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError>;

    async fn update_specific_fields_in_merchant(
        &self,
        merchant_id: &str,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError>;

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl MerchantAccountInterface for Store {
    async fn insert_merchant(
        &self,
        merchant_account: storage::MerchantAccountNew,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        merchant_account
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        let fetch_func = || async {
            let conn = pg_connection(&self.master_pool).await?;
            storage::MerchantAccount::find_by_merchant_id(&conn, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            fetch_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_cache(self, merchant_id, fetch_func).await
        }
    }

    async fn update_merchant(
        &self,
        this: storage::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        let _merchant_id = this.merchant_id.clone();
        let update_func = || async {
            let conn = pg_connection(&self.master_pool).await?;
            this.update(&conn, merchant_account)
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            update_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::redact_cache(self, &_merchant_id, update_func).await
        }
    }

    async fn update_specific_fields_in_merchant(
        &self,
        merchant_id: &str,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        let update_func = || async {
            let conn = pg_connection(&self.master_pool).await?;
            storage::MerchantAccount::update_with_specific_fields(
                &conn,
                merchant_id,
                merchant_account,
            )
            .await
            .map_err(Into::into)
            .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            update_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::redact_cache(self, merchant_id, update_func).await
        }
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::MerchantAccount::find_by_publishable_key(&conn, publishable_key)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let delete_func = || async {
            let conn = pg_connection(&self.master_pool).await?;
            storage::MerchantAccount::delete_by_merchant_id(&conn, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            delete_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::redact_cache(self, merchant_id, delete_func).await
        }
    }
}

#[async_trait::async_trait]
impl MerchantAccountInterface for MockDb {
    #[allow(clippy::panic)]
    async fn insert_merchant(
        &self,
        merchant_account: storage::MerchantAccountNew,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        let account = storage::MerchantAccount {
            #[allow(clippy::as_conversions)]
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
            sub_merchants_enabled: merchant_account.sub_merchants_enabled,
            parent_merchant_id: merchant_account.parent_merchant_id,
            publishable_key: merchant_account.publishable_key,
            storage_scheme: enums::MerchantStorageScheme::PostgresOnly,
            locker_id: merchant_account.locker_id,
            metadata: merchant_account.metadata,
        };
        accounts.push(account.clone());
        Ok(account)
    }

    #[allow(clippy::panic)]
    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        let accounts = self.merchant_accounts.lock().await;
        let account = accounts
            .iter()
            .find(|account| account.merchant_id == merchant_id);

        match account {
            Some(account) => Ok(account.clone()),
            // [#172]: Implement function for `MockDb`
            None => Err(errors::StorageError::MockDbError)?,
        }
    }

    async fn update_merchant(
        &self,
        _this: storage::MerchantAccount,
        _merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_specific_fields_in_merchant(
        &self,
        _merchant_id: &str,
        _merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        // [#TODO]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        _publishable_key: &str,
    ) -> CustomResult<storage::MerchantAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
