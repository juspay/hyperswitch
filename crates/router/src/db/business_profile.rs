use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage,
    },
};

#[async_trait::async_trait]
pub trait ProfileInterface
where
    domain::Profile: Conversion<DstType = storage::Profile, NewDstType = storage::ProfileNew>,
{
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, errors::StorageError>;

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, errors::StorageError>;
}

#[async_trait::async_trait]
impl ProfileInterface for Store {
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        business_profile
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Profile::find_by_profile_id(&conn, profile_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Profile::find_by_merchant_id_profile_id(&conn, merchant_id, profile_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Profile::find_by_profile_name_merchant_id(&conn, profile_name, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update_by_profile_id(&conn, storage::ProfileUpdateInternal::from(profile_update))
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Profile::delete_by_profile_id_merchant_id(&conn, profile_id, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Profile::list_profile_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|business_profiles| async {
                let mut domain_business_profiles = Vec::with_capacity(business_profiles.len());
                for business_profile in business_profiles.into_iter() {
                    domain_business_profiles.push(
                        business_profile
                            .convert(
                                key_manager_state,
                                merchant_key_store.key.get_inner(),
                                merchant_key_store.merchant_id.clone().into(),
                            )
                            .await
                            .change_context(errors::StorageError::DecryptionError)?,
                    );
                }
                Ok(domain_business_profiles)
            })
            .await
    }
}

#[async_trait::async_trait]
impl ProfileInterface for MockDb {
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let stored_business_profile = Conversion::convert(business_profile)
            .await
            .change_context(errors::StorageError::EncryptionError)?;

        self.business_profiles
            .lock()
            .await
            .push(stored_business_profile.clone());

        stored_business_profile
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| business_profile.get_id() == profile_id)
            .cloned()
            .async_map(|business_profile| async {
                business_profile
                    .convert(
                        key_manager_state,
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
                    "No business profile found for profile_id = {profile_id:?}"
                ))
                .into(),
            )
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| {
                business_profile.merchant_id == *merchant_id
                    && business_profile.get_id() == profile_id
            })
            .cloned()
            .async_map(|business_profile| async {
                business_profile
                    .convert(
                        key_manager_state,
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
                    "No business profile found for merchant_id = {merchant_id:?} and profile_id = {profile_id:?}"
                ))
                .into(),
            )
    }

    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        let profile_id = current_state.get_id().to_owned();
        self.business_profiles
            .lock()
            .await
            .iter_mut()
            .find(|business_profile| business_profile.get_id() == current_state.get_id())
            .async_map(|business_profile| async {
                let profile_updated = storage::ProfileUpdateInternal::from(profile_update)
                    .apply_changeset(
                        Conversion::convert(current_state)
                            .await
                            .change_context(errors::StorageError::EncryptionError)?,
                    );
                *business_profile = profile_updated.clone();

                profile_updated
                    .convert(
                        key_manager_state,
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
                    "No business profile found for profile_id = {profile_id:?}",
                ))
                .into(),
            )
    }

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut business_profiles = self.business_profiles.lock().await;
        let index = business_profiles
            .iter()
            .position(|business_profile| {
                business_profile.get_id() == profile_id
                    && business_profile.merchant_id == *merchant_id
            })
            .ok_or::<errors::StorageError>(errors::StorageError::ValueNotFound(format!(
                "No business profile found for profile_id = {profile_id:?} and merchant_id = {merchant_id:?}"
            )))?;
        business_profiles.remove(index);
        Ok(true)
    }

    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, errors::StorageError> {
        let business_profiles = self
            .business_profiles
            .lock()
            .await
            .iter()
            .filter(|business_profile| business_profile.merchant_id == *merchant_id)
            .cloned()
            .collect::<Vec<_>>();

        let mut domain_business_profiles = Vec::with_capacity(business_profiles.len());

        for business_profile in business_profiles {
            let domain_profile = business_profile
                .convert(
                    key_manager_state,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)?;
            domain_business_profiles.push(domain_profile);
        }

        Ok(domain_business_profiles)
    }

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &domain::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, errors::StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| {
                business_profile.profile_name == profile_name
                    && business_profile.merchant_id == *merchant_id
            })
            .cloned()
            .async_map(|business_profile| async {
                business_profile
                    .convert(
                        key_manager_state,
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
                    "No business profile found for profile_name = {profile_name} and merchant_id = {merchant_id:?}"

                ))
                .into(),
            )
    }
}
