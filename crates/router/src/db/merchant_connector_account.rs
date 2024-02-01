use common_utils::ext_traits::{AsyncExt, ByteSliceExt, Encode};
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "accounts_cache")]
use storage_impl::redis::cache;
use storage_impl::redis::kv_store::RedisConnInterface;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
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
        /// Asynchronously retrieves an access token for a given merchant and connector from the storage.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - The ID of the merchant for whom the access token is being retrieved.
    /// * `connector_name` - The name of the connector for which the access token is being retrieved.
    /// 
    /// # Returns
    /// 
    /// Returns a `CustomResult` containing an `Option` of `types::AccessToken` or a `StorageError` if an error occurs.
    ///
    /// # Remarks
    ///
    /// This function acquires a global lock on some resource to handle the race condition when multiple requests are trying to refresh the access token simultaneously.
    /// If the access token is already being refreshed by another request, this function waits until the refresh process finishes and then uses the same access token.
    ///
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
            .get_redis_conn()
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

        /// Asynchronously sets the access token for a given merchant and connector in the storage. 
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - The ID of the merchant for which the access token is being set.
    /// * `connector_name` - The name of the connector for which the access token is being set.
    /// * `access_token` - The access token to be set.
    ///
    /// # Returns
    ///
    /// * `CustomResult<(), errors::StorageError>` - A result indicating success or an error of type `errors::StorageError`.
    ///
    /// # Errors
    ///
    /// This method can return an error of type `errors::StorageError` in case of serialization failure or key-value storage error.
    ///
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
        self.get_redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .set_key_with_expiry(&key, serialized_access_token, access_token.expires)
            .await
            .change_context(errors::StorageError::KVError)
    }
}

#[async_trait::async_trait]
impl ConnectorAccessToken for MockDb {
        /// Asynchronously retrieves the access token for the specified merchant and connector from the storage.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - a string slice representing the ID of the merchant
    /// * `connector_name` - a string slice representing the name of the connector
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing an `Option` of `types::AccessToken` if successful, otherwise returns a `StorageError`
    ///
    async fn get_access_token(
        &self,
        _merchant_id: &str,
        _connector_name: &str,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError> {
        Ok(None)
    }

        /// Asynchronously sets the access token for a given merchant and connector.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - The ID of the merchant for which the access token is being set.
    /// * `_connector_name` - The name of the connector for which the access token is being set.
    /// * `_access_token` - The access token to be set.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` indicating whether the operation was successful or if an error occurred.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if an error occurs while setting the access token.
    ///
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

    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        profile_id: &str,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        merchant_id: &str,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError>;

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
        /// Asynchronously finds a merchant connector account by the given merchant ID and connector label,
    /// using the provided MerchantKeyStore for decryption. If the "accounts_cache" feature is enabled, it
    /// also checks for the account in the in-memory cache and populates it if not found.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string slice representing the merchant ID
    /// * `connector_label` - A string slice representing the connector label
    /// * `key_store` - A reference to a MerchantKeyStore used for decryption
    ///
    /// # Returns
    ///
    /// A CustomResult containing the found MerchantConnectorAccount or a StorageError if an error occurs.
    ///
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
                &cache::ACCOUNTS_CACHE,
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

        /// Asynchronously finds a merchant connector account by profile ID and connector name, using the provided key store for decryption. If the "accounts_cache" feature is enabled, the method will attempt to retrieve the account from the in-memory cache before querying the database. If the account is not found in the cache, it will be fetched from the database and then stored in the cache for future use.
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        profile_id: &str,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_profile_id_connector_name(
                &conn,
                profile_id,
                connector_name,
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
                &format!("{}_{}", profile_id, connector_name),
                find_call,
                &cache::ACCOUNTS_CACHE,
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

        /// Asynchronously finds a merchant connector account by the specified merchant ID and connector name,
    /// decrypts the retrieved accounts using the provided merchant key store, and returns a vector of
    /// merchant connector accounts. If successful, it returns the vector of accounts, otherwise it
    /// returns a StorageError.
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        merchant_id: &str,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id_connector_name(
            &conn,
            merchant_id,
            connector_name,
        )
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

        /// Asynchronously finds a merchant connector account by the specified merchant ID and merchant connector ID,
    /// using the provided merchant key store for encryption and decryption. If the 'accounts_cache' feature is enabled,
    /// it will attempt to retrieve the account from the cache before querying the database. Returns a CustomResult
    /// containing the found domain::MerchantConnectorAccount or an errors::StorageError if the operation fails.
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &str,
        merchant_connector_id: &str,
        key_store: &domain::MerchantKeyStore,
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
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_in_memory(
                self,
                &format!("{}_{}", merchant_id, merchant_connector_id),
                find_call,
                &cache::ACCOUNTS_CACHE,
            )
            .await?
            .convert(key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
        }
    }

        /// Inserts a new merchant connector account into the database after constructing it, encrypting it, and converting it using the provided key store. 
    ///
    /// # Arguments
    ///
    /// * `t` - The new merchant connector account to be inserted
    /// * `key_store` - The key store used for encryption and decryption
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the inserted merchant connector account on success, or a `StorageError` on failure.
    ///
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

        /// Asynchronously finds merchant connector accounts by merchant ID and disabled list,
    /// using the provided merchant key store for decryption. Returns a vector of
    /// `domain::MerchantConnectorAccount`, or a `StorageError` if an error occurs.
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

        /// Asynchronously updates a merchant connector account in the storage with the provided
    /// merchant connector account update internal and merchant key store. It performs the
    /// necessary encryption and decryption operations using the key store. If the feature
    /// "accounts_cache" is enabled, it also updates the caches for the account. Returns a
    /// result containing the updated merchant connector account or a storage error.
    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let _connector_name = this.connector_name.clone();
        let _profile_id = this
            .profile_id
            .clone()
            .ok_or(errors::StorageError::ValueNotFound(
                "profile_id".to_string(),
            ))?;

        let _merchant_id = this.merchant_id.clone();
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
                    item.convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
        };

        #[cfg(feature = "accounts_cache")]
        {
            // Redact both the caches as any one or both might be used because of backwards compatibility
            super::cache::publish_and_redact_multiple(
                self,
                [
                    cache::CacheKind::Accounts(
                        format!("{}_{}", _profile_id, _connector_name).into(),
                    ),
                    cache::CacheKind::Accounts(
                        format!("{}_{}", _merchant_id, _merchant_connector_id).into(),
                    ),
                ],
                update_call,
            )
            .await
        }

        #[cfg(not(feature = "accounts_cache"))]
        {
            update_call().await
        }
    }

        /// Asynchronously deletes a merchant connector account by the given merchant ID and merchant connector ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string reference representing the ID of the merchant
    /// * `merchant_connector_id` - A string reference representing the ID of the merchant connector
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a boolean indicating whether the deletion was successful, or an `errors::StorageError` in case of failure.
    ///
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

            let _profile_id = mca.profile_id.ok_or(errors::StorageError::ValueNotFound(
                "profile_id".to_string(),
            ))?;

            super::cache::publish_and_redact(
                self,
                cache::CacheKind::Accounts(format!("{}_{}", mca.merchant_id, _profile_id).into()),
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
        /// Asynchronously finds a merchant connector account by the given merchant ID and connector label, and decrypts the account using the provided key store. Returns a `CustomResult` containing the decrypted `MerchantConnectorAccount` if found, otherwise returns a `StorageError` indicating that the value was not found.
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
                account.merchant_id == merchant_id
                    && account.connector_label == Some(connector.to_string())
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

        /// Asynchronously finds the merchant connector account by the given merchant ID and connector name,
    /// then decrypts the account using the provided key store. Returns a vector of MerchantConnectorAccount
    /// or a StorageError if decryption fails.
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        merchant_id: &str,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        let accounts = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .filter(|account| {
                account.merchant_id == merchant_id && account.connector_name == connector_name
            })
            .cloned()
            .collect::<Vec<_>>();
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

        /// Asynchronously finds a merchant connector account by the given profile ID and connector name.
    /// If found, it converts the account using the provided key store and returns the result.
    /// If not found, it returns an error indicating that the merchant connector account cannot be found.
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        profile_id: &str,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let maybe_mca = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .find(|account| {
                account.profile_id.eq(&Some(profile_id.to_owned()))
                    && account.connector_name == connector_name
            })
            .cloned();

        match maybe_mca {
            Some(mca) => mca
                .to_owned()
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find merchant connector account".to_string(),
            )
            .into()),
        }
    }

        /// Asynchronously finds a merchant connector account by the given merchant ID and merchant connector ID,
    /// and decrypts the account using the provided key store. If the account is found, it returns the decrypted
    /// merchant connector account. If the account is not found, it returns a storage error indicating that the
    /// account could not be found.
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

        /// Asynchronously inserts a merchant connector account into the storage,
    /// encrypting the sensitive data using the provided key store.
    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        let account = storage::MerchantConnectorAccount {
            id: accounts
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            merchant_id: t.merchant_id,
            connector_name: t.connector_name,
            connector_account_details: t.connector_account_details.into(),
            test_mode: t.test_mode,
            disabled: t.disabled,
            merchant_connector_id: t.merchant_connector_id,
            payment_methods_enabled: t.payment_methods_enabled,
            metadata: t.metadata,
            frm_configs: None,
            frm_config: t.frm_configs,
            connector_type: t.connector_type,
            connector_label: t.connector_label,
            business_country: t.business_country,
            business_label: t.business_label,
            business_sub_label: t.business_sub_label,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            connector_webhook_details: t.connector_webhook_details,
            profile_id: t.profile_id,
            applepay_verified_domains: t.applepay_verified_domains,
            pm_auth_config: t.pm_auth_config,
            status: t.status,
        };
        accounts.push(account.clone());
        account
            .convert(key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

        /// Asynchronously finds merchant connector accounts by merchant ID and disabled list.
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

        /// Asynchronously updates a merchant connector account with the provided data and returns the updated account.
    ///
    /// # Arguments
    ///
    /// * `this` - The merchant connector account to be updated
    /// * `merchant_connector_account` - The updated data for the merchant connector account
    /// * `key_store` - The key store used for encryption and decryption
    ///
    /// # Returns
    ///
    /// The updated merchant connector account if found, or a `StorageError` if the account is not found.
    ///
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

        /// Asynchronously deletes a merchant connector account by the given merchant ID and merchant connector ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string reference representing the merchant ID
    /// * `merchant_connector_id` - A string reference representing the merchant connector ID
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a boolean indicating whether the deletion was successful, or a `StorageError` if the account was not found.
    ///
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
    use diesel_models::enums::ConnectorType;
    use error_stack::ResultExt;
    use masking::PeekInterface;
    use storage_impl::redis::{
        cache::{CacheKind, ACCOUNTS_CACHE},
        kv_store::RedisConnInterface,
        pub_sub::PubSubInterface,
    };
    use time::macros::datetime;

    use crate::{
        core::errors,
        db::{
            cache, merchant_connector_account::MerchantConnectorAccountInterface,
            merchant_key_store::MerchantKeyStoreInterface, MasterKeyInterface, MockDb,
        },
        services,
        types::{
            domain::{self, behaviour::Conversion},
            storage,
        },
    };

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
        /// Asynchronously tests the connector profile ID cache by performing a series of database operations
    /// including insertion, retrieval, and deletion of merchant connector account data using Redis
    /// cache. It subscribes to a Redis channel, inserts a merchant key into the database, retrieves
    /// the merchant key, inserts a merchant connector account, populates the in-memory cache, deletes
    /// the merchant connector account, and verifies the cache is empty. Returns () when the test is
    /// successful.
    async fn test_connector_profile_id_cache() {
        #[allow(clippy::expect_used)]
        let db = MockDb::new(&redis_interface::RedisSettings::default())
            .await
            .expect("Failed to create Mock store");

        let redis_conn = db.get_redis_conn().unwrap();
        let master_key = db.get_master_key();
        redis_conn
            .subscribe("hyperswitch_invalidate")
            .await
            .unwrap();

        let merchant_id = "test_merchant";
        let connector_label = "stripe_USA";
        let merchant_connector_id = "simple_merchant_connector_id";
        let profile_id = "pro_max_ultra";

        db.insert_merchant_key_store(
            domain::MerchantKeyStore {
                merchant_id: merchant_id.into(),
                key: domain::types::encrypt(
                    services::generate_aes256_key().unwrap().to_vec().into(),
                    master_key,
                )
                .await
                .unwrap(),
                created_at: datetime!(2023-02-01 0:00),
            },
            &master_key.to_vec().into(),
        )
        .await
        .unwrap();

        let merchant_key = db
            .get_merchant_key_store_by_merchant_id(merchant_id, &master_key.to_vec().into())
            .await
            .unwrap();

        let mca = domain::MerchantConnectorAccount {
            id: Some(1),
            merchant_id: merchant_id.to_string(),
            connector_name: "stripe".to_string(),
            connector_account_details: domain::types::encrypt(
                serde_json::Value::default().into(),
                merchant_key.key.get_inner().peek(),
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
            connector_label: Some(connector_label.to_string()),
            business_country: Some(CountryAlpha2::US),
            business_label: Some("cloth".to_string()),
            business_sub_label: None,
            created_at: date_time::now(),
            modified_at: date_time::now(),
            connector_webhook_details: None,
            profile_id: Some(profile_id.to_string()),
            applepay_verified_domains: None,
            pm_auth_config: None,
            status: common_enums::ConnectorStatus::Inactive,
        };

        db.insert_merchant_connector_account(mca.clone(), &merchant_key)
            .await
            .unwrap();

        let find_call = || async {
            Conversion::convert(
                db.find_merchant_connector_account_by_profile_id_connector_name(
                    profile_id,
                    &mca.connector_name,
                    &merchant_key,
                )
                .await
                .unwrap(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        };
        let _: storage::MerchantConnectorAccount = cache::get_or_populate_in_memory(
            &db,
            &format!("{}_{}", merchant_id, profile_id),
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
            .await
            .is_none())
    }
}
