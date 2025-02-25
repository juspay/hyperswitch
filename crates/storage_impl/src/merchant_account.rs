use std::collections::HashMap;

use common_utils::{errors::CustomResult, types::keymanager::KeyManagerState};
use diesel_models::{merchant_account as storage, MerchantAccountUpdateInternal};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_account as domain, merchant_key_store,
};
use router_env::{instrument, tracing};

use sample::merchant_account::MerchantAccountInterface;
use sample::MasterKeyInterface;

#[cfg(feature = "olap")]
use sample::merchant_key_store::MerchantKeyStoreInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> MerchantAccountInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_merchant(
        &self,
        state: &KeyManagerState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
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
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
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
        merchant_account: domain::MerchantAccountUpdate,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
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
        merchant_account: domain::MerchantAccountUpdate,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
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
    ) -> CustomResult<(domain::MerchantAccount, merchant_key_store::MerchantKeyStore), errors::StorageError>
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
        merchant_account: domain::MerchantAccountUpdate,
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
