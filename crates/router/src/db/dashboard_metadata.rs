use diesel_models::{enums, user::dashboard_metadata as storage};
use error_stack::{IntoReport, ResultExt};
use storage_impl::MockDb;

use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

#[async_trait::async_trait]
pub trait DashboardMetadataInterface {
    async fn insert_metadata(
        &self,
        metadata: storage::DashboardMetadataNew,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError>;

    async fn update_metadata(
        &self,
        user_id: Option<String>,
        merchant_id: String,
        org_id: String,
        data_key: enums::DashboardMetadata,
        dashboard_metadata_update: storage::DashboardMetadataUpdate,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError>;

    async fn find_user_scoped_dashboard_metadata(
        &self,
        user_id: &str,
        merchant_id: &str,
        org_id: &str,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError>;

    async fn find_merchant_scoped_dashboard_metadata(
        &self,
        merchant_id: &str,
        org_id: &str,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError>;

    async fn delete_all_user_scoped_dashboard_metadata_by_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
        &self,
        user_id: &str,
        merchant_id: &str,
        data_key: enums::DashboardMetadata,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError>;
}

#[async_trait::async_trait]
impl DashboardMetadataInterface for Store {
    async fn insert_metadata(
        &self,
        metadata: storage::DashboardMetadataNew,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        metadata
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_metadata(
        &self,
        user_id: Option<String>,
        merchant_id: String,
        org_id: String,
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
        .map_err(Into::into)
        .into_report()
    }

    async fn find_user_scoped_dashboard_metadata(
        &self,
        user_id: &str,
        merchant_id: &str,
        org_id: &str,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DashboardMetadata::find_user_scoped_dashboard_metadata(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
            org_id.to_owned(),
            data_keys,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn find_merchant_scoped_dashboard_metadata(
        &self,
        merchant_id: &str,
        org_id: &str,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DashboardMetadata::find_merchant_scoped_dashboard_metadata(
            &conn,
            merchant_id.to_owned(),
            org_id.to_owned(),
            data_keys,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
    async fn delete_all_user_scoped_dashboard_metadata_by_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DashboardMetadata::delete_all_user_scoped_dashboard_metadata_by_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
        &self,
        user_id: &str,
        merchant_id: &str,
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
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl DashboardMetadataInterface for MockDb {
    async fn insert_metadata(
        &self,
        metadata: storage::DashboardMetadataNew,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        let mut dashboard_metadata = self.dashboard_metadata.lock().await;
        if dashboard_metadata.iter().any(|metadata_inner| {
            metadata_inner.user_id == metadata.user_id
                && metadata_inner.merchant_id == metadata.merchant_id
                && metadata_inner.org_id == metadata.org_id
                && metadata_inner.data_key == metadata.data_key
        }) {
            Err(errors::StorageError::DuplicateValue {
                entity: "user_id, merchant_id, org_id and data_key",
                key: None,
            })?
        }
        let metadata_new = storage::DashboardMetadata {
            id: dashboard_metadata
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            user_id: metadata.user_id,
            merchant_id: metadata.merchant_id,
            org_id: metadata.org_id,
            data_key: metadata.data_key,
            data_value: metadata.data_value,
            created_by: metadata.created_by,
            created_at: metadata.created_at,
            last_modified_by: metadata.last_modified_by,
            last_modified_at: metadata.last_modified_at,
        };
        dashboard_metadata.push(metadata_new.clone());
        Ok(metadata_new)
    }

    async fn update_metadata(
        &self,
        user_id: Option<String>,
        merchant_id: String,
        org_id: String,
        data_key: enums::DashboardMetadata,
        dashboard_metadata_update: storage::DashboardMetadataUpdate,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        let mut dashboard_metadata = self.dashboard_metadata.lock().await;

        let dashboard_metadata_to_update = dashboard_metadata
            .iter_mut()
            .find(|metadata| {
                metadata.user_id == user_id
                    && metadata.merchant_id == merchant_id
                    && metadata.org_id == org_id
                    && metadata.data_key == data_key
            })
            .ok_or(errors::StorageError::MockDbError)?;

        match dashboard_metadata_update {
            storage::DashboardMetadataUpdate::UpdateData {
                data_key,
                data_value,
                last_modified_by,
            } => {
                dashboard_metadata_to_update.data_key = data_key;
                dashboard_metadata_to_update.data_value = data_value;
                dashboard_metadata_to_update.last_modified_by = last_modified_by;
                dashboard_metadata_to_update.last_modified_at = common_utils::date_time::now();
            }
        }
        Ok(dashboard_metadata_to_update.clone())
    }

    async fn find_user_scoped_dashboard_metadata(
        &self,
        user_id: &str,
        merchant_id: &str,
        org_id: &str,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        let dashboard_metadata = self.dashboard_metadata.lock().await;
        let query_result = dashboard_metadata
            .iter()
            .filter(|metadata_inner| {
                metadata_inner
                    .user_id
                    .clone()
                    .map(|user_id_inner| user_id_inner == user_id)
                    .unwrap_or(false)
                    && metadata_inner.merchant_id == merchant_id
                    && metadata_inner.org_id == org_id
                    && data_keys.contains(&metadata_inner.data_key)
            })
            .cloned()
            .collect::<Vec<storage::DashboardMetadata>>();

        if query_result.is_empty() {
            return Err(errors::StorageError::ValueNotFound(format!(
                "No dashboard_metadata available for user_id = {user_id},\
                merchant_id = {merchant_id}, org_id = {org_id} and data_keys = {data_keys:?}",
            ))
            .into());
        }
        Ok(query_result)
    }

    async fn find_merchant_scoped_dashboard_metadata(
        &self,
        merchant_id: &str,
        org_id: &str,
        data_keys: Vec<enums::DashboardMetadata>,
    ) -> CustomResult<Vec<storage::DashboardMetadata>, errors::StorageError> {
        let dashboard_metadata = self.dashboard_metadata.lock().await;
        let query_result = dashboard_metadata
            .iter()
            .filter(|metadata_inner| {
                metadata_inner.user_id.is_none()
                    && metadata_inner.merchant_id == merchant_id
                    && metadata_inner.org_id == org_id
                    && data_keys.contains(&metadata_inner.data_key)
            })
            .cloned()
            .collect::<Vec<storage::DashboardMetadata>>();

        if query_result.is_empty() {
            return Err(errors::StorageError::ValueNotFound(format!(
                "No dashboard_metadata available for merchant_id = {merchant_id},\
                      org_id = {org_id} and data_keyss = {data_keys:?}",
            ))
            .into());
        }
        Ok(query_result)
    }
    async fn delete_all_user_scoped_dashboard_metadata_by_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let mut dashboard_metadata = self.dashboard_metadata.lock().await;

        let initial_len = dashboard_metadata.len();

        dashboard_metadata.retain(|metadata_inner| {
            !(metadata_inner
                .user_id
                .clone()
                .map(|user_id_inner| user_id_inner == user_id)
                .unwrap_or(false)
                && metadata_inner.merchant_id == merchant_id)
        });

        if dashboard_metadata.len() == initial_len {
            return Err(errors::StorageError::ValueNotFound(format!(
                "No user available for user_id = {user_id} and merchant id = {merchant_id}"
            ))
            .into());
        }

        Ok(true)
    }

    async fn delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
        &self,
        user_id: &str,
        merchant_id: &str,
        data_key: enums::DashboardMetadata,
    ) -> CustomResult<storage::DashboardMetadata, errors::StorageError> {
        let mut dashboard_metadata = self.dashboard_metadata.lock().await;

        let index_to_remove = dashboard_metadata
            .iter()
            .position(|metadata_inner| {
                metadata_inner
                    .user_id
                    .as_deref()
                    .map_or(false, |user_id_inner| user_id_inner == user_id)
                    && metadata_inner.merchant_id == merchant_id
                    && metadata_inner.data_key == data_key
            })
            .ok_or(errors::StorageError::ValueNotFound(
                "No data found".to_string(),
            ))?;

        let deleted_value = dashboard_metadata.swap_remove(index_to_remove);

        Ok(deleted_value)
    }
}
