use async_bb8_diesel::AsyncConnection;
use common_utils::{
    encryption::Encryption,
    ext_traits::{AsyncExt, ByteSliceExt, Encode},
    types::keymanager::KeyManagerState,
};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};
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
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id_or_connector_name: &str,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError>;

    async fn set_access_token(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id_or_connector_name: &str,
        access_token: types::AccessToken,
    ) -> CustomResult<(), errors::StorageError>;
}

#[async_trait::async_trait]
impl ConnectorAccessToken for Store {
    #[instrument(skip_all)]
    async fn get_access_token(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id_or_connector_name: &str,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError> {
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
            .map(|token| token.parse_struct::<types::AccessToken>("AccessToken"))
            .transpose()
            .change_context(errors::StorageError::DeserializationFailed)?;

        Ok(access_token)
    }

    #[instrument(skip_all)]
    async fn set_access_token(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id_or_connector_name: &str,
        access_token: types::AccessToken,
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

#[async_trait::async_trait]
impl ConnectorAccessToken for MockDb {
    async fn get_access_token(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _merchant_connector_id_or_connector_name: &str,
    ) -> CustomResult<Option<types::AccessToken>, errors::StorageError> {
        Ok(None)
    }

    async fn set_access_token(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        _merchant_connector_id_or_connector_name: &str,
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
    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_label: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError>;

    async fn insert_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    #[cfg(feature = "v2")]
    async fn find_merchant_connector_account_by_id(
        &self,
        state: &KeyManagerState,
        id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError>;

    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError>;

    #[cfg(all(feature = "oltp", feature = "v2"))]
    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &domain::MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError>;

    async fn update_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError>;

    async fn update_multiple_merchant_connector_accounts(
        &self,
        this: Vec<(
            domain::MerchantConnectorAccount,
            storage::MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), errors::StorageError>;

    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, errors::StorageError>;

    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &common_utils::id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl MerchantConnectorAccountInterface for Store {
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
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
        key_store: &domain::MerchantKeyStore,
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
        key_store: &domain::MerchantKeyStore,
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
        key_store: &domain::MerchantKeyStore,
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
        key_store: &domain::MerchantKeyStore,
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
        key_store: &domain::MerchantKeyStore,
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
        key_store: &domain::MerchantKeyStore,
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
        ) -> Result<(), error_stack::Report<storage_impl::errors::StorageError>> {
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
        key_store: &domain::MerchantKeyStore,
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
        key_store: &domain::MerchantKeyStore,
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
impl MerchantConnectorAccountInterface for MockDb {
    async fn update_multiple_merchant_connector_accounts(
        &self,
        _merchant_connector_accounts: Vec<(
            domain::MerchantConnectorAccount,
            storage::MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), errors::StorageError> {
        // No need to implement this function for `MockDb` as this function will be removed after the
        // apple pay certificate migration
        Err(errors::StorageError::MockDbError)?
    }
    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        connector: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
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
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
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

    #[cfg(all(feature = "oltp", feature = "v2"))]
    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &domain::MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
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
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)?,
            )
        }
        Ok(output)
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
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
                .convert(
                    state,
                    key_store.key.get_inner(),
                    key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError),
            None => Err(errors::StorageError::ValueNotFound(
                "cannot find merchant connector account".to_string(),
            )
            .into()),
        }
    }

    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
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
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
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

    #[cfg(feature = "v2")]
    async fn find_merchant_connector_account_by_id(
        &self,
        state: &KeyManagerState,
        id: &common_utils::id_type::MerchantConnectorAccountId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
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

    #[cfg(feature = "v1")]
    async fn insert_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        let account = storage::MerchantConnectorAccount {
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
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[cfg(feature = "v2")]
    async fn insert_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        t: domain::MerchantConnectorAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
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
                state,
                key_store.key.get_inner(),
                common_utils::types::keymanager::Identifier::Merchant(
                    key_store.merchant_id.clone(),
                ),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
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
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)?,
            )
        }
        Ok(output)
    }

    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, errors::StorageError> {
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
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)?,
            )
        }
        Ok(output)
    }

    #[cfg(feature = "v1")]
    async fn update_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
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
                        state,
                        key_store.key.get_inner(),
                        key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await;

        match mca_update_res {
            Some(result) => result,
            None => {
                return Err(errors::StorageError::ValueNotFound(
                    "cannot find merchant connector account to update".to_string(),
                )
                .into())
            }
        }
    }

    #[cfg(feature = "v2")]
    async fn update_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: storage::MerchantConnectorAccountUpdateInternal,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, errors::StorageError> {
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
                        state,
                        key_store.key.get_inner(),
                        common_utils::types::keymanager::Identifier::Merchant(
                            key_store.merchant_id.clone(),
                        ),
                    )
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await;

        match mca_update_res {
            Some(result) => result,
            None => {
                return Err(errors::StorageError::ValueNotFound(
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
    ) -> CustomResult<bool, errors::StorageError> {
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
                return Err(errors::StorageError::ValueNotFound(
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
    ) -> CustomResult<bool, errors::StorageError> {
        let mut accounts = self.merchant_connector_accounts.lock().await;
        match accounts.iter().position(|account| account.get_id() == *id) {
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

#[cfg(feature = "accounts_cache")]
#[cfg(test)]
mod merchant_connector_account_cache_tests {
    use std::sync::Arc;

    #[cfg(feature = "v1")]
    use api_models::enums::CountryAlpha2;
    use common_utils::{date_time, type_name, types::keymanager::Identifier};
    use diesel_models::enums::ConnectorType;
    use error_stack::ResultExt;
    use masking::PeekInterface;
    use storage_impl::redis::{
        cache::{self, CacheKey, CacheKind, ACCOUNTS_CACHE},
        kv_store::RedisConnInterface,
        pub_sub::PubSubInterface,
    };
    use time::macros::datetime;
    use tokio::sync::oneshot;

    use crate::{
        core::errors,
        db::{
            merchant_connector_account::MerchantConnectorAccountInterface,
            merchant_key_store::MerchantKeyStoreInterface, MasterKeyInterface, MockDb,
        },
        routes::{
            self,
            app::{settings::Settings, StorageImpl},
        },
        services,
        types::{
            domain::{self, behaviour::Conversion},
            storage,
        },
    };

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    #[cfg(feature = "v1")]
    async fn test_connector_profile_id_cache() {
        let conf = Settings::new().unwrap();
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let app_state = Box::pin(routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        ))
        .await;

        let state = &Arc::new(app_state)
            .get_session_state(
                &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
                None,
                || {},
            )
            .unwrap();
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

        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("test_merchant"))
                .unwrap();

        let connector_label = "stripe_USA";
        let merchant_connector_id = "simple_merchant_connector_id";
        let profile_id =
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from("pro_max_ultra"))
                .unwrap();
        let key_manager_state = &state.into();
        db.insert_merchant_key_store(
            key_manager_state,
            domain::MerchantKeyStore {
                merchant_id: merchant_id.clone(),
                key: domain::types::crypto_operation(
                    key_manager_state,
                    type_name!(domain::MerchantKeyStore),
                    domain::types::CryptoOperation::Encrypt(
                        services::generate_aes256_key().unwrap().to_vec().into(),
                    ),
                    Identifier::Merchant(merchant_id.clone()),
                    master_key,
                )
                .await
                .and_then(|val| val.try_into_operation())
                .unwrap(),
                created_at: datetime!(2023-02-01 0:00),
            },
            &master_key.to_vec().into(),
        )
        .await
        .unwrap();

        let merchant_key = db
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &merchant_id,
                &master_key.to_vec().into(),
            )
            .await
            .unwrap();

        let mca = domain::MerchantConnectorAccount {
            merchant_id: merchant_id.to_owned(),
            connector_name: "stripe".to_string(),
            connector_account_details: domain::types::crypto_operation(
                key_manager_state,
                type_name!(domain::MerchantConnectorAccount),
                domain::types::CryptoOperation::Encrypt(serde_json::Value::default().into()),
                Identifier::Merchant(merchant_key.merchant_id.clone()),
                merchant_key.key.get_inner().peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .unwrap(),
            test_mode: None,
            disabled: None,
            merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId::wrap(
                merchant_connector_id.to_string(),
            )
            .unwrap(),
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
            profile_id: profile_id.to_owned(),
            applepay_verified_domains: None,
            pm_auth_config: None,
            status: common_enums::ConnectorStatus::Inactive,
            connector_wallets_details: Some(
                domain::types::crypto_operation(
                    key_manager_state,
                    type_name!(domain::MerchantConnectorAccount),
                    domain::types::CryptoOperation::Encrypt(serde_json::Value::default().into()),
                    Identifier::Merchant(merchant_key.merchant_id.clone()),
                    merchant_key.key.get_inner().peek(),
                )
                .await
                .and_then(|val| val.try_into_operation())
                .unwrap(),
            ),
            additional_merchant_data: None,
            version: hyperswitch_domain_models::consts::API_VERSION,
        };

        db.insert_merchant_connector_account(key_manager_state, mca.clone(), &merchant_key)
            .await
            .unwrap();

        let find_call = || async {
            Conversion::convert(
                db.find_merchant_connector_account_by_profile_id_connector_name(
                    key_manager_state,
                    &profile_id,
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
            &format!(
                "{}_{}",
                merchant_id.get_string_repr(),
                profile_id.get_string_repr(),
            ),
            find_call,
            &ACCOUNTS_CACHE,
        )
        .await
        .unwrap();

        let delete_call = || async {
            db.delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
                &merchant_id,
                &common_utils::id_type::MerchantConnectorAccountId::wrap(
                    merchant_connector_id.to_string(),
                )
                .unwrap(),
            )
            .await
        };

        cache::publish_and_redact(
            &db,
            CacheKind::Accounts(
                format!("{}_{}", merchant_id.get_string_repr(), connector_label).into(),
            ),
            delete_call,
        )
        .await
        .unwrap();

        assert!(ACCOUNTS_CACHE
            .get_val::<domain::MerchantConnectorAccount>(CacheKey {
                key: format!("{}_{}", merchant_id.get_string_repr(), connector_label),
                prefix: String::default(),
            },)
            .await
            .is_none())
    }

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    #[cfg(feature = "v2")]
    async fn test_connector_profile_id_cache() {
        let conf = Settings::new().unwrap();
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let app_state = Box::pin(routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        ))
        .await;
        let state = &Arc::new(app_state)
            .get_session_state(
                &common_utils::id_type::TenantId::try_from_string("public".to_string()).unwrap(),
                None,
                || {},
            )
            .unwrap();
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

        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("test_merchant"))
                .unwrap();
        let connector_label = "stripe_USA";
        let id = common_utils::generate_merchant_connector_account_id_of_default_length();
        let profile_id =
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from("pro_max_ultra"))
                .unwrap();
        let key_manager_state = &state.into();
        db.insert_merchant_key_store(
            key_manager_state,
            domain::MerchantKeyStore {
                merchant_id: merchant_id.clone(),
                key: domain::types::crypto_operation(
                    key_manager_state,
                    type_name!(domain::MerchantConnectorAccount),
                    domain::types::CryptoOperation::Encrypt(
                        services::generate_aes256_key().unwrap().to_vec().into(),
                    ),
                    Identifier::Merchant(merchant_id.clone()),
                    master_key,
                )
                .await
                .and_then(|val| val.try_into_operation())
                .unwrap(),
                created_at: datetime!(2023-02-01 0:00),
            },
            &master_key.to_vec().into(),
        )
        .await
        .unwrap();

        let merchant_key = db
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &merchant_id,
                &master_key.to_vec().into(),
            )
            .await
            .unwrap();

        let mca = domain::MerchantConnectorAccount {
            id: id.clone(),
            merchant_id: merchant_id.clone(),
            connector_name: common_enums::connector_enums::Connector::Stripe,
            connector_account_details: domain::types::crypto_operation(
                key_manager_state,
                type_name!(domain::MerchantConnectorAccount),
                domain::types::CryptoOperation::Encrypt(serde_json::Value::default().into()),
                Identifier::Merchant(merchant_key.merchant_id.clone()),
                merchant_key.key.get_inner().peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .unwrap(),
            disabled: None,
            payment_methods_enabled: None,
            connector_type: ConnectorType::FinOperations,
            metadata: None,
            frm_configs: None,
            connector_label: Some(connector_label.to_string()),
            created_at: date_time::now(),
            modified_at: date_time::now(),
            connector_webhook_details: None,
            profile_id: profile_id.to_owned(),
            applepay_verified_domains: None,
            pm_auth_config: None,
            status: common_enums::ConnectorStatus::Inactive,
            connector_wallets_details: Some(
                domain::types::crypto_operation(
                    key_manager_state,
                    type_name!(domain::MerchantConnectorAccount),
                    domain::types::CryptoOperation::Encrypt(serde_json::Value::default().into()),
                    Identifier::Merchant(merchant_key.merchant_id.clone()),
                    merchant_key.key.get_inner().peek(),
                )
                .await
                .and_then(|val| val.try_into_operation())
                .unwrap(),
            ),
            additional_merchant_data: None,
            version: hyperswitch_domain_models::consts::API_VERSION,
            feature_metadata: None,
        };

        db.insert_merchant_connector_account(key_manager_state, mca.clone(), &merchant_key)
            .await
            .unwrap();

        let find_call = || async {
            #[cfg(feature = "v1")]
            let mca = db
                .find_merchant_connector_account_by_profile_id_connector_name(
                    key_manager_state,
                    profile_id,
                    &mca.connector_name,
                    &merchant_key,
                )
                .await
                .unwrap();
            #[cfg(feature = "v2")]
            let mca: domain::MerchantConnectorAccount = { todo!() };
            Conversion::convert(mca)
                .await
                .change_context(errors::StorageError::DecryptionError)
        };

        let _: storage::MerchantConnectorAccount = cache::get_or_populate_in_memory(
            &db,
            &format!(
                "{}_{}",
                merchant_id.clone().get_string_repr(),
                profile_id.get_string_repr()
            ),
            find_call,
            &ACCOUNTS_CACHE,
        )
        .await
        .unwrap();

        let delete_call = || async { db.delete_merchant_connector_account_by_id(&id).await };

        cache::publish_and_redact(
            &db,
            CacheKind::Accounts(
                format!("{}_{}", merchant_id.get_string_repr(), connector_label).into(),
            ),
            delete_call,
        )
        .await
        .unwrap();

        assert!(ACCOUNTS_CACHE
            .get_val::<domain::MerchantConnectorAccount>(CacheKey {
                key: format!("{}_{}", merchant_id.get_string_repr(), connector_label),
                prefix: String::default(),
            },)
            .await
            .is_none())
    }
}
