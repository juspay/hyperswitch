use error_stack::report;
use router_env::{instrument, tracing};
#[cfg(feature = "accounts_cache")]
use storage_impl::redis::cache::{CacheKind, ACCOUNTS_CACHE};

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
        business_profile_update: business_profile::BusinessProfileUpdate,
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
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        profile_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        let db_func = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::business_profile::BusinessProfile::find_by_profile_id(&conn, profile_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        };
        #[cfg(not(feature = "accounts_cache"))]
        {
            db_func().await
        }
        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_in_memory(self, profile_id, db_func, &ACCOUNTS_CACHE)
                .await
        }
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        profile_name: &str,
        merchant_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        let db_func = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::business_profile::BusinessProfile::find_by_profile_name_merchant_id(
                &conn,
                profile_name,
                merchant_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(feature = "accounts_cache")]
        {
            let key = format!("{}_{}", profile_name, merchant_id);
            super::cache::get_or_populate_in_memory(self, &key, db_func, &ACCOUNTS_CACHE).await
        }

        #[cfg(not(feature = "accounts_cache"))]
        {
            db_func().await
        }
    }

    #[instrument(skip_all)]
    async fn update_business_profile_by_profile_id(
        &self,
        current_state: business_profile::BusinessProfile,
        business_profile_update: business_profile::BusinessProfileUpdate,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let updated_profile = storage::business_profile::BusinessProfile::update_by_profile_id(
            current_state,
            &conn,
            business_profile_update,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))?;

        #[cfg(feature = "accounts_cache")]
        {
            publish_and_redact_business_profile_cache(self, &updated_profile).await?;
        }
        Ok(updated_profile)
    }

    #[instrument(skip_all)]
    async fn delete_business_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let db_func = || async {
            storage::business_profile::BusinessProfile::delete_by_profile_id_merchant_id(
                &conn,
                profile_id,
                merchant_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            db_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            let business_profile = self.find_business_profile_by_profile_id(profile_id).await?;
            publish_and_redact_business_profile_cache(self, &business_profile).await?;
            db_func().await
        }
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
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[cfg(feature = "accounts_cache")]
async fn publish_and_redact_business_profile_cache(
    store: &dyn super::StorageInterface,
    business_profile: &business_profile::BusinessProfile,
) -> CustomResult<(), errors::StorageError> {
    let key1 = CacheKind::Accounts(business_profile.profile_id.as_str().into());
    let str_key = format!(
        "{}_{}",
        business_profile.profile_name.as_str(),
        business_profile.merchant_id.as_str()
    );
    let key2 = CacheKind::Accounts(str_key.as_str().into());
    let keys = vec![key1, key2];
    super::cache::publish_into_redact_channel(store, keys).await?;
    Ok(())
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
        business_profile_update: business_profile::BusinessProfileUpdate,
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
