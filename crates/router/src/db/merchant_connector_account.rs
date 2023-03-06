use common_utils::ext_traits::{ByteSliceExt, Encode};
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;

use super::{MockDb, Store};
use crate::{
    connection::pg_connection,
    core::errors::{self, CustomResult},
    services::logger,
    types::{self, storage},
};

#[async_trait::async_trait]
pub trait ConnectorAccessToken {
    async fn get_access_token(
        &self,
        merchant_id: &str,
        connector_name: &str,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError>;

    async fn set_access_token(
        &self,
        merchant_id: &str,
        connector_name: &str,
        access_token: types::AccessToken,
    ) -> CustomResult<(), errors::StorageError>;
}

#[async_trait::async_trait]
impl ConnectorAccessToken for Store {
    async fn get_access_token(
        &self,
        merchant_id: &str,
        connector_name: &str,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError> {
        //TODO: Handle race condition
        // This function should acquire a global lock on some resource, if access token is already
        // being refreshed by other request then wait till it finishes and use the same access token
        let key = format!("access_token_{merchant_id}_{connector_name}");
        let maybe_token = self
            .redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .get_key::<Option<Vec<u8>>>(&key)
            .await
            .change_context(errors::StorageError::KVError)
            .attach_printable("DB error when getting access token")?;

        let access_token: Option<types::AccessToken> = maybe_token
            .map(|token| token.parse_struct("AccessToken"))
            .transpose()
            .change_context(errors::ParsingError)
            .change_context(errors::StorageError::DeserializationFailed)?;

        Ok(access_token)
    }

    async fn set_access_token(
        &self,
        merchant_id: &str,
        connector_name: &str,
        access_token: types::AccessToken,
    ) -> CustomResult<(), errors::StorageError> {
        let key = format!("access_token_{merchant_id}_{connector_name}");
        let serialized_access_token =
            Encode::<types::AccessToken>::encode_to_string_of_json(&access_token)
                .change_context(errors::StorageError::SerializationFailed)?;
        self.redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .set_key_with_expiry(&key, serialized_access_token, access_token.expires)
            .await
            .map_err(|error| {
                logger::error!(access_token_kv_error=?error);
                errors::StorageError::KVError
            })
            .into_report()
    }
}

#[async_trait::async_trait]
impl ConnectorAccessToken for MockDb {
    async fn get_access_token(
        &self,
        _merchant_id: &str,
        _connector_name: &str,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError> {
        Ok(None)
    }

    async fn set_access_token(
        &self,
        _merchant_id: &str,
        _connector_name: &str,
        _access_token: types::AccessToken,
    ) -> CustomResult<(), errors::StorageError> {
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait MerchantConnectorAccountInterface {
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError>;

    async fn insert_merchant_connector_account(
        &self,
        t: storage::MerchantConnectorAccountNew,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError>;

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError>;

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &str,
        get_disabled: bool,
    ) -> CustomResult<Vec<storage::MerchantConnectorAccount>, errors::StorageError>;

    async fn update_merchant_connector_account(
        &self,
        this: storage::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdate,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError>;

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for Store {
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id_connector(
            &conn,
            merchant_id,
            connector,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError> {
        let find_call = || async {
            let conn = pg_connection(&self.master_pool).await?;
            storage::MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
                &conn,
                merchant_id,
                merchant_connector_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        };
        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_cache(self, merchant_connector_id, find_call).await
        }
    }

    async fn insert_merchant_connector_account(
        &self,
        t: storage::MerchantConnectorAccountNew,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        t.insert(&conn).await.map_err(Into::into).into_report()
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &str,
        get_disabled: bool,
    ) -> CustomResult<Vec<storage::MerchantConnectorAccount>, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id(&conn, merchant_id, get_disabled)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_merchant_connector_account(
        &self,
        this: storage::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdate,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError> {
        let _merchant_connector_id = this.merchant_connector_id.clone();
        let update_call = || async {
            let conn = pg_connection(&self.master_pool).await?;
            this.update(&conn, merchant_connector_account)
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::redact_cache(self, &_merchant_connector_id, update_call).await
        }

        #[cfg(not(feature = "accounts_cache"))]
        {
            update_call().await
        }
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = pg_connection(&self.master_pool).await?;
        storage::MerchantConnectorAccount::delete_by_merchant_id_merchant_connector_id(
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
    // safety: only used for testing
    #[allow(clippy::unwrap_used)]
    async fn find_merchant_connector_account_by_merchant_id_connector(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError> {
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
        _merchant_connector_id: &str,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_merchant_connector_account(
        &self,
        t: storage::MerchantConnectorAccountNew,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        let account = storage::MerchantConnectorAccount {
            #[allow(clippy::as_conversions)]
            id: accounts.len() as i32,
            merchant_id: t.merchant_id.unwrap_or_default(),
            connector_name: t.connector_name.unwrap_or_default(),
            connector_account_details: t.connector_account_details.unwrap_or_default().expose(),
            test_mode: t.test_mode,
            disabled: t.disabled,
            merchant_connector_id: t.merchant_connector_id,
            payment_methods_enabled: t.payment_methods_enabled,
            metadata: t.metadata,
            connector_type: t
                .connector_type
                .unwrap_or(crate::types::storage::enums::ConnectorType::FinOperations),
        };
        accounts.push(account.clone());
        Ok(account)
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        _merchant_id: &str,
        _get_disabled: bool,
    ) -> CustomResult<Vec<storage::MerchantConnectorAccount>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_merchant_connector_account(
        &self,
        _this: storage::MerchantConnectorAccount,
        _merchant_connector_account: storage::MerchantConnectorAccountUpdate,
    ) -> CustomResult<storage::MerchantConnectorAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        _merchant_id: &str,
        _merchant_connector_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
