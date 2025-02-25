// use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
// use error_stack::{report, ResultExt};
// use router_env::{instrument, tracing};

// use super::Store;
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     db::MockDb,
//     types::{
//         domain::{
//             self,
//             behaviour::{behaviour::Conversion, ReverseConversion},
//         },
//         storage,
//     },
// };

// use hyperswitch_domain_models::errors;
use common_utils::{types::keymanager::KeyManagerState, errors::CustomResult};
use hyperswitch_domain_models::{merchant_key_store, behaviour};
use hyperswitch_domain_models::business_profile as domain;
use diesel_models::business_profile as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait ProfileInterface
where
    domain::Profile: behaviour::Conversion<DstType = storage::Profile, NewDstType = storage::ProfileNew>,
{
    type Error;
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, Self::Error>;

    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, Self::Error>;

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, Self::Error>;

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, Self::Error>;

    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, Self::Error>;

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error>;

    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &merchant_key_store::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, Self::Error>;
}

// #[async_trait::async_trait]
// impl ProfileInterface for MockDb {
//     async fn insert_business_profile(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         business_profile: domain::Profile,
//     ) -> CustomResult<domain::Profile, errors::StorageError> {
//         let stored_business_profile = behaviour::Conversion::convert(business_profile)
//             .await
//             .change_context(errors::StorageError::EncryptionError)?;

//         self.business_profiles
//             .lock()
//             .await
//             .push(stored_business_profile.clone());

//         stored_business_profile
//             .convert(
//                 key_manager_state,
//                 merchant_key_store.key.get_inner(),
//                 merchant_key_store.merchant_id.clone().into(),
//             )
//             .await
//             .change_context(errors::StorageError::DecryptionError)
//     }

//     async fn find_business_profile_by_profile_id(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         profile_id: &common_utils::id_type::ProfileId,
//     ) -> CustomResult<domain::Profile, errors::StorageError> {
//         self.business_profiles
//             .lock()
//             .await
//             .iter()
//             .find(|business_profile| business_profile.get_id() == profile_id)
//             .cloned()
//             .async_map(|business_profile| async {
//                 business_profile
//                     .convert(
//                         key_manager_state,
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
//                     "No business profile found for profile_id = {profile_id:?}"
//                 ))
//                 .into(),
//             )
//     }

//     async fn find_business_profile_by_merchant_id_profile_id(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         merchant_id: &common_utils::id_type::MerchantId,
//         profile_id: &common_utils::id_type::ProfileId,
//     ) -> CustomResult<domain::Profile, errors::StorageError> {
//         self.business_profiles
//             .lock()
//             .await
//             .iter()
//             .find(|business_profile| {
//                 business_profile.merchant_id == *merchant_id
//                     && business_profile.get_id() == profile_id
//             })
//             .cloned()
//             .async_map(|business_profile| async {
//                 business_profile
//                     .convert(
//                         key_manager_state,
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
//                     "No business profile found for merchant_id = {merchant_id:?} and profile_id = {profile_id:?}"
//                 ))
//                 .into(),
//             )
//     }

//     async fn update_profile_by_profile_id(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         current_state: domain::Profile,
//         profile_update: domain::ProfileUpdate,
//     ) -> CustomResult<domain::Profile, errors::StorageError> {
//         let profile_id = current_state.get_id().to_owned();
//         self.business_profiles
//             .lock()
//             .await
//             .iter_mut()
//             .find(|business_profile| business_profile.get_id() == current_state.get_id())
//             .async_map(|business_profile| async {
//                 let profile_updated = storage::ProfileUpdateInternal::from(profile_update)
//                     .apply_changeset(
//                         behaviour::Conversion::convert(current_state)
//                             .await
//                             .change_context(errors::StorageError::EncryptionError)?,
//                     );
//                 *business_profile = profile_updated.clone();

//                 profile_updated
//                     .convert(
//                         key_manager_state,
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
//                     "No business profile found for profile_id = {profile_id:?}",
//                 ))
//                 .into(),
//             )
//     }

//     async fn delete_profile_by_profile_id_merchant_id(
//         &self,
//         profile_id: &common_utils::id_type::ProfileId,
//         merchant_id: &common_utils::id_type::MerchantId,
//     ) -> CustomResult<bool, errors::StorageError> {
//         let mut business_profiles = self.business_profiles.lock().await;
//         let index = business_profiles
//             .iter()
//             .position(|business_profile| {
//                 business_profile.get_id() == profile_id
//                     && business_profile.merchant_id == *merchant_id
//             })
//             .ok_or::<errors::StorageError>(errors::StorageError::ValueNotFound(format!(
//                 "No business profile found for profile_id = {profile_id:?} and merchant_id = {merchant_id:?}"
//             )))?;
//         business_profiles.remove(index);
//         Ok(true)
//     }

//     async fn list_profile_by_merchant_id(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         merchant_id: &common_utils::id_type::MerchantId,
//     ) -> CustomResult<Vec<domain::Profile>, errors::StorageError> {
//         let business_profiles = self
//             .business_profiles
//             .lock()
//             .await
//             .iter()
//             .filter(|business_profile| business_profile.merchant_id == *merchant_id)
//             .cloned()
//             .collect::<Vec<_>>();

//         let mut domain_business_profiles = Vec::with_capacity(business_profiles.len());

//         for business_profile in business_profiles {
//             let domain_profile = business_profile
//                 .convert(
//                     key_manager_state,
//                     merchant_key_store.key.get_inner(),
//                     merchant_key_store.merchant_id.clone().into(),
//                 )
//                 .await
//                 .change_context(errors::StorageError::DecryptionError)?;
//             domain_business_profiles.push(domain_profile);
//         }

//         Ok(domain_business_profiles)
//     }

//     async fn find_business_profile_by_profile_name_merchant_id(
//         &self,
//         key_manager_state: &KeyManagerState,
//         merchant_key_store: &merchant_key_store::MerchantKeyStore,
//         profile_name: &str,
//         merchant_id: &common_utils::id_type::MerchantId,
//     ) -> CustomResult<domain::Profile, errors::StorageError> {
//         self.business_profiles
//             .lock()
//             .await
//             .iter()
//             .find(|business_profile| {
//                 business_profile.profile_name == profile_name
//                     && business_profile.merchant_id == *merchant_id
//             })
//             .cloned()
//             .async_map(|business_profile| async {
//                 business_profile
//                     .convert(
//                         key_manager_state,
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
//                     "No business profile found for profile_name = {profile_name} and merchant_id = {merchant_id:?}"

//                 ))
//                 .into(),
//             )
//     }
// }
