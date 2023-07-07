use common_utils::ext_traits::{AsyncExt, ByteSliceExt, Encode};
use error_stack::{IntoReport, ResultExt};

use super::{MockDb, Store};
#[cfg(feature = "accounts_cache")]
use crate::cache::{self, ACCOUNTS_CACHE};
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
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &str,
        get_disabled: bool,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError>;

    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &domain::MerchantKeyStore,
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
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_merchant_id_connector(
                &conn,
                merchant_id,
                connector_label,
            )
            .await
            .map_err(Into::into)
            .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DeserializationFailed)
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_in_memory(
                self,
                &format!("{}_{}", merchant_id, connector_label),
                find_call,
                &ACCOUNTS_CACHE,
            )
            .await
            .async_and_then(|item| async {
                item.convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
        }
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
            &conn,
            merchant_id,
            merchant_connector_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
        .async_and_then(|item| async {
            item.convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)
        })
        .await
    }

    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
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
                item.convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &str,
        get_disabled: bool,
        key_store: &domain::MerchantKeyStore,
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
                        item.convert(key_store.key.get_inner())
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
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let _merchant_id = this.merchant_id.clone();
        let _merchant_connector_label = this.connector_label.clone();

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
                    item.convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
        };

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::publish_and_redact(
                self,
                cache::CacheKind::Accounts(
                    format!("{}_{}", _merchant_id, _merchant_connector_label).into(),
                ),
                update_call,
            )
            .await
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
        let delete_call = || async {
            storage::MerchantConnectorAccount::delete_by_merchant_id_merchant_connector_id(
                &conn,
                merchant_id,
                merchant_connector_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        };

        #[cfg(feature = "accounts_cache")]
        {
            // We need to fetch mca here because the key that's saved in cache in
            // {merchant_id}_{connector_label}.
            // Used function from storage model to reuse the connection that made here instead of
            // creating new.

            let mca = storage::MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
                &conn,
                merchant_id,
                merchant_connector_id,
            )
            .await
            .map_err(Into::into)
            .into_report()?;

            super::cache::publish_and_redact(
                self,
                cache::CacheKind::Accounts(
                    format!("{}_{}", mca.merchant_id, mca.connector_label).into(),
                ),
                delete_call,
            )
            .await
        }

        #[cfg(not(feature = "accounts_cache"))]
        {
            delete_call().await
        }
    }
}

#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for MockDb {
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &str,
        connector: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        match self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .find(|account| {
                account.merchant_id == merchant_id && account.connector_name == connector
            })
            .cloned()
            .async_map(|account| async {
                account
                    .convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
        {
            Some(result) => result,
            None => {
                return Err(errors::StorageError::ValueNotFound(
                    "cannot find merchant connector account".to_string(),
                )
                .into())
            }
        }
    }

    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        match self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .find(|account| {
                account.merchant_id == merchant_id
                    && account.merchant_connector_id == merchant_connector_id
            })
            .cloned()
            .async_map(|account| async {
                account
                    .convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
        {
            Some(result) => result,
            None => {
                return Err(errors::StorageError::ValueNotFound(
                    "cannot find merchant connector account".to_string(),
                )
                .into())
            }
        }
    }

    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
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
            .convert(key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &str,
        get_disabled: bool,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        let accounts = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .filter(|account: &&storage::MerchantConnectorAccount| {
                if get_disabled {
                    account.merchant_id == merchant_id
                } else {
                    account.merchant_id == merchant_id && account.disabled == Some(false)
                }
            })
            .cloned()
            .collect::<Vec<storage::MerchantConnectorAccount>>();

        let mut output = Vec::with_capacity(accounts.len());
        for account in accounts.into_iter() {
            output.push(
                account
                    .convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)?,
            )
        }
        Ok(output)
    }

    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        match self
            .merchant_connector_accounts
            .lock()
            .await
            .iter_mut()
            .find(|account| Some(account.id) == this.id)
            .map(|a| {
                let updated =
                    merchant_connector_account.create_merchant_connector_account(a.clone());
                *a = updated.clone();
                updated
            })
            .async_map(|account| async {
                account
                    .convert(key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
        {
            Some(result) => result,
            None => {
                return Err(errors::StorageError::ValueNotFound(
                    "cannot find merchant connector account to update".to_string(),
                )
                .into())
            }
        }
    }

    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        match accounts.iter().position(|account| {
            account.merchant_id == merchant_id
                && account.merchant_connector_id == merchant_connector_id
        }) {
            Some(index) => {
                accounts.remove(index);
                return Ok(true);
            }
            None => {
                return Err(errors::StorageError::ValueNotFound(
                    "cannot find merchant connector account to delete".to_string(),
                )
                .into())
            }
        }
    }
}

#[cfg(test)]
mod merchant_connector_account_cache_tests {
    use api_models::enums::CountryAlpha2;
    use common_utils::date_time;
    use error_stack::ResultExt;
    use storage_models::enums::ConnectorType;

    use crate::{
        cache::{CacheKind, ACCOUNTS_CACHE},
        core::errors,
        db::{
            cache, merchant_connector_account::MerchantConnectorAccountInterface,
            merchant_key_store::MerchantKeyStoreInterface, MasterKeyInterface, MockDb,
        },
        services::{PubSubInterface, RedisConnInterface},
        types::{
            domain::{self, behaviour::Conversion, types as domain_types},
            storage,
        },
    };

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    async fn test_connector_label_cache() {
        let db = MockDb::new(&Default::default()).await;

        let redis_conn = db.get_redis_conn();
        let key = db.get_master_key();
        redis_conn
            .subscribe("hyperswitch_invalidate")
            .await
            .unwrap();

        let merchant_id = "test_merchant";
        let connector_label = "stripe_USA";
        let merchant_connector_id = "simple_merchant_connector_id";

        let mca = domain::MerchantConnectorAccount {
            id: Some(1),
            merchant_id: merchant_id.to_string(),
            connector_name: "stripe".to_string(),
            connector_account_details: domain_types::encrypt(
                serde_json::Value::default().into(),
                key,
            )
            .await
            .unwrap(),
            test_mode: None,
            disabled: None,
            merchant_connector_id: merchant_connector_id.to_string(),
            payment_methods_enabled: None,
            connector_type: ConnectorType::FinOperations,
            metadata: None,
            frm_configs: None,
            connector_label: connector_label.to_string(),
            business_country: CountryAlpha2::US,
            business_label: "cloth".to_string(),
            business_sub_label: None,
            created_at: date_time::now(),
            modified_at: date_time::now(),
        };

        let key_store = db
            .get_merchant_key_store_by_merchant_id(merchant_id, &key.to_vec().into())
            .await
            .unwrap();

        db.insert_merchant_connector_account(mca, &key_store)
            .await
            .unwrap();
        let find_call = || async {
            db.find_merchant_connector_account_by_merchant_id_connector_label(
                merchant_id,
                connector_label,
                &key_store,
            )
            .await
            .unwrap()
            .convert()
            .await
            .change_context(errors::StorageError::DecryptionError)
        };
        let _: storage::MerchantConnectorAccount = cache::get_or_populate_in_memory(
            &db,
            &format!("{}_{}", merchant_id, connector_label),
            find_call,
            &ACCOUNTS_CACHE,
        )
        .await
        .unwrap();

        let delete_call = || async {
            db.delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
                merchant_id,
                merchant_connector_id,
            )
            .await
        };

        cache::publish_and_redact(
            &db,
            CacheKind::Accounts(format!("{}_{}", merchant_id, connector_label).into()),
            delete_call,
        )
        .await
        .unwrap();

        assert!(ACCOUNTS_CACHE
            .get_val::<domain::MerchantConnectorAccount>(&format!(
                "{}_{}",
                merchant_id, connector_label
            ),)
            .is_none())
    }
}
