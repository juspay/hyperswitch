use common_utils::ext_traits::{AsyncExt, ByteSliceExt, Encode};
use error_stack::{IntoReport, ResultExt};

#[cfg(feature = "accounts_cache")]
use super::cache;
use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::logger,
    types::{
        self,
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage,
    },
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
            .change_context(errors::ParsingError::UnknownError)
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
pub trait MerchantConnectorAccountInterface
where
    domain::MerchantConnectorAccount: Conversion<
        DstType = storage::MerchantConnectorAccount,
        NewDstType = storage::MerchantConnectorAccountNew,
    >,
{
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &str,
        connector_label: &str,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &str,
        get_disabled: bool,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError>;

    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for Store {
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &str,
        connector_label: &str,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id_connector(
            &conn,
            merchant_id,
            connector_label,
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|item| async {
            item.convert(self, merchant_id)
                .await
                .change_context(errors::StorageError::DecryptionError)
        })
        .await
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
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
            find_call()
                .await?
                .convert(self, merchant_id)
                .await
                .change_context(errors::StorageError::DeserializationFailed)
        }

        #[cfg(feature = "accounts_cache")]
        {
            cache::get_or_populate_redis(self, merchant_connector_id, find_call)
                .await?
                .convert(self, merchant_id)
                .await
                .change_context(errors::StorageError::DeserializationFailed)
        }
    }

    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        t.construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|item| async {
                let merchant_id = item.merchant_id.clone();
                item.convert(self, &merchant_id)
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &str,
        get_disabled: bool,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id(&conn, merchant_id, get_disabled)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|items| async {
                let mut output = Vec::with_capacity(items.len());
                for item in items.into_iter() {
                    output.push(
                        item.convert(self, merchant_id)
                            .await
                            .change_context(errors::StorageError::DecryptionError)?,
                    )
                }
                Ok(output)
            })
            .await
    }

    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let _merchant_connector_id = this.merchant_connector_id.clone();
        let update_call = || async {
            let conn = connection::pg_connection_write(self).await?;
            Conversion::convert(this)
                .await
                .change_context(errors::StorageError::EncryptionError)?
                .update(&conn, merchant_connector_account)
                .await
                .map_err(Into::into)
                .into_report()
                .async_and_then(|item| async {
                    let merchant_id = item.merchant_id.clone();
                    item.convert(self, &merchant_id)
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
        };

        #[cfg(feature = "accounts_cache")]
        {
            cache::redact_cache(self, &_merchant_connector_id, update_call, None).await
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
        let conn = connection::pg_connection_write(self).await?;
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
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &str,
        connector: &str,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let accounts = self.merchant_connector_accounts.lock().await;
        let account = accounts
            .iter()
            .find(|account| {
                account.merchant_id == merchant_id && account.connector_name == connector
            })
            .cloned()
            .unwrap();
        account
            .convert(self, merchant_id)
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        _merchant_id: &str,
        _merchant_connector_id: &str,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[allow(clippy::panic)]
    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        let merchant_id = t.merchant_id.clone();
        let account = storage::MerchantConnectorAccount {
            #[allow(clippy::as_conversions)]
            id: accounts.len() as i32,
            merchant_id: t.merchant_id,
            connector_name: t.connector_name,
            connector_account_details: t.connector_account_details.into(),
            test_mode: t.test_mode,
            disabled: t.disabled,
            merchant_connector_id: t.merchant_connector_id,
            payment_methods_enabled: t.payment_methods_enabled,
            metadata: t.metadata,
            frm_configs: t.frm_configs,
            connector_type: t.connector_type,
            connector_label: t.connector_label,
            business_country: t.business_country,
            business_label: t.business_label,
            business_sub_label: t.business_sub_label,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
        };
        accounts.push(account.clone());
        account
            .convert(self, &merchant_id)
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        _merchant_id: &str,
        _get_disabled: bool,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_merchant_connector_account(
        &self,
        _this: domain::MerchantConnectorAccount,
        _merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
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
