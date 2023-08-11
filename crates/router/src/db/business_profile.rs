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
}

#[async_trait::async_trait]
impl BusinessProfileInterface for MockDb {
    async fn insert_business_profile(
        &self,
        _business_profile: business_profile::BusinessProfileNew,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_business_profile_by_profile_id(
        &self,
        _profile_id: &str,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_business_profile_by_profile_id(
        &self,
        _current_state: business_profile::BusinessProfile,
        _business_profile_update: business_profile::BusinessProfileUpdateInternal,
    ) -> CustomResult<business_profile::BusinessProfile, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_business_profile_by_profile_id_merchant_id(
        &self,
        _profile_id: &str,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
