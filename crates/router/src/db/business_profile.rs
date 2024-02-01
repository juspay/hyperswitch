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
        /// Inserts a new business profile into the database using a writable connection and returns the inserted business profile.
    /// 
    /// # Arguments
    /// 
    /// * `business_profile` - The new business profile to be inserted into the database.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the inserted `BusinessProfile` if successful, otherwise an `errors::StorageError`.
    /// 
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

        /// Asynchronously finds a business profile by the given profile ID.
    /// 
    /// # Arguments
    /// 
    /// * `profile_id` - A reference to a string representing the profile ID of the business profile to be found.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the found `business_profile::BusinessProfile` if successful, or an `errors::StorageError` if an error occurs.
    ///
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

        /// Asynchronously finds a business profile by its profile name and merchant ID.
    ///
    /// # Arguments
    ///
    /// * `profile_name` - The name of the profile to search for.
    /// * `merchant_id` - The ID of the merchant associated with the profile.
    ///
    /// # Returns
    ///
    /// The result of the operation, which is a custom result containing either a business profile or a storage error.
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

        /// Asynchronously updates a business profile using the provided profile ID. It takes the current state of the business profile and the internal business profile update as input, and returns a CustomResult containing the updated business profile or a StorageError if an error occurs during the update process.
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

        /// Asynchronously deletes a business profile by its profile ID and merchant ID.
    /// 
    /// # Arguments
    /// 
    /// * `profile_id` - The profile ID of the business profile to be deleted.
    /// * `merchant_id` - The merchant ID of the business profile to be deleted.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a boolean value indicating whether the deletion was successful, or an `errors::StorageError` if an error occurred.
    /// 
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

        /// Asynchronously retrieves a list of business profiles associated with a specific merchant ID.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `BusinessProfile` objects or a `StorageError` if the operation fails.
    ///
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
        /// Asynchronously inserts a new business profile into the storage and returns the inserted business profile.
    /// 
    /// # Arguments
    /// 
    /// * `business_profile` - The new business profile to be inserted.
    /// 
    /// # Returns
    /// 
    /// * `CustomResult<business_profile::BusinessProfile, errors::StorageError>` - A result containing the inserted business profile or a storage error.
    /// 
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
        /// Asynchronously finds a business profile by the given profile_id. It locks the business_profiles
    /// and iterates through them to find a matching profile_id. If a business profile with the
    /// specified profile_id is found, it is returned wrapped in a Result. If no matching business
    /// profile is found, a StorageError is returned indicating that the value was not found.
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

        /// This method updates a business profile with the given profile ID by applying the changes specified in the `business_profile_update`. It searches for the business profile in the stored collection, updates it, and returns the updated business profile. If no business profile is found for the given profile ID, it returns a `StorageError` indicating the value was not found.
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

        /// Asynchronously deletes a business profile based on the provided profile ID and merchant ID.
    ///
    /// # Arguments
    /// * `profile_id` - A string slice representing the profile ID of the business profile to be deleted.
    /// * `merchant_id` - A string slice representing the merchant ID of the business profile to be deleted.
    ///
    /// # Returns
    /// A `CustomResult` containing a boolean value indicating whether the deletion was successful, or an `errors::StorageError` if an error occurred.
    ///
    /// # Errors
    /// An `errors::StorageError` is returned if no business profile is found for the provided profile ID and merchant ID.
    ///
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

        /// Asynchronously retrieves a list of business profiles based on the merchant ID provided.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A string reference representing the merchant ID to filter the business profiles by.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of BusinessProfile objects if the operation is successful, or a StorageError if an error occurs.
    ///
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

        /// Asynchronously finds a business profile by profile name and merchant id.
    ///
    /// # Arguments
    ///
    /// * `_profile_name` - A reference to a string representing the profile name.
    /// * `_merchant_id` - A reference to a string representing the merchant id.
    ///
    /// # Returns
    ///
    /// * A `CustomResult` containing a `business_profile::BusinessProfile` if the profile is found, otherwise a `StorageError`.
    ///
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        _profile_name: &str,
        _merchant_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
