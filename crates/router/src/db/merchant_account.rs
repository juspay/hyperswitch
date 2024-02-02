#[cfg(feature = "olap")]
use std::collections::HashMap;

use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "accounts_cache")]
use storage_impl::redis::cache::{CacheKind, ACCOUNTS_CACHE};

use super::{MasterKeyInterface, MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::merchant_key_store::MerchantKeyStoreInterface,
    services::authentication,
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage,
    },
};

#[async_trait::async_trait]
pub trait MerchantAccountInterface
where
    domain::MerchantAccount:
        Conversion<DstType = storage::MerchantAccount, NewDstType = storage::MerchantAccountNew>,
{
    async fn insert_merchant(
        &self,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn update_merchant(
        &self,
        this: domain::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn update_specific_fields_in_merchant(
        &self,
        merchant_id: &str,
        merchant_account: storage::MerchantAccountUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<authentication::AuthenticationData, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        organization_id: &str,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError>;

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn list_multiple_merchant_accounts(
        &self,
        merchant_ids: Vec<String>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError>;
}

#[async_trait::async_trait]
impl MerchantAccountInterface for Store {
        /// Asynchronously inserts a new merchant account into the database after constructing a new account, encrypting the data, and converting the merchant key. Returns a Result containing the inserted domain::MerchantAccount or an errors::StorageError.
    async fn insert_merchant(
        &self,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        merchant_account
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()?
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

        /// Asynchronously finds a merchant account by the given merchant ID using the provided merchant key store.
    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let fetch_func = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantAccount::find_by_merchant_id(&conn, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            fetch_func()
                .await?
                .convert(merchant_key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_in_memory(self, merchant_id, fetch_func, &ACCOUNTS_CACHE)
                .await?
                .convert(merchant_key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }
    }

        /// Asynchronously updates a merchant account in the storage, using the provided merchant account data and key store. Returns a CustomResult with the updated merchant account if successful, or a StorageError if an error occurs during the update process.
    async fn update_merchant(
        &self,
        this: domain::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        let updated_merchant_account = Conversion::convert(this)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update(&conn, merchant_account.into())
            .await
            .map_err(Into::into)
            .into_report()?;

        #[cfg(feature = "accounts_cache")]
        {
            publish_and_redact_merchant_account_cache(self, &updated_merchant_account).await?;
        }
        updated_merchant_account
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

        /// Asynchronously updates specific fields in a merchant account and returns the updated merchant account.
    async fn update_specific_fields_in_merchant(
        &self,
        merchant_id: &str,
        merchant_account: storage::MerchantAccountUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let updated_merchant_account = storage::MerchantAccount::update_with_specific_fields(
            &conn,
            merchant_id,
            merchant_account.into(),
        )
        .await
        .map_err(Into::into)
        .into_report()?;

        #[cfg(feature = "accounts_cache")]
        {
            publish_and_redact_merchant_account_cache(self, &updated_merchant_account).await?;
        }
        updated_merchant_account
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

        /// Asynchronously finds a merchant account by the given publishable key. This method first checks if the "accounts_cache" feature is enabled, and if so, it attempts to retrieve the merchant account from the in-memory cache. If the feature is not enabled, or if the cache does not contain the account, it fetches the account from the database. Once the merchant account is retrieved, it obtains the key store associated with the merchant ID and the master key, and returns an `authentication::AuthenticationData` struct containing the decrypted merchant account and the key store.
    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<authentication::AuthenticationData, errors::StorageError> {
        let fetch_by_pub_key_func = || async {
            let conn = connection::pg_connection_read(self).await?;

            storage::MerchantAccount::find_by_publishable_key(&conn, publishable_key)
                .await
                .map_err(Into::into)
                .into_report()
        };

        let merchant_account;
        #[cfg(not(feature = "accounts_cache"))]
        {
            merchant_account = fetch_by_pub_key_func().await?;
        }

        #[cfg(feature = "accounts_cache")]
        {
            merchant_account = super::cache::get_or_populate_in_memory(
                self,
                publishable_key,
                fetch_by_pub_key_func,
                &ACCOUNTS_CACHE,
            )
            .await?;
        }
        let key_store = self
            .get_merchant_key_store_by_merchant_id(
                &merchant_account.merchant_id,
                &self.get_master_key().to_vec().into(),
            )
            .await?;

        Ok(authentication::AuthenticationData {
            merchant_account: merchant_account
                .convert(key_store.key.get_inner())
                .await
                .change_context(errors::StorageError::DecryptionError)?,

            key_store,
        })
    }

    #[cfg(feature = "olap")]
        /// Retrieves a list of merchant accounts associated with the given organization ID. This method
    /// decrypts the encrypted merchant accounts using the master key, retrieves the corresponding
    /// merchant key stores, and converts the decrypted merchant accounts. It returns a vector
    /// containing the domain::MerchantAccount objects.
    async fn list_merchant_accounts_by_organization_id(
        &self,
        organization_id: &str,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        use futures::future::try_join_all;
        let conn = connection::pg_connection_read(self).await?;

        let encrypted_merchant_accounts =
            storage::MerchantAccount::list_by_organization_id(&conn, organization_id)
                .await
                .map_err(Into::into)
                .into_report()?;

        let db_master_key = self.get_master_key().to_vec().into();

        let merchant_key_stores =
            try_join_all(encrypted_merchant_accounts.iter().map(|merchant_account| {
                self.get_merchant_key_store_by_merchant_id(
                    &merchant_account.merchant_id,
                    &db_master_key,
                )
            }))
            .await?;

        let merchant_accounts = try_join_all(
            encrypted_merchant_accounts
                .into_iter()
                .zip(merchant_key_stores.iter())
                .map(|(merchant_account, key_store)| async {
                    merchant_account
                        .convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                }),
        )
        .await?;

        Ok(merchant_accounts)
    }

        /// Asynchronously deletes a merchant account by its ID. If the 'accounts_cache' feature is enabled, it also updates the merchant account cache after deletion.
        async fn delete_merchant_account_by_merchant_id(
            &self,
            merchant_id: &str,
        ) -> CustomResult<bool, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
    
            let is_deleted_func = || async {
                storage::MerchantAccount::delete_by_merchant_id(&conn, merchant_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
    
            let is_deleted;
    
            #[cfg(not(feature = "accounts_cache"))]
            {
                is_deleted = is_deleted_func().await?;
            }
    
            #[cfg(feature = "accounts_cache")]
            {
                let merchant_account =
                    storage::MerchantAccount::find_by_merchant_id(&conn, merchant_id)
                        .await
                        .map_err(Into::into)
                        .into_report()?;
    
                is_deleted = is_deleted_func().await?;
    
                publish_and_redact_merchant_account_cache(self, &merchant_account).await?;
            }
    
            Ok(is_deleted)
        }

    #[cfg(feature = "olap")]
        /// Retrieves multiple merchant accounts from the database, decrypts their encrypted data using the corresponding key stores, and returns the decrypted merchant accounts.
    async fn list_multiple_merchant_accounts(
        &self,
        merchant_ids: Vec<String>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        let encrypted_merchant_accounts =
            storage::MerchantAccount::list_multiple_merchant_accounts(&conn, merchant_ids)
                .await
                .map_err(Into::into)
                .into_report()?;

        let db_master_key = self.get_master_key().to_vec().into();

        let merchant_key_stores = self
            .list_multiple_key_stores(
                encrypted_merchant_accounts
                    .iter()
                    .map(|merchant_account| &merchant_account.merchant_id)
                    .cloned()
                    .collect(),
                &db_master_key,
            )
            .await?;

        let key_stores_by_id: HashMap<_, _> = merchant_key_stores
            .iter()
            .map(|key_store| (key_store.merchant_id.to_owned(), key_store))
            .collect();

        let merchant_accounts =
            futures::future::try_join_all(encrypted_merchant_accounts.into_iter().map(
                |merchant_account| async {
                    let key_store = key_stores_by_id.get(&merchant_account.merchant_id).ok_or(
                        errors::StorageError::ValueNotFound(format!(
                            "merchant_key_store with merchant_id = {}",
                            merchant_account.merchant_id
                        )),
                    )?;
                    merchant_account
                        .convert(key_store.key.get_inner())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                },
            ))
            .await?;

        Ok(merchant_accounts)
    }
}

#[async_trait::async_trait]
impl MerchantAccountInterface for MockDb {
    #[allow(clippy::panic)]
        /// Asynchronously inserts a merchant account into the storage, encrypting the account data using the provided merchant key store.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_account` - The merchant account to be inserted into the storage.
    /// * `merchant_key_store` - The key store used for encrypting the merchant account data.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the inserted merchant account if successful, otherwise a `StorageError` is returned.
    /// 
    async fn insert_merchant(
        &self,
        mut merchant_account: domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        merchant_account.id.get_or_insert(
            accounts
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
        );
        let account = Conversion::convert(merchant_account)
            .await
            .change_context(errors::StorageError::EncryptionError)?;
        accounts.push(account.clone());

        account
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[allow(clippy::panic)]
        /// Asynchronously finds a merchant account by the given merchant ID using the provided merchant key store.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID to search for.
    /// * `merchant_key_store` - A reference to a `MerchantKeyStore` containing the keys necessary for decryption.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `MerchantAccount` if found, or a `StorageError` if not found or if an error occurs during decryption.
    ///
    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let accounts = self.merchant_accounts.lock().await;
        let account: Option<domain::MerchantAccount> = accounts
            .iter()
            .find(|account| account.merchant_id == merchant_id)
            .cloned()
            .async_map(|a| async {
                a.convert(merchant_key_store.key.get_inner())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?;

        match account {
            Some(account) => Ok(account),
            // [#172]: Implement function for `MockDb`
            None => Err(errors::StorageError::MockDbError)?,
        }
    }

        /// Updates a merchant account with the provided merchant account update and merchant key store.
    /// 
    /// # Arguments
    /// 
    /// - `_this`: The merchant account to be updated.
    /// - `_merchant_account`: The update to be applied to the merchant account.
    /// - `_merchant_key_store`: The key store for the merchant.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the updated `MerchantAccount` if successful, otherwise a `StorageError`.
    /// 
    /// # Errors
    /// 
    /// An error of type `StorageError` is returned if the update operation fails.
    /// 
    async fn update_merchant(
        &self,
        _this: domain::MerchantAccount,
        _merchant_account: storage::MerchantAccountUpdate,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously updates specific fields in a merchant account in the storage.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - The ID of the merchant account to be updated.
    /// * `_merchant_account` - The updated merchant account information.
    /// * `_merchant_key_store` - The key store for the merchant account.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the updated `MerchantAccount` if successful, otherwise returns a `StorageError`.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::MockDbError` if the function for `MockDb` is not implemented.
    ///
    async fn update_specific_fields_in_merchant(
        &self,
        _merchant_id: &str,
        _merchant_account: storage::MerchantAccountUpdate,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        // [#TODO]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds a merchant account using the provided publishable key.
    /// 
    /// # Arguments
    /// 
    /// * `_publishable_key` - A reference to a string containing the publishable key to search for.
    /// 
    /// # Returns
    /// 
    /// * `CustomResult<authentication::AuthenticationData, errors::StorageError>` - Result containing the found merchant account data if successful, or a `StorageError` if there was an error.
    /// 
    async fn find_merchant_account_by_publishable_key(
        &self,
        _publishable_key: &str,
    ) -> CustomResult<authentication::AuthenticationData, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Deletes a merchant account by the given merchant ID.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - The ID of the merchant account to be deleted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a boolean value indicating whether the deletion was successful or not, or a `StorageError` if an error occurred.
    ///
    async fn delete_merchant_account_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
        /// Asynchronously retrieves a list of merchant accounts associated with the specified organization ID.
    /// 
    /// # Arguments
    /// 
    /// * `organization_id` - A string slice representing the unique identifier of the organization.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` that contains a `Vec` of `domain::MerchantAccount` if successful, otherwise an `errors::StorageError` is returned.
    /// 
    async fn list_merchant_accounts_by_organization_id(
        &self,
        _organization_id: &str,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
        /// Asynchronously retrieves multiple merchant accounts by their IDs from the storage.
    ///
    /// # Arguments
    ///
    /// * `merchant_ids` - A vector of strings representing the IDs of the merchant accounts to retrieve.
    ///
    /// # Returns
    ///
    /// A `CustomResult` that resolves to a vector of `MerchantAccount` objects if the operation is successful, otherwise an `errors::StorageError` is returned.
    ///
    /// # Errors
    ///
    /// An `errors::StorageError::MockDbError` will be returned if the operation encounters a mock database error.
    ///
    async fn list_multiple_merchant_accounts(
        &self,
        _merchant_ids: Vec<String>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[cfg(feature = "accounts_cache")]
/// Asynchronously publishes the merchant account information into a cache and then redacts it.
///
/// # Arguments
///
/// * `store` - A reference to a storage interface.
/// * `merchant_account` - A reference to the merchant account to be published and redacted.
///
/// # Returns
///
/// A Result containing either `()` on success or a `StorageError` on failure.
///
async fn publish_and_redact_merchant_account_cache(
    store: &dyn super::StorageInterface,
    merchant_account: &storage::MerchantAccount,
) -> CustomResult<(), errors::StorageError> {
    let publishable_key = merchant_account
        .publishable_key
        .as_ref()
        .map(|publishable_key| CacheKind::Accounts(publishable_key.into()));

    let mut cache_keys = vec![CacheKind::Accounts(
        merchant_account.merchant_id.as_str().into(),
    )];

    cache_keys.extend(publishable_key.into_iter());

    super::cache::publish_into_redact_channel(store, cache_keys).await?;
    Ok(())
}
