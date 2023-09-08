use error_stack::IntoReport;

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
        let mut business_profiles = self.business_profiles.lock().await;

        let business_profile_insert = business_profile::BusinessProfile {
            profile_id: business_profile.profile_id,
            merchant_id: business_profile.merchant_id,
            profile_name: business_profile.profile_name,
            created_at: business_profile.created_at,
            modified_at: business_profile.modified_at,
            return_url: business_profile.return_url,
            enable_payment_response_hash: business_profile.enable_payment_response_hash,
            payment_response_hash_key: business_profile.payment_response_hash_key,
            redirect_to_merchant_with_http_post: business_profile
                .redirect_to_merchant_with_http_post,
            webhook_details: business_profile.webhook_details,
            metadata: business_profile.metadata,
            routing_algorithm: business_profile.routing_algorithm,
            intent_fulfillment_time: business_profile.intent_fulfillment_time,
            frm_routing_algorithm: business_profile.frm_routing_algorithm,
            payout_routing_algorithm: business_profile.payout_routing_algorithm,
            is_recon_enabled: business_profile.is_recon_enabled,
        };

        business_profiles.push(business_profile_insert.clone());

        Ok(business_profile_insert)
    }

    async fn find_business_profile_by_profile_id(
        &self,
        profile_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        match self
            .business_profiles
            .lock()
            .await
            .iter()
            .find(|business_profile| business_profile.profile_id == profile_id)
        {
            Some(business_profile) => Ok(business_profile.clone()),
            None => {
                return Err(errors::StorageError::ValueNotFound(
                    "No business profile found for profile_id = {profile_id}".to_string(),
                )
                .into())
            }
        }
    }

    async fn update_business_profile_by_profile_id(
        &self,
        current_state: business_profile::BusinessProfile,
        business_profile_update: business_profile::BusinessProfileUpdateInternal,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        match self
            .business_profiles
            .lock()
            .await
            .iter_mut()
            .find(|bp| bp.profile_id == current_state.profile_id)
            .map(|bp| {
                let business_profile_updated =
                    business_profile_update.apply_changeset(current_state);
                *bp = business_profile_updated.clone();
                business_profile_updated
            }) {
            Some(business_profile) => Ok(business_profile),
            None => {
                return Err(errors::StorageError::ValueNotFound(
                    "No business profile found for profile_id = {profile_id}".to_string(),
                )
                .into())
            }
        }
    }

    async fn delete_business_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut business_profiles = self.business_profiles.lock().await;

        match business_profiles
            .iter()
            .position(|bp| bp.profile_id == profile_id && bp.merchant_id == merchant_id)
        {
            Some(index) => {
                _ = business_profiles.remove(index);
                Ok(true)
            }
            None => Err(errors::StorageError::ValueNotFound(
                "No business profile found for profile_id = {profile_id} and merchant_id = {merchant_id}".to_string(),
            )
            .into()),
        }
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
}
