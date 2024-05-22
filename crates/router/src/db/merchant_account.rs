#[cfg(feature = "olap")]
use std::collections::HashMap;

use common_utils::ext_traits::AsyncExt;
use diesel_models::MerchantAccountUpdateInternal;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
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

    async fn update_all_merchant_account(
        &self,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<usize, errors::StorageError>;

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
    #[instrument(skip_all)]
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
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let fetch_func = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantAccount::find_by_merchant_id(&conn, merchant_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
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

    #[instrument(skip_all)]
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
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        #[cfg(feature = "accounts_cache")]
        {
            publish_and_redact_merchant_account_cache(self, &updated_merchant_account).await?;
        }
        updated_merchant_account
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
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
        .map_err(|error| report!(errors::StorageError::from(error)))?;

        #[cfg(feature = "accounts_cache")]
        {
            publish_and_redact_merchant_account_cache(self, &updated_merchant_account).await?;
        }
        updated_merchant_account
            .convert(merchant_key_store.key.get_inner())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<authentication::AuthenticationData, errors::StorageError> {
        let fetch_by_pub_key_func = || async {
            let conn = connection::pg_connection_read(self).await?;

            storage::MerchantAccount::find_by_publishable_key(&conn, publishable_key)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
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
    #[instrument(skip_all)]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        organization_id: &str,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        use futures::future::try_join_all;
        let conn = connection::pg_connection_read(self).await?;

        let encrypted_merchant_accounts =
            storage::MerchantAccount::list_by_organization_id(&conn, organization_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;

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

    #[instrument(skip_all)]
    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        let is_deleted_func = || async {
            storage::MerchantAccount::delete_by_merchant_id(&conn, merchant_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
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
                    .map_err(|error| report!(errors::StorageError::from(error)))?;

            is_deleted = is_deleted_func().await?;

            publish_and_redact_merchant_account_cache(self, &merchant_account).await?;
        }

        Ok(is_deleted)
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn list_multiple_merchant_accounts(
        &self,
        merchant_ids: Vec<String>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        let encrypted_merchant_accounts =
            storage::MerchantAccount::list_multiple_merchant_accounts(&conn, merchant_ids)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;

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

    async fn update_all_merchant_account(
        &self,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<usize, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        let db_func = || async {
            storage::MerchantAccount::update_all_merchant_accounts(
                &conn,
                MerchantAccountUpdateInternal::from(merchant_account),
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        let total;
        #[cfg(not(feature = "accounts_cache"))]
        {
            let ma = db_func().await?;
            total = ma.len();
        }

        #[cfg(feature = "accounts_cache")]
        {
            let ma = db_func().await?;
            publish_and_redact_all_merchant_account_cache(self, &ma).await?;
            total = ma.len();
        }

        Ok(total)
    }
}

#[async_trait::async_trait]
impl MerchantAccountInterface for MockDb {
    #[allow(clippy::panic)]
    async fn insert_merchant(
        &self,
        mut merchant_account: domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        merchant_account.id.get_or_insert(
            i32::try_from(accounts.len()).change_context(errors::StorageError::MockDbError)?,
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

    async fn update_merchant(
        &self,
        _this: domain::MerchantAccount,
        _merchant_account: storage::MerchantAccountUpdate,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_specific_fields_in_merchant(
        &self,
        _merchant_id: &str,
        _merchant_account: storage::MerchantAccountUpdate,
        _merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        // [#TODO]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        _publishable_key: &str,
    ) -> CustomResult<authentication::AuthenticationData, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_all_merchant_account(
        &self,
        _merchant_account_update: storage::MerchantAccountUpdate,
    ) -> CustomResult<usize, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        _organization_id: &str,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn list_multiple_merchant_accounts(
        &self,
        _merchant_ids: Vec<String>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}

#[cfg(feature = "accounts_cache")]
async fn publish_and_redact_merchant_account_cache(
    store: &dyn super::StorageInterface,
    merchant_account: &storage::MerchantAccount,
) -> CustomResult<(), errors::StorageError> {
    let publishable_key = merchant_account
        .publishable_key
        .as_ref()
        .map(|publishable_key| CacheKind::Accounts(publishable_key.into()));

    #[cfg(feature = "business_profile_routing")]
    let kgraph_key = merchant_account.default_profile.as_ref().map(|profile_id| {
        CacheKind::CGraph(
            format!(
                "kgraph_{}_{}",
                merchant_account.merchant_id.clone(),
                profile_id,
            )
            .into(),
        )
    });

    #[cfg(not(feature = "business_profile_routing"))]
    let kgraph_key = Some(CacheKind::CGraph(
        format!("kgraph_{}", merchant_account.merchant_id.clone()).into(),
    ));

    let mut cache_keys = vec![CacheKind::Accounts(
        merchant_account.merchant_id.as_str().into(),
    )];

    cache_keys.extend(publishable_key.into_iter());
    cache_keys.extend(kgraph_key.into_iter());

    super::cache::publish_into_redact_channel(store, cache_keys).await?;
    Ok(())
}

#[cfg(feature = "accounts_cache")]
async fn publish_and_redact_all_merchant_account_cache(
    store: &dyn super::StorageInterface,
    merchant_accounts: &[storage::MerchantAccount],
) -> CustomResult<(), errors::StorageError> {
    let merchant_ids = merchant_accounts.iter().map(|m| m.merchant_id.clone());
    let publishable_keys = merchant_accounts
        .iter()
        .filter_map(|m| m.publishable_key.clone());

    let cache_keys: Vec<CacheKind<'_>> = merchant_ids
        .chain(publishable_keys)
        .map(|s| CacheKind::Accounts(s.into()))
        .collect();

    super::cache::publish_into_redact_channel(store, cache_keys).await?;
    Ok(())
}
