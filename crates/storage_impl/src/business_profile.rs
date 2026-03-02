use common_utils::ext_traits::AsyncExt;
use diesel_models::business_profile::{self, ProfileUpdateInternal};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    business_profile as domain,
    business_profile::ProfileInterface,
    merchant_key_store::MerchantKeyStore,
};
use router_env::{instrument, tracing};

use crate::{
    kv_router_store,
    utils::{pg_accounts_connection_read, pg_accounts_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore, StorageError,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> ProfileInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        merchant_key_store: &MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .insert_business_profile(merchant_key_store, business_profile)
            .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_profile_id(merchant_key_store, profile_id)
            .await
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_merchant_id_profile_id(
                merchant_key_store,
                merchant_id,
                profile_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .find_business_profile_by_profile_name_merchant_id(
                merchant_key_store,
                profile_name,
                merchant_id,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.router_store
            .update_profile_by_profile_id(merchant_key_store, current_state, profile_update)
            .await
    }

    #[instrument(skip_all)]
    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        self.router_store
            .delete_profile_by_profile_id_merchant_id(profile_id, merchant_id)
            .await
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, StorageError> {
        self.router_store
            .list_profile_by_merchant_id(merchant_key_store, merchant_id)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ProfileInterface for RouterStore<T> {
    type Error = StorageError;
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        merchant_key_store: &MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_write(self).await?;
        business_profile
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database(
            merchant_key_store,
            business_profile::Profile::find_by_profile_id(&conn, profile_id),
        )
        .await
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database(
            merchant_key_store,
            business_profile::Profile::find_by_merchant_id_profile_id(
                &conn,
                merchant_id,
                profile_id,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.call_database(
            merchant_key_store,
            business_profile::Profile::find_by_profile_name_merchant_id(
                &conn,
                profile_name,
                merchant_id,
            ),
        )
        .await
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, StorageError> {
        let conn = pg_accounts_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(StorageError::EncryptionError)?
            .update_by_profile_id(&conn, ProfileUpdateInternal::from(profile_update))
            .await
            .map_err(|error| report!(StorageError::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        let conn = pg_accounts_connection_write(self).await?;
        business_profile::Profile::delete_by_profile_id_merchant_id(&conn, profile_id, merchant_id)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, StorageError> {
        let conn = pg_accounts_connection_read(self).await?;
        self.find_resources(
            merchant_key_store,
            business_profile::Profile::list_profile_by_merchant_id(&conn, merchant_id),
        )
        .await
    }
}

#[async_trait::async_trait]
impl ProfileInterface for MockDb {
    type Error = StorageError;
    async fn insert_business_profile(
        &self,
        merchant_key_store: &MerchantKeyStore,
        business_profile: domain::Profile,
    ) -> CustomResult<domain::Profile, StorageError> {
        let stored_business_profile = Conversion::convert(business_profile)
            .await
            .change_context(StorageError::EncryptionError)?;

        self.business_profiles
            .lock()
            .await
            .push(stored_business_profile.clone());

        stored_business_profile
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn find_business_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| business_profile.get_id() == profile_id)
            .cloned()
            .async_map(|business_profile| async {
                business_profile
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "No business profile found for profile_id = {profile_id:?}"
                ))
                .into(),
            )
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<domain::Profile, StorageError> {
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
                        self.get_keymanager_state().attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "No business profile found for merchant_id = {merchant_id:?} and profile_id = {profile_id:?}"
                ))
                .into(),
            )
    }

    async fn update_profile_by_profile_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        current_state: domain::Profile,
        profile_update: domain::ProfileUpdate,
    ) -> CustomResult<domain::Profile, StorageError> {
        let profile_id = current_state.get_id().to_owned();
        self.business_profiles
            .lock()
            .await
            .iter_mut()
            .find(|business_profile| business_profile.get_id() == current_state.get_id())
            .async_map(|business_profile| async {
                let profile_updated = ProfileUpdateInternal::from(profile_update).apply_changeset(
                    Conversion::convert(current_state)
                        .await
                        .change_context(StorageError::EncryptionError)?,
                );
                *business_profile = profile_updated.clone();

                profile_updated
                    .convert(
                        self.get_keymanager_state()
                            .attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "No business profile found for profile_id = {profile_id:?}",
                ))
                .into(),
            )
    }

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, StorageError> {
        let mut business_profiles = self.business_profiles.lock().await;
        let index = business_profiles
            .iter()
            .position(|business_profile| {
                business_profile.get_id() == profile_id
                    && business_profile.merchant_id == *merchant_id
            })
            .ok_or::<StorageError>(StorageError::ValueNotFound(format!(
                "No business profile found for profile_id = {profile_id:?} and merchant_id = {merchant_id:?}"
            )))?;
        business_profiles.remove(index);
        Ok(true)
    }

    async fn list_profile_by_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<Vec<domain::Profile>, StorageError> {
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
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)?;
            domain_business_profiles.push(domain_profile);
        }

        Ok(domain_business_profiles)
    }

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        merchant_key_store: &MerchantKeyStore,
        profile_name: &str,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<domain::Profile, StorageError> {
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
                        self.get_keymanager_state().attach_printable("Missing KeyManagerState")?,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
            })
            .await
            .transpose()?
            .ok_or(
                StorageError::ValueNotFound(format!(
                    "No business profile found for profile_name = {profile_name} and merchant_id = {merchant_id:?}"

                ))
                .into(),
            )
    }
}
