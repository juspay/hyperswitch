

// use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
// use diesel_models::MerchantAccountUpdateInternal;
// use error_stack::{report, ResultExt};
// use router_env::{instrument, tracing};
// #[cfg(feature = "accounts_cache")]
// use storage_impl::redis::cache::{self, CacheKind, ACCOUNTS_CACHE};

// use super::{MasterKeyInterface, MockDb, Store};
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     db::merchant_key_store::MerchantKeyStoreInterface,
//     types::{
//         domain::{
//             self,
//             behaviour::{Conversion, ReverseConversion},
//         },
//         storage,
//     },
// };

// use hyperswitch_domain_models::errors;
use common_utils::{types::keymanager::KeyManagerState, errors::CustomResult};
use diesel_models::merchant_account as storage;
use hyperswitch_domain_models::{merchant_key_store, behaviour, merchant_account as domain};

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait MerchantAccountInterface
where
    domain::MerchantAccount:
        behaviour::Conversion<DstType = storage::MerchantAccount, NewDstType = storage::MerchantAccountNew>,
{
    type Error;
    async fn insert_merchant(
        &self,
        state: &KeyManagerState,
        merchant_account: domain::MerchantAccount,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, Self::Error>;

    async fn find_merchant_account_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, Self::Error>;

    async fn update_all_merchant_account(
        &self,
        merchant_account: domain::MerchantAccountUpdate,
    ) -> CustomResult<usize, Self::Error>;

    async fn update_merchant(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantAccount,
        merchant_account: domain::MerchantAccountUpdate,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, Self::Error>;

    async fn update_specific_fields_in_merchant(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        merchant_account: domain::MerchantAccountUpdate,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
    ) -> CustomResult<domain::MerchantAccount, Self::Error>;

    async fn find_merchant_account_by_publishable_key(
        &self,
        state: &KeyManagerState,
        publishable_key: &str,
    ) -> CustomResult<(domain::MerchantAccount, merchant_key_store::MerchantKeyStore), Self::Error>;

    #[cfg(feature = "olap")]
    async fn list_merchant_accounts_by_organization_id(
        &self,
        state: &KeyManagerState,
        organization_id: &common_utils::id_type::OrganizationId,
    ) -> CustomResult<Vec<domain::MerchantAccount>, Self::Error>;

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error>;

    #[cfg(feature = "olap")]
    async fn list_multiple_merchant_accounts(
        &self,
        state: &KeyManagerState,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
    ) -> CustomResult<Vec<domain::MerchantAccount>, Self::Error>;

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
        Self::Error,
    >;
}

// #[async_trait::async_trait]
// impl MerchantAccountInterface for MockDb {
//     #[allow(clippy::panic)]
//     async fn insert_merchant(
//         &self,
//         state: &KeyManagerState,
//         merchant_account: domain::MerchantAccount,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//     ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
//         let mut accounts = self.merchant_accounts.lock().await;
//         let account = Conversion::convert(merchant_account)
//             .await
//             .change_context(errors::StorageError::EncryptionError)?;
//         accounts.push(account.clone());

//         account
//             .convert(
//                 state,
//                 merchant_key_store.key.get_inner(),
//                 merchant_key_store.merchant_id.clone().into(),
//             )
//             .await
//             .change_context(errors::StorageError::DecryptionError)
//     }

//     #[allow(clippy::panic)]
//     async fn find_merchant_account_by_merchant_id(
//         &self,
//         state: &KeyManagerState,
//         merchant_id: &common_utils::id_type::MerchantId,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//     ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
//         let accounts = self.merchant_accounts.lock().await;
//         accounts
//             .iter()
//             .find(|account| account.get_id() == merchant_id)
//             .cloned()
//             .ok_or(errors::StorageError::ValueNotFound(format!(
//                 "Merchant ID: {:?} not found",
//                 merchant_id
//             )))?
//             .convert(
//                 state,
//                 merchant_key_store.key.get_inner(),
//                 merchant_key_store.merchant_id.clone().into(),
//             )
//             .await
//             .change_context(errors::StorageError::DecryptionError)
//     }

//     async fn update_merchant(
//         &self,
//         state: &KeyManagerState,
//         merchant_account: domain::MerchantAccount,
//         merchant_account_update: storage::MerchantAccountUpdate,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//     ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
//         let merchant_id = merchant_account.get_id().to_owned();
//         let mut accounts = self.merchant_accounts.lock().await;
//         accounts
//             .iter_mut()
//             .find(|account| account.get_id() == merchant_account.get_id())
//             .async_map(|account| async {
//                 let update = MerchantAccountUpdateInternal::from(merchant_account_update)
//                     .apply_changeset(
//                         Conversion::convert(merchant_account)
//                             .await
//                             .change_context(errors::StorageError::EncryptionError)?,
//                     );
//                 *account = update.clone();
//                 update
//                     .convert(
//                         state,
//                         merchant_key_store.key.get_inner(),
//                         merchant_key_store.merchant_id.clone().into(),
//                     )
//                     .await
//                     .change_context(errors::StorageError::DecryptionError)
//             })
//             .await
//             .transpose()?
//             .ok_or(
//                 errors::StorageError::ValueNotFound(format!(
//                     "Merchant ID: {:?} not found",
//                     merchant_id
//                 ))
//                 .into(),
//             )
//     }

//     async fn update_specific_fields_in_merchant(
//         &self,
//         state: &KeyManagerState,
//         merchant_id: &common_utils::id_type::MerchantId,
//         merchant_account_update: storage::MerchantAccountUpdate,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//     ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
//         let mut accounts = self.merchant_accounts.lock().await;
//         accounts
//             .iter_mut()
//             .find(|account| account.get_id() == merchant_id)
//             .async_map(|account| async {
//                 let update = MerchantAccountUpdateInternal::from(merchant_account_update)
//                     .apply_changeset(account.clone());
//                 *account = update.clone();
//                 update
//                     .convert(
//                         state,
//                         merchant_key_store.key.get_inner(),
//                         merchant_key_store.merchant_id.clone().into(),
//                     )
//                     .await
//                     .change_context(errors::StorageError::DecryptionError)
//             })
//             .await
//             .transpose()?
//             .ok_or(
//                 errors::StorageError::ValueNotFound(format!(
//                     "Merchant ID: {:?} not found",
//                     merchant_id
//                 ))
//                 .into(),
//             )
//     }

//     async fn find_merchant_account_by_publishable_key(
//         &self,
//         state: &KeyManagerState,
//         publishable_key: &str,
//     ) -> CustomResult<(domain::MerchantAccount, merchant_key_store::MerchantKeyStore), errors::StorageError>
//     {
//         let accounts = self.merchant_accounts.lock().await;
//         let account = accounts
//             .iter()
//             .find(|account| {
//                 account
//                     .publishable_key
//                     .as_ref()
//                     .is_some_and(|key| key == publishable_key)
//             })
//             .ok_or(errors::StorageError::ValueNotFound(format!(
//                 "Publishable Key: {} not found",
//                 publishable_key
//             )))?;
//         let key_store = self
//             .get_merchant_key_store_by_merchant_id(
//                 state,
//                 account.get_id(),
//                 &self.get_master_key().to_vec().into(),
//             )
//             .await?;
//         let merchant_account = account
//             .clone()
//             .convert(
//                 state,
//                 key_store.key.get_inner(),
//                 key_store.merchant_id.clone().into(),
//             )
//             .await
//             .change_context(errors::StorageError::DecryptionError)?;
//         Ok((merchant_account, key_store))
//     }

//     async fn update_all_merchant_account(
//         &self,
//         merchant_account_update: storage::MerchantAccountUpdate,
//     ) -> CustomResult<usize, errors::StorageError> {
//         let mut accounts = self.merchant_accounts.lock().await;
//         Ok(accounts.iter_mut().fold(0, |acc, account| {
//             let update = MerchantAccountUpdateInternal::from(merchant_account_update.clone())
//                 .apply_changeset(account.clone());
//             *account = update;
//             acc + 1
//         }))
//     }

//     async fn delete_merchant_account_by_merchant_id(
//         &self,
//         merchant_id: &common_utils::id_type::MerchantId,
//     ) -> CustomResult<bool, errors::StorageError> {
//         let mut accounts = self.merchant_accounts.lock().await;
//         accounts.retain(|x| x.get_id() != merchant_id);
//         Ok(true)
//     }

//     #[cfg(feature = "olap")]
//     async fn list_merchant_accounts_by_organization_id(
//         &self,
//         state: &KeyManagerState,
//         organization_id: &common_utils::id_type::OrganizationId,
//     ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
//         let accounts = self.merchant_accounts.lock().await;
//         let futures = accounts
//             .iter()
//             .filter(|account| account.organization_id == *organization_id)
//             .map(|account| async {
//                 let key_store = self
//                     .get_merchant_key_store_by_merchant_id(
//                         state,
//                         account.get_id(),
//                         &self.get_master_key().to_vec().into(),
//                     )
//                     .await;
//                 match key_store {
//                     Ok(key) => account
//                         .clone()
//                         .convert(state, key.key.get_inner(), key.merchant_id.clone().into())
//                         .await
//                         .change_context(errors::StorageError::DecryptionError),
//                     Err(err) => Err(err),
//                 }
//             });
//         futures::future::join_all(futures)
//             .await
//             .into_iter()
//             .collect()
//     }

//     #[cfg(feature = "olap")]
//     async fn list_multiple_merchant_accounts(
//         &self,
//         state: &KeyManagerState,
//         merchant_ids: Vec<common_utils::id_type::MerchantId>,
//     ) -> CustomResult<Vec<domain::MerchantAccount>, errors::StorageError> {
//         let accounts = self.merchant_accounts.lock().await;
//         let futures = accounts
//             .iter()
//             .filter(|account| merchant_ids.contains(account.get_id()))
//             .map(|account| async {
//                 let key_store = self
//                     .get_merchant_key_store_by_merchant_id(
//                         state,
//                         account.get_id(),
//                         &self.get_master_key().to_vec().into(),
//                     )
//                     .await;
//                 match key_store {
//                     Ok(key) => account
//                         .clone()
//                         .convert(state, key.key.get_inner(), key.merchant_id.clone().into())
//                         .await
//                         .change_context(errors::StorageError::DecryptionError),
//                     Err(err) => Err(err),
//                 }
//             });
//         futures::future::join_all(futures)
//             .await
//             .into_iter()
//             .collect()
//     }

//     #[cfg(feature = "olap")]
//     async fn list_merchant_and_org_ids(
//         &self,
//         _state: &KeyManagerState,
//         limit: u32,
//         offset: Option<u32>,
//     ) -> CustomResult<
//         Vec<(
//             common_utils::id_type::MerchantId,
//             common_utils::id_type::OrganizationId,
//         )>,
//         errors::StorageError,
//     > {
//         let accounts = self.merchant_accounts.lock().await;
//         let limit = limit.try_into().unwrap_or(accounts.len());
//         let offset = offset.unwrap_or(0).try_into().unwrap_or(0);

//         let merchant_and_org_ids = accounts
//             .iter()
//             .skip(offset)
//             .take(limit)
//             .map(|account| (account.get_id().clone(), account.organization_id.clone()))
//             .collect::<Vec<_>>();

//         Ok(merchant_and_org_ids)
//     }
// }

// #[cfg(feature = "accounts_cache")]
// async fn publish_and_redact_merchant_account_cache(
//     store: &dyn super::StorageInterface,
//     merchant_account: &storage::MerchantAccount,
// ) -> CustomResult<(), errors::StorageError> {
//     let publishable_key = merchant_account
//         .publishable_key
//         .as_ref()
//         .map(|publishable_key| CacheKind::Accounts(publishable_key.into()));

//     #[cfg(feature = "v1")]
//     let cgraph_key = merchant_account.default_profile.as_ref().map(|profile_id| {
//         CacheKind::CGraph(
//             format!(
//                 "cgraph_{}_{}",
//                 merchant_account.get_id().get_string_repr(),
//                 profile_id.get_string_repr(),
//             )
//             .into(),
//         )
//     });

//     // TODO: we will not have default profile in v2
//     #[cfg(feature = "v2")]
//     let cgraph_key = None;

//     let mut cache_keys = vec![CacheKind::Accounts(
//         merchant_account.get_id().get_string_repr().into(),
//     )];

//     cache_keys.extend(publishable_key.into_iter());
//     cache_keys.extend(cgraph_key.into_iter());

//     cache::redact_from_redis_and_publish(store.get_cache_store().as_ref(), cache_keys).await?;
//     Ok(())
// }

// #[cfg(feature = "accounts_cache")]
// async fn publish_and_redact_all_merchant_account_cache(
//     store: &dyn super::StorageInterface,
//     merchant_accounts: &[storage::MerchantAccount],
// ) -> CustomResult<(), errors::StorageError> {
//     let merchant_ids = merchant_accounts
//         .iter()
//         .map(|merchant_account| merchant_account.get_id().get_string_repr().to_string());
//     let publishable_keys = merchant_accounts
//         .iter()
//         .filter_map(|m| m.publishable_key.clone());

//     let cache_keys: Vec<CacheKind<'_>> = merchant_ids
//         .chain(publishable_keys)
//         .map(|s| CacheKind::Accounts(s.into()))
//         .collect();

//     cache::redact_from_redis_and_publish(store.get_cache_store().as_ref(), cache_keys).await?;
//     Ok(())
// }
