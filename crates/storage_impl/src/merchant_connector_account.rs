use async_bb8_diesel::AsyncConnection;
use common_utils::{encryption::Encryption, ext_traits::AsyncExt};
use diesel_models::merchant_connector_account as storage;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_connector_account::{self as domain, MerchantConnectorAccountInterface},
    merchant_key_store::MerchantKeyStore,
};
use router_env::{instrument, tracing};

#[cfg(feature = "accounts_cache")]
use crate::redis::cache;
use crate::{
    kv_router_store,
    utils::{pg_accounts_connection_read, pg_accounts_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore, StorageError,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> MerchantConnectorAccountInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_label: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_merchant_id_connector_label(
                merchant_id,
                connector_label,
                key_store,
            )
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_profile_id_connector_name(
                profile_id,
                connector_name,
                key_store,
            )
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_merchant_id_connector_name(
                merchant_id,
                connector_name,
                key_store,
            )
            .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                merchant_id,
                merchant_connector_id,
                key_store,
            )
            .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn find_merchant_connector_account_by_id(
        &self,
        id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_id(id, key_store)
            .await
    }

    #[instrument(skip_all)]
    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .insert_merchant_connector_account(t, key_store)
            .await
    }

    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        self.router_store
            .list_enabled_connector_accounts_by_profile_id(profile_id, key_store, connector_type)
            .await
    }

    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccounts, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                merchant_id,
                get_disabled,
                key_store,
            )
            .await
    }

    #[instrument(skip_all)]
    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        self.router_store
            .list_connector_account_by_profile_id(profile_id, key_store)
            .await
    }

    #[instrument(skip_all)]
    async fn update_multiple_merchant_connector_accounts(
        &self,
        merchant_connector_accounts: Vec<(
            domain::MerchantConnectorAccount,
            storage::MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), Self::Error> {
        self.router_store
            .update_multiple_merchant_connector_accounts(merchant_connector_accounts)
            .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .update_merchant_connector_account(this, merchant_connector_account, key_store)
            .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .update_merchant_connector_account(this, merchant_connector_account, key_store)
            .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        self.router_store
            .delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
                merchant_id,
                merchant_connector_id,
            )
            .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        self.router_store
            .delete_merchant_connector_account_by_id(id)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> MerchantConnectorAccountInterface for RouterStore<T> {
    type Error = StorageError;
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_label: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let find_call = || async {
            let conn = pg_accounts_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_merchant_id_connector(
                &conn,
                merchant_id,
                connector_label,
            )
            .await
            .map_err(|error| report!(Self::Error::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    merchant_id.clone().into(),
                )
                .await
                .change_context(Self::Error::DeserializationFailed)
        }

        #[cfg(feature = "accounts_cache")]
        {
            cache::get_or_populate_in_memory(
                self,
                &format!("{}_{}", merchant_id.get_string_repr(), connector_label),
                find_call,
                &cache::ACCOUNTS_CACHE,
            )
            .await
            .async_and_then(|item| async {
                item.convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(Self::Error::DecryptionError)
            })
            .await
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let find_call = || async {
            let conn = pg_accounts_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_profile_id_connector_name(
                &conn,
                profile_id,
                connector_name,
            )
            .await
            .map_err(|error| report!(Self::Error::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(Self::Error::DeserializationFailed)
        }

        #[cfg(feature = "accounts_cache")]
        {
            cache::get_or_populate_in_memory(
                self,
                &format!("{}_{}", profile_id.get_string_repr(), connector_name),
                find_call,
                &cache::ACCOUNTS_CACHE,
            )
            .await
            .async_and_then(|item| async {
                item.convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(Self::Error::DecryptionError)
            })
            .await
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id_connector_name(
            &conn,
            merchant_id,
            connector_name,
        )
        .await
        .map_err(|error| report!(Self::Error::from(error)))
        .async_and_then(|items| async {
            let mut output = Vec::with_capacity(items.len());
            for item in items.into_iter() {
                output.push(
                    item.convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(Self::Error::DecryptionError)?,
                )
            }
            Ok(output)
        })
        .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let find_call = || async {
            let conn = pg_accounts_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
                &conn,
                merchant_id,
                merchant_connector_id,
            )
            .await
            .map_err(|error| report!(Self::Error::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(Self::Error::DecryptionError)
        }

        #[cfg(feature = "accounts_cache")]
        {
            cache::get_or_populate_in_memory(
                self,
                &format!(
                    "{}_{}",
                    merchant_id.get_string_repr(),
                    merchant_connector_id.get_string_repr()
                ),
                find_call,
                &cache::ACCOUNTS_CACHE,
            )
            .await?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(Self::Error::DecryptionError)
        }
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn find_merchant_connector_account_by_id(
        &self,
        id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let find_call = || async {
            let conn = pg_accounts_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_id(&conn, id)
                .await
                .map_err(|error| report!(Self::Error::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone(),
                )
                .await
                .change_context(Self::Error::DecryptionError)
        }

        #[cfg(feature = "accounts_cache")]
        {
            cache::get_or_populate_in_memory(
                self,
                id.get_string_repr(),
                find_call,
                &cache::ACCOUNTS_CACHE,
            )
            .await?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key_store.key.get_inner(),
                common_utils::types::keymanager::Identifier::Merchant(
                    key_store.merchant_id.clone(),
                ),
            )
            .await
            .change_context(Self::Error::DecryptionError)
        }
    }

    #[instrument(skip_all)]
    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;
        t.construct_new()
            .await
            .change_context(Self::Error::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(Self::Error::from(error)))
            .async_and_then(|item| async {
                item.convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(Self::Error::DecryptionError)
            })
            .await
    }

    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;

        storage::MerchantConnectorAccount::list_enabled_by_profile_id(
            &conn,
            profile_id,
            connector_type,
        )
        .await
        .map_err(|error| report!(Self::Error::from(error)))
        .async_and_then(|items| async {
            let mut output = Vec::with_capacity(items.len());
            for item in items.into_iter() {
                output.push(
                    item.convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(Self::Error::DecryptionError)?,
                )
            }
            Ok(output)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccounts, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        let merchant_connector_account_vec =
            storage::MerchantConnectorAccount::find_by_merchant_id(
                &conn,
                merchant_id,
                get_disabled,
            )
            .await
            .map_err(|error| report!(Self::Error::from(error)))
            .async_and_then(|items| async {
                let mut output = Vec::with_capacity(items.len());
                for item in items.into_iter() {
                    output.push(
                        item.convert(
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(Self::Error::DecryptionError)?,
                    )
                }
                Ok(output)
            })
            .await?;
        Ok(domain::MerchantConnectorAccounts::new(
            merchant_connector_account_vec,
        ))
    }

    #[instrument(skip_all)]
    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        storage::MerchantConnectorAccount::list_by_profile_id(&conn, profile_id)
            .await
            .map_err(|error| report!(Self::Error::from(error)))
            .async_and_then(|items| async {
                let mut output = Vec::with_capacity(items.len());
                for item in items.into_iter() {
                    output.push(
                        item.convert(
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(Self::Error::DecryptionError)?,
                    )
                }
                Ok(output)
            })
            .await
    }

    #[instrument(skip_all)]
    async fn update_multiple_merchant_connector_accounts(
        &self,
        merchant_connector_accounts: Vec<(
            domain::MerchantConnectorAccount,
            storage::MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;

        async fn update_call(
            connection: &diesel_models::PgPooledConn,
            (merchant_connector_account, mca_update): (
                domain::MerchantConnectorAccount,
                storage::MerchantConnectorAccountUpdateInternal,
            ),
        ) -> Result<(), error_stack::Report<StorageError>> {
            Conversion::convert(merchant_connector_account)
                .await
                .change_context(StorageError::EncryptionError)?
                .update(connection, mca_update)
                .await
                .map_err(|error| report!(StorageError::from(error)))?;
            Ok(())
        }

        conn.transaction_async(|connection_pool| async move {
            for (merchant_connector_account, update_merchant_connector_account) in
                merchant_connector_accounts
            {
                #[cfg(feature = "v1")]
                let _connector_name = merchant_connector_account.connector_name.clone();

                #[cfg(feature = "v2")]
                let _connector_name = merchant_connector_account.connector_name.to_string();

                let _profile_id = merchant_connector_account.profile_id.clone();

                let _merchant_id = merchant_connector_account.merchant_id.clone();
                let _merchant_connector_id = merchant_connector_account.get_id().clone();

                let update = update_call(
                    &connection_pool,
                    (
                        merchant_connector_account,
                        update_merchant_connector_account,
                    ),
                );

                #[cfg(feature = "accounts_cache")]
                // Redact all caches as any of might be used because of backwards compatibility
                Box::pin(cache::publish_and_redact_multiple(
                    self,
                    [
                        cache::CacheKind::Accounts(
                            format!("{}_{}", _profile_id.get_string_repr(), _connector_name).into(),
                        ),
                        cache::CacheKind::Accounts(
                            format!(
                                "{}_{}",
                                _merchant_id.get_string_repr(),
                                _merchant_connector_id.get_string_repr()
                            )
                            .into(),
                        ),
                        cache::CacheKind::CGraph(
                            format!(
                                "cgraph_{}_{}",
                                _merchant_id.get_string_repr(),
                                _profile_id.get_string_repr()
                            )
                            .into(),
                        ),
                    ],
                    || update,
                ))
                .await
                .map_err(|error| {
                    // Returning `DatabaseConnectionError` after logging the actual error because
                    // -> it is not possible to get the underlying from `error_stack::Report<C>`
                    // -> it is not possible to write a `From` impl to convert the `diesel::result::Error` to `error_stack::Report<StorageError>`
                    //    because of Rust's orphan rules
                    router_env::logger::error!(
                        ?error,
                        "DB transaction for updating multiple merchant connector account failed"
                    );
                    Self::Error::DatabaseConnectionError
                })?;

                #[cfg(not(feature = "accounts_cache"))]
                {
                    update.await.map_err(|error| {
                        // Returning `DatabaseConnectionError` after logging the actual error because
                        // -> it is not possible to get the underlying from `error_stack::Report<C>`
                        // -> it is not possible to write a `From` impl to convert the `diesel::result::Error` to `error_stack::Report<StorageError>`
                        //    because of Rust's orphan rules
                        router_env::logger::error!(
                            ?error,
                            "DB transaction for updating multiple merchant connector account failed"
                        );
                        Self::Error::DatabaseConnectionError
                    })?;
                }
            }
            Ok::<_, Self::Error>(())
        })
        .await?;
        Ok(())
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let _connector_name = this.connector_name.clone();
        let _profile_id = this.profile_id.clone();

        let _merchant_id = this.merchant_id.clone();
        let _merchant_connector_id = this.merchant_connector_id.clone();

        let update_call = || async {
            let conn = pg_accounts_connection_write(self).await?;
            Conversion::convert(this)
                .await
                .change_context(Self::Error::EncryptionError)?
                .update(&conn, merchant_connector_account)
                .await
                .map_err(|error| report!(Self::Error::from(error)))
                .async_and_then(|item| async {
                    item.convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(Self::Error::DecryptionError)
                })
                .await
        };

        #[cfg(feature = "accounts_cache")]
        {
            // Redact all caches as any of might be used because of backwards compatibility
            cache::publish_and_redact_multiple(
                self,
                [
                    cache::CacheKind::Accounts(
                        format!("{}_{}", _profile_id.get_string_repr(), _connector_name).into(),
                    ),
                    cache::CacheKind::Accounts(
                        format!(
                            "{}_{}",
                            _merchant_id.get_string_repr(),
                            _merchant_connector_id.get_string_repr()
                        )
                        .into(),
                    ),
                    cache::CacheKind::CGraph(
                        format!(
                            "cgraph_{}_{}",
                            _merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
                    ),
                    cache::CacheKind::PmFiltersCGraph(
                        format!(
                            "pm_filters_cgraph_{}_{}",
                            _merchant_id.get_string_repr(),
                            _profile_id.get_string_repr(),
                        )
                        .into(),
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

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let _connector_name = this.connector_name;
        let _profile_id = this.profile_id.clone();

        let _merchant_id = this.merchant_id.clone();
        let _merchant_connector_id = this.get_id().clone();

        let update_call = || async {
            let conn = pg_accounts_connection_write(self).await?;
            Conversion::convert(this)
                .await
                .change_context(Self::Error::EncryptionError)?
                .update(&conn, merchant_connector_account)
                .await
                .map_err(|error| report!(Self::Error::from(error)))
                .async_and_then(|item| async {
                    item.convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        common_utils::types::keymanager::Identifier::Merchant(
                            key_store.merchant_id.clone(),
                        ),
                    )
                    .await
                    .change_context(Self::Error::DecryptionError)
                })
                .await
        };

        #[cfg(feature = "accounts_cache")]
        {
            // Redact all caches as any of might be used because of backwards compatibility
            cache::publish_and_redact_multiple(
                self,
                [
                    cache::CacheKind::Accounts(
                        format!("{}_{}", _profile_id.get_string_repr(), _connector_name).into(),
                    ),
                    cache::CacheKind::Accounts(
                        _merchant_connector_id.get_string_repr().to_string().into(),
                    ),
                    cache::CacheKind::CGraph(
                        format!(
                            "cgraph_{}_{}",
                            _merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
                    ),
                    cache::CacheKind::PmFiltersCGraph(
                        format!(
                            "pm_filters_cgraph_{}_{}",
                            _merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
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

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;
        let delete_call = || async {
            storage::MerchantConnectorAccount::delete_by_merchant_id_merchant_connector_id(
                &conn,
                merchant_id,
                merchant_connector_id,
            )
            .await
            .map_err(|error| report!(Self::Error::from(error)))
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
            .map_err(|error| report!(Self::Error::from(error)))?;

            let _profile_id = mca
                .profile_id
                .ok_or(Self::Error::ValueNotFound("profile_id".to_string()))?;

            cache::publish_and_redact_multiple(
                self,
                [
                    cache::CacheKind::Accounts(
                        format!(
                            "{}_{}",
                            mca.merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
                    ),
                    cache::CacheKind::CGraph(
                        format!(
                            "cgraph_{}_{}",
                            mca.merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
                    ),
                    cache::CacheKind::PmFiltersCGraph(
                        format!(
                            "pm_filters_cgraph_{}_{}",
                            mca.merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
                    ),
                ],
                delete_call,
            )
            .await
        }

        #[cfg(not(feature = "accounts_cache"))]
        {
            delete_call().await
        }
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;
        let delete_call = || async {
            storage::MerchantConnectorAccount::delete_by_id(&conn, id)
                .await
                .map_err(|error| report!(Self::Error::from(error)))
        };

        #[cfg(feature = "accounts_cache")]
        {
            // We need to fetch mca here because the key that's saved in cache in
            // {merchant_id}_{connector_label}.
            // Used function from storage model to reuse the connection that made here instead of
            // creating new.

            let mca = storage::MerchantConnectorAccount::find_by_id(&conn, id)
                .await
                .map_err(|error| report!(Self::Error::from(error)))?;

            let _profile_id = mca.profile_id;

            cache::publish_and_redact_multiple(
                self,
                [
                    cache::CacheKind::Accounts(
                        format!(
                            "{}_{}",
                            mca.merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
                    ),
                    cache::CacheKind::CGraph(
                        format!(
                            "cgraph_{}_{}",
                            mca.merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
                    ),
                    cache::CacheKind::PmFiltersCGraph(
                        format!(
                            "pm_filters_cgraph_{}_{}",
                            mca.merchant_id.get_string_repr(),
                            _profile_id.get_string_repr()
                        )
                        .into(),
                    ),
                ],
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
    type Error = StorageError;
    async fn update_multiple_merchant_connector_accounts(
        &self,
        _merchant_connector_accounts: Vec<(
            domain::MerchantConnectorAccount,
            storage::MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), StorageError> {
        // No need to implement this function for `MockDb` as this function will be removed after the
        // apple pay certificate migration
        Err(StorageError::MockDbError)?
    }
    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, StorageError> {
        match self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .find(|account| {
                account.merchant_id == *merchant_id
                    && account.connector_label == Some(connector.to_string())
            })
            .cloned()
            .async_map(|account| async {
                account
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
        {
            Some(result) => result,
            None => {
                return Err(StorageError::ValueNotFound(
                    "cannot find merchant connector account".to_string(),
                )
                .into())
            }
        }
    }

    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        _profile_id: &common_utils::id_type::ProfileId,
        _key_store: &MerchantKeyStore,
        _connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, StorageError> {
        let accounts = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .filter(|account| {
                account.merchant_id == *merchant_id && account.connector_name == connector_name
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut output = Vec::with_capacity(accounts.len());
        for account in accounts.into_iter() {
            output.push(
                account
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)?,
            )
        }
        Ok(output)
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, StorageError> {
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
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError),
            None => Err(StorageError::ValueNotFound(
                "cannot find merchant connector account".to_string(),
            )
            .into()),
        }
    }

    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, StorageError> {
        match self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .find(|account| {
                account.merchant_id == *merchant_id
                    && account.merchant_connector_id == *merchant_connector_id
            })
            .cloned()
            .async_map(|account| async {
                account
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
        {
            Some(result) => result,
            None => {
                return Err(StorageError::ValueNotFound(
                    "cannot find merchant connector account".to_string(),
                )
                .into())
            }
        }
    }

    #[cfg(feature = "v2")]
    async fn find_merchant_connector_account_by_id(
        &self,
        id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, StorageError> {
        match self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .find(|account| account.get_id() == *id)
            .cloned()
            .async_map(|account| async {
                account
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        common_utils::types::keymanager::Identifier::Merchant(
                            key_store.merchant_id.clone(),
                        ),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
        {
            Some(result) => result,
            None => {
                return Err(StorageError::ValueNotFound(
                    "cannot find merchant connector account".to_string(),
                )
                .into())
            }
        }
    }

    #[cfg(feature = "v1")]
    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        let account = storage::MerchantConnectorAccount {
            merchant_id: t.merchant_id,
            connector_name: t.connector_name,
            connector_account_details: t.connector_account_details.into(),
            test_mode: t.test_mode,
            disabled: t.disabled,
            merchant_connector_id: t.merchant_connector_id.clone(),
            id: Some(t.merchant_connector_id),
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
            profile_id: Some(t.profile_id),
            applepay_verified_domains: t.applepay_verified_domains,
            pm_auth_config: t.pm_auth_config,
            status: t.status,
            connector_wallets_details: t.connector_wallets_details.map(Encryption::from),
            additional_merchant_data: t.additional_merchant_data.map(|data| data.into()),
            version: t.version,
        };
        accounts.push(account.clone());
        account
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v2")]
    async fn insert_merchant_connector_account(
        &self,
        t: domain::MerchantConnectorAccount,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        let account = storage::MerchantConnectorAccount {
            id: t.id,
            merchant_id: t.merchant_id,
            connector_name: t.connector_name,
            connector_account_details: t.connector_account_details.into(),
            disabled: t.disabled,
            payment_methods_enabled: t.payment_methods_enabled,
            metadata: t.metadata,
            frm_config: t.frm_configs,
            connector_type: t.connector_type,
            connector_label: t.connector_label,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            connector_webhook_details: t.connector_webhook_details,
            profile_id: t.profile_id,
            applepay_verified_domains: t.applepay_verified_domains,
            pm_auth_config: t.pm_auth_config,
            status: t.status,
            connector_wallets_details: t.connector_wallets_details.map(Encryption::from),
            additional_merchant_data: t.additional_merchant_data.map(|data| data.into()),
            version: t.version,
            feature_metadata: t.feature_metadata.map(From::from),
        };
        accounts.push(account.clone());
        account
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key_store.key.get_inner(),
                common_utils::types::keymanager::Identifier::Merchant(
                    key_store.merchant_id.clone(),
                ),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccounts, StorageError> {
        let accounts = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .filter(|account: &&storage::MerchantConnectorAccount| {
                if get_disabled {
                    account.merchant_id == *merchant_id
                } else {
                    account.merchant_id == *merchant_id && account.disabled == Some(false)
                }
            })
            .cloned()
            .collect::<Vec<storage::MerchantConnectorAccount>>();

        let mut output = Vec::with_capacity(accounts.len());
        for account in accounts.into_iter() {
            output.push(
                account
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)?,
            )
        }
        Ok(domain::MerchantConnectorAccounts::new(output))
    }

    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, StorageError> {
        let accounts = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .filter(|account: &&storage::MerchantConnectorAccount| {
                account.profile_id == *profile_id
            })
            .cloned()
            .collect::<Vec<storage::MerchantConnectorAccount>>();

        let mut output = Vec::with_capacity(accounts.len());
        for account in accounts.into_iter() {
            output.push(
                account
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)?,
            )
        }
        Ok(output)
    }

    #[cfg(feature = "v1")]
    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, StorageError> {
        let mca_update_res = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter_mut()
            .find(|account| account.merchant_connector_id == this.merchant_connector_id)
            .map(|a| {
                let updated =
                    merchant_connector_account.create_merchant_connector_account(a.clone());
                *a = updated.clone();
                updated
            })
            .async_map(|account| async {
                account
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await;

        match mca_update_res {
            Some(result) => result,
            None => {
                return Err(StorageError::ValueNotFound(
                    "cannot find merchant connector account to update".to_string(),
                )
                .into())
            }
        }
    }

    #[cfg(feature = "v2")]
    async fn update_merchant_connector_account(
        &self,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, StorageError> {
        let mca_update_res = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter_mut()
            .find(|account| account.get_id() == this.get_id())
            .map(|a| {
                let updated =
                    merchant_connector_account.create_merchant_connector_account(a.clone());
                *a = updated.clone();
                updated
            })
            .async_map(|account| async {
                account
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        key_store.key.get_inner(),
                        common_utils::types::keymanager::Identifier::Merchant(
                            key_store.merchant_id.clone(),
                        ),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await;

        match mca_update_res {
            Some(result) => result,
            None => {
                return Err(StorageError::ValueNotFound(
                    "cannot find merchant connector account to update".to_string(),
                )
                .into())
            }
        }
    }

    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        match accounts.iter().position(|account| {
            account.merchant_id == *merchant_id
                && account.merchant_connector_id == *merchant_connector_id
        }) {
            Some(index) => {
                accounts.remove(index);
                return Ok(true);
            }
            None => {
                return Err(StorageError::ValueNotFound(
                    "cannot find merchant connector account to delete".to_string(),
                )
                .into())
            }
        }
    }

    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        match accounts.iter().position(|account| account.get_id() == *id) {
            Some(index) => {
                accounts.remove(index);
                return Ok(true);
            }
            None => {
                return Err(StorageError::ValueNotFound(
                    "cannot find merchant connector account to delete".to_string(),
                )
                .into())
            }
        }
    }
}
