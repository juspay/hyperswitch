use error_stack::IntoReport;
use router_env::{instrument, tracing};
use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    types::storage::{self, business_profile},
};

#[async_trait::async_trait]
pub trait BusinessProfileInterface {
    async fn insert_business_profile(
        &self,
        business_profile: business_profile::BusinessProfileNew,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError>;

    async fn find_business_profile_by_profile_id(
        &self,
        profile_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError>;

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        profile_name: &str,
        merchant_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError>;

    async fn update_business_profile_by_profile_id(
        &self,
        current_state: business_profile::BusinessProfile,
        business_profile_update: business_profile::BusinessProfileUpdateInternal,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError>;

    async fn delete_business_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn list_business_profile_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<business_profile::BusinessProfile>, errors::StorageError>;
}

#[async_trait::async_trait]
impl BusinessProfileInterface for Store {
    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        business_profile: business_profile::BusinessProfileNew,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        business_profile
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        profile_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::business_profile::BusinessProfile::find_by_profile_id(&conn, profile_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        profile_name: &str,
        merchant_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::business_profile::BusinessProfile::find_by_profile_name_merchant_id(
            &conn,
            profile_name,
            merchant_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    #[instrument(skip_all)]
    async fn update_business_profile_by_profile_id(
        &self,
        current_state: business_profile::BusinessProfile,
        business_profile_update: business_profile::BusinessProfileUpdateInternal,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::business_profile::BusinessProfile::update_by_profile_id(
            current_state,
            &conn,
            business_profile_update,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    #[instrument(skip_all)]
    async fn delete_business_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::business_profile::BusinessProfile::delete_by_profile_id_merchant_id(
            &conn,
            profile_id,
            merchant_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    #[instrument(skip_all)]
    async fn list_business_profile_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<business_profile::BusinessProfile>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::business_profile::BusinessProfile::list_business_profile_by_merchant_id(
            &conn,
            merchant_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl BusinessProfileInterface for MockDb {
    async fn insert_business_profile(
        &self,
        business_profile: business_profile::BusinessProfileNew,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        let business_profile_insert = business_profile::BusinessProfile::from(business_profile);
        self.business_profiles
            .lock()
            .await
            .push(business_profile_insert.clone());
        Ok(business_profile_insert)
    }

    async fn find_business_profile_by_profile_id(
        &self,
        profile_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| business_profile.profile_id == profile_id)
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No business profile found for profile_id = {}",
                    profile_id
                ))
                .into(),
            )
            .cloned()
    }

    async fn update_business_profile_by_profile_id(
        &self,
        current_state: business_profile::BusinessProfile,
        business_profile_update: business_profile::BusinessProfileUpdateInternal,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        self.business_profiles
            .lock()
            .await
            .iter_mut()
            .find(|bp| bp.profile_id == current_state.profile_id)
            .map(|bp| {
                let business_profile_updated =
                    business_profile_update.apply_changeset(current_state.clone());
                *bp = business_profile_updated.clone();
                business_profile_updated
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "No business profile found for profile_id = {}",
                    current_state.profile_id
                ))
                .into(),
            )
    }

    async fn delete_business_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut business_profiles = self.business_profiles.lock().await;
        let index = business_profiles
            .iter()
            .position(|bp| bp.profile_id == profile_id && bp.merchant_id == merchant_id)
            .ok_or::<errors::StorageError>(errors::StorageError::ValueNotFound(format!(
                "No business profile found for profile_id = {} and merchant_id = {}",
                profile_id, merchant_id
            )))?;
        business_profiles.remove(index);
        Ok(true)
    }

    async fn list_business_profile_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<Vec<business_profile::BusinessProfile>, errors::StorageError> {
        let business_profile_by_merchant_id = self
            .business_profiles
            .lock()
            .await
            .iter()
            .filter(|business_profile| business_profile.merchant_id == merchant_id)
            .cloned()
            .collect();

        Ok(business_profile_by_merchant_id)
    }

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        _profile_name: &str,
        _merchant_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
