use async_bb8_diesel::AsyncConnection;
use common_utils::{
    errors::CustomResult, ext_traits::AsyncExt, ext_traits::Encode, ext_traits::ByteSliceExt, types::keymanager::KeyManagerState,
};
use diesel_models::merchant_connector_account as storage;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    router_data,
    merchant_connector_account as domain, merchant_key_store,
};
use router_env::{instrument, tracing};
use sample::merchant_connector_account::{MerchantConnectorAccountInterface, ConnectorAccessToken};

use crate::{connection, errors, DatabaseStore, RouterStore, redis::kv_store::RedisConnInterface};

#[async_trait::async_trait]
impl<T: DatabaseStore> MerchantConnectorAccountInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_label: &str,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_merchant_id_connector(
                &conn,
                merchant_id,
                connector_label,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(state, key_store.key.get_inner(), merchant_id.clone().into())
                .await
                .change_context(errors::StorageError::DeserializationFailed)
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
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .await
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        connector_name: &str,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_profile_id_connector_name(
                &conn,
                profile_id,
                connector_name,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DeserializationFailed)
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
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .await
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id_connector_name(
            &conn,
            merchant_id,
            connector_name,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|items| async {
            let mut output = Vec::with_capacity(items.len());
            for item in items.into_iter() {
                output.push(
                    item.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)?,
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
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
                &conn,
                merchant_id,
                merchant_connector_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
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
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        }
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn find_merchant_connector_account_by_id(
        &self,
        state: &KeyManagerState,
        id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantConnectorAccount::find_by_id(&conn, id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            find_call()
                .await?
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
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
                state,
                key_store.key.get_inner(),
                common_utils::types::keymanager::Identifier::Merchant(
                    key_store.merchant_id.clone(),
                ),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
        }
    }

    #[instrument(skip_all)]
    async fn insert_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        t: domain::MerchantConnectorAccount,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        t.construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|item| async {
                item.convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    #[cfg(all(feature = "oltp", feature = "v2"))]
    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &merchant_key_store::MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;

        storage::MerchantConnectorAccount::list_enabled_by_profile_id(
            &conn,
            profile_id,
            connector_type,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
        .async_and_then(|items| async {
            let mut output = Vec::with_capacity(items.len());
            for item in items.into_iter() {
                output.push(
                    item.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)?,
                )
            }
            Ok(output)
        })
        .await
    }

    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::MerchantConnectorAccount::find_by_merchant_id(&conn, merchant_id, get_disabled)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|items| async {
                let mut output = Vec::with_capacity(items.len());
                for item in items.into_iter() {
                    output.push(
                        item.convert(
                            state,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
                    )
                }
                Ok(output)
            })
            .await
    }

    #[instrument(skip_all)]
    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::MerchantConnectorAccount::list_by_profile_id(&conn, profile_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|items| async {
                let mut output = Vec::with_capacity(items.len());
                for item in items.into_iter() {
                    output.push(
                        item.convert(
                            state,
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)?,
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
    ) -> CustomResult<(), errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;

        async fn update_call(
            connection: &diesel_models::PgPooledConn,
            (merchant_connector_account, mca_update): (
                domain::MerchantConnectorAccount,
                storage::MerchantConnectorAccountUpdateInternal,
            ),
        ) -> Result<(), error_stack::Report<errors::StorageError>> {
            Conversion::convert(merchant_connector_account)
                .await
                .change_context(errors::StorageError::EncryptionError)?
                .update(connection, mca_update)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;
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
                    errors::StorageError::DatabaseConnectionError
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
                        errors::StorageError::DatabaseConnectionError
                    })?;
                }
            }
            Ok::<_, errors::StorageError>(())
        })
        .await?;
        Ok(())
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn update_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let _connector_name = this.connector_name.clone();
        let _profile_id = this.profile_id.clone();

        let _merchant_id = this.merchant_id.clone();
        let _merchant_connector_id = this.merchant_connector_id.clone();

        let update_call = || async {
            let conn = connection::pg_connection_write(self).await?;
            Conversion::convert(this)
                .await
                .change_context(errors::StorageError::EncryptionError)?
                .update(&conn, merchant_connector_account)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
                .async_and_then(|item| async {
                    item.convert(
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
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
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let _connector_name = this.connector_name;
        let _profile_id = this.profile_id.clone();

        let _merchant_id = this.merchant_id.clone();
        let _merchant_connector_id = this.get_id().clone();

        let update_call = || async {
            let conn = connection::pg_connection_write(self).await?;
            Conversion::convert(this)
                .await
                .change_context(errors::StorageError::EncryptionError)?
                .update(&conn, merchant_connector_account)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
                .async_and_then(|item| async {
                    item.convert(
                        state,
                        key_store.key.get_inner(),
                        common_utils::types::keymanager::Identifier::Merchant(
                            key_store.merchant_id.clone(),
                        ),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
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
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let delete_call = || async {
            storage::MerchantConnectorAccount::delete_by_merchant_id_merchant_connector_id(
                &conn,
                merchant_id,
                merchant_connector_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
            .map_err(|error| report!(errors::StorageError::from(error)))?;

            let _profile_id = mca.profile_id.ok_or(errors::StorageError::ValueNotFound(
                "profile_id".to_string(),
            ))?;

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
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let delete_call = || async {
            storage::MerchantConnectorAccount::delete_by_id(&conn, id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(feature = "accounts_cache")]
        {
            // We need to fetch mca here because the key that's saved in cache in
            // {merchant_id}_{connector_label}.
            // Used function from storage model to reuse the connection that made here instead of
            // creating new.

            let mca = storage::MerchantConnectorAccount::find_by_id(&conn, id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;

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
impl<T: DatabaseStore> ConnectorAccessToken for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn get_access_token(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id_or_connector_name: &str,
    ) -> CustomResult<Option<router_data::AccessToken>, errors::StorageError> {
        //TODO: Handle race condition
        // This function should acquire a global lock on some resource, if access token is already
        // being refreshed by other request then wait till it finishes and use the same access token
        let key = common_utils::access_token::create_access_token_key(
            merchant_id,
            merchant_connector_id_or_connector_name,
        );

        let maybe_token = self
            .get_redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .get_key::<Option<Vec<u8>>>(&key.into())
            .await
            .change_context(errors::StorageError::KVError)
            .attach_printable("DB error when getting access token")?;

        let access_token = maybe_token
            .map(|token| token.parse_struct::<router_data::AccessToken>("router_data::AccessToken"))
            .transpose()
            .change_context(errors::StorageError::DeserializationFailed)?;

        Ok(access_token)
    }

    #[instrument(skip_all)]
    async fn set_access_token(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id_or_connector_name: &str,
        access_token: router_data::AccessToken,
    ) -> CustomResult<(), errors::StorageError> {
        let key = common_utils::access_token::create_access_token_key(
            merchant_id,
            merchant_connector_id_or_connector_name,
        );
        let serialized_access_token = access_token
            .encode_to_string_of_json()
            .change_context(errors::StorageError::SerializationFailed)?;
        self.get_redis_conn()
            .map_err(Into::<errors::StorageError>::into)?
            .set_key_with_expiry(&key.into(), serialized_access_token, access_token.expires)
            .await
            .change_context(errors::StorageError::KVError)
    }
}