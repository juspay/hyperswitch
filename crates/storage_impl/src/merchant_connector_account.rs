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

/// Cache keys and invalidation for the merchant connector account list caches.
///
/// Every list query in this module is a row-subset of one of two supersets — all accounts
/// of a merchant, or all accounts of a profile — so those are the only two things ever
/// cached. A write touching a single account therefore invalidates every list entry that
/// could contain it by redacting exactly two keys, both derivable from the account alone.
///
/// This is the only place list cache keys are constructed. Adding a new list query means
/// adding a projection over one of the supersets; no write path needs to change for it.
#[cfg(feature = "accounts_cache")]
mod list_cache {
    use common_utils::id_type;

    use crate::redis::cache::CacheKind;

    /// Key of the superset holding every account of a merchant, disabled included.
    pub(super) fn merchant_scope_key(merchant_id: &id_type::MerchantId) -> String {
        format!("mca_list_m_{}", merchant_id.get_string_repr())
    }

    /// Key of the superset holding every account of a merchant's profile, disabled
    /// included. Scoped by merchant as well as profile so that one merchant's entries are
    /// never addressable by another merchant's key.
    pub(super) fn merchant_profile_scope_key(
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> String {
        format!(
            "mca_list_mp_{}_{}",
            merchant_id.get_string_repr(),
            profile_id.get_string_repr()
        )
    }

    /// The complete set of list cache entries a single account belongs to.
    ///
    /// `profile_id` is deliberately required rather than optional. An `Option` here would
    /// conflate two different things — "this account has no profile", where there is no
    /// profile superset to drop, and "the caller does not know this account's profile",
    /// where one must be dropped but no key can be built for it. The second silently
    /// leaves `mca_list_mp_*` serving pre-write rows, so a caller that lacks the profile
    /// id has to go and fetch it (as the delete paths do) rather than pass `None`.
    ///
    /// Returning a fixed-size array keeps "always exactly these two" a property of the
    /// type rather than of the body.
    pub(super) fn invalidation_kinds<'a>(
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> [CacheKind<'a>; 2] {
        [
            CacheKind::MerchantConnectorAccountList(merchant_scope_key(merchant_id).into()),
            CacheKind::MerchantConnectorAccountList(
                merchant_profile_scope_key(merchant_id, profile_id).into(),
            ),
        ]
    }
}

/// Every merchant connector account of a merchant, disabled included, ordered by
/// `created_at` ascending.
///
/// The merchant-scoped list queries are row-subsets of this and project from it, so this
/// is the only place they touch the database or the cache.
async fn list_all_by_merchant_id<T: DatabaseStore>(
    store: &RouterStore<T>,
    merchant_id: &common_utils::id_type::MerchantId,
) -> CustomResult<Vec<storage::MerchantConnectorAccount>, StorageError> {
    let find_call = || async {
        let conn = pg_accounts_connection_read(store).await?;
        storage::MerchantConnectorAccount::list_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    };

    #[cfg(not(feature = "accounts_cache"))]
    {
        find_call().await
    }

    #[cfg(feature = "accounts_cache")]
    {
        cache::get_or_populate_in_memory(
            store,
            &list_cache::merchant_scope_key(merchant_id),
            find_call,
            &cache::MCA_LIST_CACHE,
        )
        .await
    }
}

/// Every merchant connector account of a merchant's profile, disabled included, ordered
/// by `created_at` ascending.
///
/// The profile-scoped list queries are row-subsets of this and project from it, so this
/// is the only place they touch the database or the cache.
async fn list_all_by_merchant_id_profile_id<T: DatabaseStore>(
    store: &RouterStore<T>,
    merchant_id: &common_utils::id_type::MerchantId,
    profile_id: &common_utils::id_type::ProfileId,
) -> CustomResult<Vec<storage::MerchantConnectorAccount>, StorageError> {
    let find_call = || async {
        let conn = pg_accounts_connection_read(store).await?;
        storage::MerchantConnectorAccount::list_by_merchant_id_profile_id(
            &conn,
            merchant_id,
            profile_id,
        )
        .await
        .map_err(|error| report!(StorageError::from(error)))
    };

    #[cfg(not(feature = "accounts_cache"))]
    {
        find_call().await
    }

    #[cfg(feature = "accounts_cache")]
    {
        cache::get_or_populate_in_memory(
            store,
            &list_cache::merchant_profile_scope_key(merchant_id, profile_id),
            find_call,
            &cache::MCA_LIST_CACHE,
        )
        .await
    }
}

/// Decrypt a batch of account rows into domain accounts, concurrently.
///
/// Each conversion is a `BatchDecrypt` which, when the encryption service is enabled, is
/// its own HTTP round-trip to the keymanager — so decrypting a list of N accounts
/// sequentially costs N round-trips. Falling back to application-local AES this is
/// CPU-bound and merely interleaves, which costs nothing either way.
///
/// `try_join_all` preserves input order, which matters: the supersets these rows come
/// from are ordered by `created_at`, and the queries projecting them rely on it.
async fn decrypt_all<T: DatabaseStore>(
    store: &RouterStore<T>,
    accounts: Vec<storage::MerchantConnectorAccount>,
    key_store: &MerchantKeyStore,
) -> CustomResult<Vec<domain::MerchantConnectorAccount>, StorageError> {
    let keymanager_state = store
        .get_keymanager_state()
        .attach_printable("Missing KeyManagerState")?;

    futures::future::try_join_all(accounts.into_iter().map(|account| async move {
        account
            .convert(
                keymanager_state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }))
    .await
}

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
    async fn find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, Self::Error> {
        self.router_store
            .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
                merchant_id,
                get_disabled,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn list_merchant_connector_accounts_without_encrypted_including_disabled_by_merchant_id_profile_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, Self::Error> {
        self.router_store
            .list_merchant_connector_accounts_without_encrypted_including_disabled_by_merchant_id_profile_id(merchant_id, profile_id)
            .await
    }

    #[instrument(skip_all)]
    async fn list_enabled_merchant_connector_accounts_without_encrypted_by_merchant_id_profile_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, Self::Error> {
        self.router_store
            .list_enabled_merchant_connector_accounts_without_encrypted_by_merchant_id_profile_id(
                merchant_id,
                profile_id,
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
        let accounts = list_all_by_merchant_id(self, merchant_id)
            .await?
            .into_iter()
            .filter(|account| account.connector_name == connector_name)
            .collect();

        decrypt_all(self, accounts, key_store).await
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
        let _merchant_id = t.merchant_id.clone();
        let _profile_id = t.profile_id.clone();

        let insert_call = || async {
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
        };

        #[cfg(feature = "accounts_cache")]
        {
            // A newly created account belongs to both list supersets, so any cached list
            // that should now contain it has to go.
            Box::pin(cache::publish_and_redact_multiple(
                self,
                list_cache::invalidation_kinds(&_merchant_id, &_profile_id),
                insert_call,
            ))
            .await
        }

        #[cfg(not(feature = "accounts_cache"))]
        {
            insert_call().await
        }
    }

    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let accounts = list_all_by_merchant_id_profile_id(self, &key_store.merchant_id, profile_id)
            .await?
            .into_iter()
            .filter(|account| account.is_enabled() && account.connector_type == connector_type)
            .collect();

        decrypt_all(self, accounts, key_store).await
    }

    #[instrument(skip_all)]
    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccounts, Self::Error> {
        let accounts = list_all_by_merchant_id(self, merchant_id)
            .await?
            .into_iter()
            .filter(|account| get_disabled || account.is_enabled())
            .collect();

        let merchant_connector_account_vec = decrypt_all(self, accounts, key_store).await?;

        Ok(domain::MerchantConnectorAccounts::new(
            merchant_connector_account_vec,
        ))
    }

    #[instrument(skip_all)]
    async fn find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, Self::Error> {
        let accounts = list_all_by_merchant_id(self, merchant_id)
            .await?
            .into_iter()
            .filter(|account| get_disabled || account.is_enabled());

        let output = accounts
            .map(domain::MerchantConnectorAccountWithoutEncrypted::try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(Self::Error::DecryptionError)?;

        Ok(domain::MerchantConnectorAccountsWithoutEncrypted::new(
            output,
        ))
    }

    #[instrument(skip_all)]
    async fn list_merchant_connector_accounts_without_encrypted_including_disabled_by_merchant_id_profile_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, Self::Error> {
        let accounts = list_all_by_merchant_id_profile_id(self, merchant_id, profile_id)
            .await?
            .into_iter();

        let output = accounts
            .map(domain::MerchantConnectorAccountWithoutEncrypted::try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(Self::Error::DecryptionError)?;

        Ok(domain::MerchantConnectorAccountsWithoutEncrypted::new(
            output,
        ))
    }

    #[instrument(skip_all)]
    async fn list_enabled_merchant_connector_accounts_without_encrypted_by_merchant_id_profile_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, Self::Error> {
        let accounts = list_all_by_merchant_id_profile_id(self, merchant_id, profile_id)
            .await?
            .into_iter()
            .filter(|account| account.is_enabled());

        let output = accounts
            .map(domain::MerchantConnectorAccountWithoutEncrypted::try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(Self::Error::DecryptionError)?;

        Ok(domain::MerchantConnectorAccountsWithoutEncrypted::new(
            output,
        ))
    }

    #[instrument(skip_all)]
    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let accounts =
            list_all_by_merchant_id_profile_id(self, &key_store.merchant_id, profile_id).await?;

        decrypt_all(self, accounts, key_store).await
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
            connection: &diesel_models::DatabaseConnectionWithContext<'_>,
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

        // The connection handed to the closure is another handle to the connection `conn` already
        // holds, so queries issued through `conn` run within this transaction.
        let connection_pool = &conn;
        conn.raw_connection()
            .transaction_async(move |_| async move {
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
                        connection_pool,
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
                                format!("{}_{}", _profile_id.get_string_repr(), _connector_name)
                                    .into(),
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
                        ]
                        .into_iter()
                        .chain(list_cache::invalidation_kinds(&_merchant_id, &_profile_id))
                        .collect::<Vec<_>>(),
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
                    cache::CacheKind::PmFiltersCGraph(
                        format!(
                            "pm_filters_cgraph_{}_{}",
                            _merchant_id.get_string_repr(),
                            _profile_id.get_string_repr(),
                        )
                        .into(),
                    ),
                ]
                .into_iter()
                .chain(list_cache::invalidation_kinds(&_merchant_id, &_profile_id))
                .collect::<Vec<_>>(),
                update_call,
            ))
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
            Box::pin(cache::publish_and_redact_multiple(
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
                ]
                .into_iter()
                .chain(list_cache::invalidation_kinds(&_merchant_id, &_profile_id))
                .collect::<Vec<_>>(),
                update_call,
            ))
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
                ]
                .into_iter()
                .chain(list_cache::invalidation_kinds(
                    &mca.merchant_id,
                    &_profile_id,
                ))
                .collect::<Vec<_>>(),
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
                ]
                .into_iter()
                .chain(list_cache::invalidation_kinds(
                    &mca.merchant_id,
                    &_profile_id,
                ))
                .collect::<Vec<_>>(),
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
            connector_webhook_registration_details: t.connector_webhook_registration_details,
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
            connector_webhook_registration_details: None,
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

    async fn find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        get_disabled: bool,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, StorageError> {
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

        let output = accounts
            .into_iter()
            .map(domain::MerchantConnectorAccountWithoutEncrypted::try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(StorageError::DecryptionError)?;

        Ok(domain::MerchantConnectorAccountsWithoutEncrypted::new(
            output,
        ))
    }

    async fn list_merchant_connector_accounts_without_encrypted_including_disabled_by_merchant_id_profile_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, StorageError> {
        let accounts = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .filter(|account: &&storage::MerchantConnectorAccount| {
                #[cfg(feature = "v1")]
                let profile_matches = account.profile_id.as_ref() == Some(profile_id);
                #[cfg(feature = "v2")]
                let profile_matches = account.profile_id == *profile_id;

                account.merchant_id == *merchant_id && profile_matches
            })
            .cloned()
            .collect::<Vec<storage::MerchantConnectorAccount>>();

        let output = accounts
            .into_iter()
            .map(domain::MerchantConnectorAccountWithoutEncrypted::try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(StorageError::DecryptionError)?;

        Ok(domain::MerchantConnectorAccountsWithoutEncrypted::new(
            output,
        ))
    }

    async fn list_enabled_merchant_connector_accounts_without_encrypted_by_merchant_id_profile_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::MerchantConnectorAccountsWithoutEncrypted, StorageError> {
        let accounts = self
            .merchant_connector_accounts
            .lock()
            .await
            .iter()
            .filter(|account: &&storage::MerchantConnectorAccount| {
                #[cfg(feature = "v1")]
                let profile_matches = account.profile_id.as_ref() == Some(profile_id);
                #[cfg(feature = "v2")]
                let profile_matches = account.profile_id == *profile_id;

                account.merchant_id == *merchant_id
                    && profile_matches
                    && account.disabled == Some(false)
            })
            .cloned()
            .collect::<Vec<storage::MerchantConnectorAccount>>();

        let output = accounts
            .into_iter()
            .map(domain::MerchantConnectorAccountWithoutEncrypted::try_from)
            .collect::<Result<Vec<_>, _>>()
            .change_context(StorageError::DecryptionError)?;

        Ok(domain::MerchantConnectorAccountsWithoutEncrypted::new(
            output,
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Any write to a single account has to drop both supersets that could contain it,
    /// otherwise a cached list keeps serving the pre-write rows.
    #[cfg(feature = "accounts_cache")]
    #[test]
    fn invalidation_covers_both_supersets() {
        let merchant_id = common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from(
            "merchant_the_first",
        ))
        .expect("valid merchant id");
        let profile_id =
            common_utils::id_type::ProfileId::try_from(std::borrow::Cow::from("profile_the_first"))
                .expect("valid profile id");

        let keys = list_cache::invalidation_kinds(&merchant_id, &profile_id)
            .into_iter()
            .map(|kind| kind.get_key_without_prefix().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            keys,
            vec![
                "mca_list_m_merchant_the_first".to_string(),
                "mca_list_mp_merchant_the_first_profile_the_first".to_string(),
            ]
        );
    }
}
