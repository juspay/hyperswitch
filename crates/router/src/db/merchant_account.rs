#[cfg(feature = "olap")]
use std::collections::HashMap;

use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
use diesel_models::MerchantAccountUpdateInternal;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
#[cfg(feature = "accounts_cache")]
use storage_impl::redis::cache::{self, CacheKind, ACCOUNTS_CACHE};

use super::{MasterKeyInterface, MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::merchant_key_store::MerchantKeyStoreInterface,
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
        state: &KeyManagerState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn update_all_merchant_account(
        &self,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<usize, errors::StorageError>;

    async fn update_merchant(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn update_specific_fields_in_merchant(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_account: storage::MerchantAccountUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_publishable_key(
        &self,
        state: &KeyManagerState,
        publishable_key: &str,
    ) -> CustomResult<(domain::MerchantAccount, domain::MerchantKeyStore), errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        state: &KeyManagerState,
        organization_id: &common_utils::id_type::OrganizationId,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError>;

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn list_multiple_merchant_accounts(
        &self,
        state: &KeyManagerState,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn list_merchant_and_org_ids(
        &self,
        state: &KeyManagerState,
        limit: u32,
        offset: Option<u32>,
    ) -> CustomResult<
        Vec<(
            common_utils::id_type::MerchantId,
            common_utils::id_type::OrganizationId,
        )>,
        errors::StorageError,
    >;
}

#[async_trait::async_trait]
impl MerchantAccountInterface for Store {
    #[instrument(skip_all)]
    async fn insert_merchant(
        &self,
        state: &KeyManagerState,
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
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_merchant_account_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
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
                .convert(
                    state,
                    merchant_key_store.key.get_inner(),
                    merchant_id.to_owned().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(feature = "accounts_cache")]
        {
            cache::get_or_populate_in_memory(
                self,
                merchant_id.get_string_repr(),
                fetch_func,
                &ACCOUNTS_CACHE,
            )
            .await?
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        }
    }

    #[instrument(skip_all)]
    async fn update_merchant(
        &self,
        state: &KeyManagerState,
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
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn update_specific_fields_in_merchant(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
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
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_merchant_account_by_publishable_key(
        &self,
        state: &KeyManagerState,
        publishable_key: &str,
    ) -> CustomResult<(domain::MerchantAccount, domain::MerchantKeyStore), errors::StorageError>
    {
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
            merchant_account = cache::get_or_populate_in_memory(
                self,
                publishable_key,
                fetch_by_pub_key_func,
                &ACCOUNTS_CACHE,
            )
            .await?;
        }
        let key_store = self
            .get_merchant_key_store_by_merchant_id(
                state,
                merchant_account.get_id(),
                &self.get_master_key().to_vec().into(),
            )
            .await?;
        let domain_merchant_account = merchant_account
            .convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        Ok((domain_merchant_account, key_store))
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        state: &KeyManagerState,
        organization_id: &common_utils::id_type::OrganizationId,
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
                    state,
                    merchant_account.get_id(),
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
                        .convert(
                            state,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
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
        merchant_id: &common_utils::id_type::MerchantId,
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
        state: &KeyManagerState,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        let encrypted_merchant_accounts =
            storage::MerchantAccount::list_multiple_merchant_accounts(&conn, merchant_ids)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;

        let db_master_key = self.get_master_key().to_vec().into();

        let merchant_key_stores = self
            .list_multiple_key_stores(
                state,
                encrypted_merchant_accounts
                    .iter()
                    .map(|merchant_account| merchant_account.get_id())
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
                    let key_store = key_stores_by_id.get(merchant_account.get_id()).ok_or(
                        errors::StorageError::ValueNotFound(format!(
                            "merchant_key_store with merchant_id = {:?}",
                            merchant_account.get_id()
                        )),
                    )?;
                    merchant_account
                        .convert(
                            state,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                },
            ))
            .await?;

        Ok(merchant_accounts)
    }

    #[cfg(feature = "olap")]
    #[instrument(skip_all)]
    async fn list_merchant_and_org_ids(
        &self,
        _state: &KeyManagerState,
        limit: u32,
        offset: Option<u32>,
    ) -> CustomResult<
        Vec<(
            common_utils::id_type::MerchantId,
            common_utils::id_type::OrganizationId,
        )>,
        errors::StorageError,
    > {
        let conn = connection::pg_connection_read(self).await?;
        let encrypted_merchant_accounts =
            storage::MerchantAccount::list_all_merchant_accounts(&conn, limit, offset)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;

        let merchant_and_org_ids = encrypted_merchant_accounts
            .into_iter()
            .map(|merchant_account| {
                let merchant_id = merchant_account.get_id().clone();
                let org_id = merchant_account.organization_id;
                (merchant_id, org_id)
            })
            .collect();
        Ok(merchant_and_org_ids)
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
        state: &KeyManagerState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        let account = Conversion::convert(merchant_account)
            .await
            .change_context(errors::StorageError::EncryptionError)?;
        accounts.push(account.clone());

        account
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[allow(clippy::panic)]
    async fn find_merchant_account_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let accounts = self.merchant_accounts.lock().await;
        accounts
            .iter()
            .find(|account| account.get_id() == merchant_id)
            .cloned()
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "Merchant ID: {:?} not found",
                merchant_id
            )))?
            .convert(
                state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn update_merchant(
        &self,
        state: &KeyManagerState,
        merchant_account: domain::MerchantAccount,
        merchant_account_update: storage::MerchantAccountUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let merchant_id = merchant_account.get_id().to_owned();
        let mut accounts = self.merchant_accounts.lock().await;
        accounts
            .iter_mut()
            .find(|account| account.get_id() == merchant_account.get_id())
            .async_map(|account| async {
                let update = MerchantAccountUpdateInternal::from(merchant_account_update)
                    .apply_changeset(
                        Conversion::convert(merchant_account)
                            .await
                            .change_context(errors::StorageError::EncryptionError)?,
                    );
                *account = update.clone();
                update
                    .convert(
                        state,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "Merchant ID: {:?} not found",
                    merchant_id
                ))
                .into(),
            )
    }

    async fn update_specific_fields_in_merchant(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_account_update: storage::MerchantAccountUpdate,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        accounts
            .iter_mut()
            .find(|account| account.get_id() == merchant_id)
            .async_map(|account| async {
                let update = MerchantAccountUpdateInternal::from(merchant_account_update)
                    .apply_changeset(account.clone());
                *account = update.clone();
                update
                    .convert(
                        state,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "Merchant ID: {:?} not found",
                    merchant_id
                ))
                .into(),
            )
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        state: &KeyManagerState,
        publishable_key: &str,
    ) -> CustomResult<(domain::MerchantAccount, domain::MerchantKeyStore), errors::StorageError>
    {
        let accounts = self.merchant_accounts.lock().await;
        let account = accounts
            .iter()
            .find(|account| {
                account
                    .publishable_key
                    .as_ref()
                    .is_some_and(|key| key == publishable_key)
            })
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "Publishable Key: {} not found",
                publishable_key
            )))?;
        let key_store = self
            .get_merchant_key_store_by_merchant_id(
                state,
                account.get_id(),
                &self.get_master_key().to_vec().into(),
            )
            .await?;
        let merchant_account = account
            .clone()
            .convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)?;
        Ok((merchant_account, key_store))
    }

    async fn update_all_merchant_account(
        &self,
        merchant_account_update: storage::MerchantAccountUpdate,
    ) -> CustomResult<usize, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        Ok(accounts.iter_mut().fold(0, |acc, account| {
            let update = MerchantAccountUpdateInternal::from(merchant_account_update.clone())
                .apply_changeset(account.clone());
            *account = update;
            acc + 1
        }))
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        accounts.retain(|x| x.get_id() != merchant_id);
        Ok(true)
    }

    #[cfg(feature = "olap")]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        state: &KeyManagerState,
        organization_id: &common_utils::id_type::OrganizationId,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        let accounts = self.merchant_accounts.lock().await;
        let futures = accounts
            .iter()
            .filter(|account| account.organization_id == *organization_id)
            .map(|account| async {
                let key_store = self
                    .get_merchant_key_store_by_merchant_id(
                        state,
                        account.get_id(),
                        &self.get_master_key().to_vec().into(),
                    )
                    .await;
                match key_store {
                    Ok(key) => account
                        .clone()
                        .convert(state, key.key.get_inner(), key.merchant_id.clone().into())
                        .await
                        .change_context(errors::StorageError::DecryptionError),
                    Err(err) => Err(err),
                }
            });
        futures::future::join_all(futures)
            .await
            .into_iter()
            .collect()
    }

    #[cfg(feature = "olap")]
    async fn list_multiple_merchant_accounts(
        &self,
        state: &KeyManagerState,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
        let accounts = self.merchant_accounts.lock().await;
        let futures = accounts
            .iter()
            .filter(|account| merchant_ids.contains(account.get_id()))
            .map(|account| async {
                let key_store = self
                    .get_merchant_key_store_by_merchant_id(
                        state,
                        account.get_id(),
                        &self.get_master_key().to_vec().into(),
                    )
                    .await;
                match key_store {
                    Ok(key) => account
                        .clone()
                        .convert(state, key.key.get_inner(), key.merchant_id.clone().into())
                        .await
                        .change_context(errors::StorageError::DecryptionError),
                    Err(err) => Err(err),
                }
            });
        futures::future::join_all(futures)
            .await
            .into_iter()
            .collect()
    }

    #[cfg(feature = "olap")]
    async fn list_merchant_and_org_ids(
        &self,
        _state: &KeyManagerState,
        limit: u32,
        offset: Option<u32>,
    ) -> CustomResult<
        Vec<(
            common_utils::id_type::MerchantId,
            common_utils::id_type::OrganizationId,
        )>,
        errors::StorageError,
    > {
        let accounts = self.merchant_accounts.lock().await;
        let limit = limit.try_into().unwrap_or(accounts.len());
        let offset = offset.unwrap_or(0).try_into().unwrap_or(0);

        let merchant_and_org_ids = accounts
            .iter()
            .skip(offset)
            .take(limit)
            .map(|account| (account.get_id().clone(), account.organization_id.clone()))
            .collect::<Vec<_>>();

        Ok(merchant_and_org_ids)
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

    #[cfg(feature = "v1")]
    let cgraph_key = merchant_account.default_profile.as_ref().map(|profile_id| {
        CacheKind::CGraph(
            format!(
                "cgraph_{}_{}",
                merchant_account.get_id().get_string_repr(),
                profile_id.get_string_repr(),
            )
            .into(),
        )
    });

    // TODO: we will not have default profile in v2
    #[cfg(feature = "v2")]
    let cgraph_key = None;

    let mut cache_keys = vec![CacheKind::Accounts(
        merchant_account.get_id().get_string_repr().into(),
    )];

    cache_keys.extend(publishable_key.into_iter());
    cache_keys.extend(cgraph_key.into_iter());

    cache::redact_from_redis_and_publish(store.get_cache_store().as_ref(), cache_keys).await?;
    Ok(())
}

#[cfg(feature = "accounts_cache")]
async fn publish_and_redact_all_merchant_account_cache(
    store: &dyn super::StorageInterface,
    merchant_accounts: &[storage::MerchantAccount],
) -> CustomResult<(), errors::StorageError> {
    let merchant_ids = merchant_accounts
        .iter()
        .map(|merchant_account| merchant_account.get_id().get_string_repr().to_string());
    let publishable_keys = merchant_accounts
        .iter()
        .filter_map(|m| m.publishable_key.clone());

    let cache_keys: Vec<CacheKind<'_>> = merchant_ids
        .chain(publishable_keys)
        .map(|s| CacheKind::Accounts(s.into()))
        .collect();

    cache::redact_from_redis_and_publish(store.get_cache_store().as_ref(), cache_keys).await?;
    Ok(())
}
