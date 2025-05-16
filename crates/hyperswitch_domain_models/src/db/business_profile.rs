use common_utils::{errors::CustomResult, id_type, types::keymanager::KeyManagerState};

use crate::{
    business_profile::{Profile, ProfileUpdate},
    merchant_key_store::MerchantKeyStore,
};

#[async_trait::async_trait]
pub trait ProfileInterface {
    type Error;

    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        business_profile: Profile,
    ) -> CustomResult<Profile, Self::Error>;

    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &id_type::ProfileId,
    ) -> CustomResult<Profile, Self::Error>;

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> CustomResult<Profile, Self::Error>;

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<Profile, Self::Error>;

    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        current_state: Profile,
        profile_update: ProfileUpdate,
    ) -> CustomResult<Profile, Self::Error>;

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &id_type::ProfileId,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error>;

    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<Vec<Profile>, Self::Error>;
}
