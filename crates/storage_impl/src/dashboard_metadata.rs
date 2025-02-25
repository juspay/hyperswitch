use common_utils::{errors::CustomResult, id_type};
use diesel_models::{enums, user::dashboard_metadata as storage};
use error_stack::report;
use router_env::{instrument, tracing};
use sample::dashboard_metadata::DashboardMetadataInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> DashboardMetadataInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_metadata(
        &self,
        metadata: storage::DashboardMetadataNew,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        metadata
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_metadata(
        &self,
        user_id: Option<String>,
        merchant_id: id_type::MerchantId,
        org_id: id_type::OrganizationId,
        data_key: enums::DashboardMetadata,
        dashboard_metadata_update: storage::DashboardMetadataUpdate,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DashboardMetadata::update(
            &conn,
            user_id,
            merchant_id,
            org_id,
            data_key,
            dashboard_metadata_update,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_user_scoped_dashboard_metadata(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::DashboardMetadata::find_user_scoped_dashboard_metadata(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
            org_id.to_owned(),
            data_keys,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_merchant_scoped_dashboard_metadata(
        &self,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::DashboardMetadata::find_merchant_scoped_dashboard_metadata(
            &conn,
            merchant_id.to_owned(),
            org_id.to_owned(),
            data_keys,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_all_user_scoped_dashboard_metadata_by_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DashboardMetadata::delete_all_user_scoped_dashboard_metadata_by_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
        &self,
        user_id: &str,
        merchant_id: &id_type::MerchantId,
        data_key: enums::DashboardMetadata,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DashboardMetadata::delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
            data_key,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
