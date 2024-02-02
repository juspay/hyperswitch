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

    async fn delete_user_scoped_dashboard_metadata_by_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl DashboardMetadataInterface for Store {
        /// Asynchronously inserts the provided metadata into the database and returns the inserted dashboard metadata.
    ///
    /// # Arguments
    ///
    /// * `metadata` - The dashboard metadata to insert into the database.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the inserted `DashboardMetadata`, or a `StorageError` if the insertion fails.
    ///
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

        /// Asynchronously updates the metadata for a dashboard with the provided information.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The optional user ID associated with the metadata.
    /// * `merchant_id` - The ID of the merchant associated with the metadata.
    /// * `org_id` - The ID of the organization associated with the metadata.
    /// * `data_key` - An enum representing the type of metadata being updated.
    /// * `dashboard_metadata_update` - The updated metadata for the dashboard.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the updated `DashboardMetadata` if successful, otherwise returns a `StorageError`.
    ///
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

        /// Asynchronously finds the dashboard metadata for a specific user within the scope of a merchant and organization, based on the provided data keys.
    /// 
    /// # Arguments
    /// 
    /// * `user_id` - The ID of the user for whom to find the dashboard metadata.
    /// * `merchant_id` - The ID of the merchant within which the user's dashboard metadata should be scoped.
    /// * `org_id` - The ID of the organization within which the user's dashboard metadata should be scoped.
    /// * `data_keys` - A vector of enums representing the specific data keys to search for within the user's dashboard metadata.
    /// 
    /// # Returns
    /// 
    /// A result containing a vector of `storage::DashboardMetadata` if successful, or a `errors::StorageError` if an error occurred during the operation.
    /// 
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

        /// Asynchronously finds the dashboard metadata for a specific merchant and organization with the given data keys.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - The ID of the merchant for which to find the dashboard metadata.
    /// * `org_id` - The ID of the organization for which to find the dashboard metadata.
    /// * `data_keys` - A vector of enums representing the dashboard metadata keys to search for.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `DashboardMetadata` or a `StorageError` if an error occurs.
    /// 
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
        /// Asynchronously deletes the dashboard metadata associated with a user and a merchant
    /// by their respective IDs.
    async fn delete_user_scoped_dashboard_metadata_by_merchant_id(
        &self,
        user_id: &str,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::DashboardMetadata::delete_user_scoped_dashboard_metadata_by_merchant_id(
            &conn,
            user_id.to_owned(),
            merchant_id.to_owned(),
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl DashboardMetadataInterface for MockDb {
        /// Asynchronously inserts a new dashboard metadata into the storage. If the metadata being inserted already exists, it returns a `DuplicateValue` error. Otherwise, it adds the new metadata to the dashboard metadata collection and returns the inserted metadata.
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

        /// Asynchronously updates the metadata for a dashboard based on the provided user ID, merchant ID, organization ID, data key, and dashboard metadata update. It searches for the metadata to update based on the provided criteria and then updates the data key, data value, last modified by, and last modified at fields with the values from the dashboard metadata update. Returns a result containing the updated dashboard metadata or a storage error if the update failed.
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

        /// Asynchronously finds the dashboard metadata scoped to a specific user, merchant, and organization
    /// based on the provided data keys. Returns a vector of storage::DashboardMetadata if found,
    /// otherwise returns a StorageError.
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

        /// Asynchronously finds the dashboard metadata scoped to a specific merchant and organization with the given data keys. 
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - A string reference representing the ID of the merchant.
    /// * `org_id` - A string reference representing the ID of the organization.
    /// * `data_keys` - A vector of DashboardMetadata enums containing the keys for the data to be retrieved.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `DashboardMetadata` or a `StorageError` if the metadata is not found.
    /// 
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
        /// Asynchronously deletes the dashboard metadata associated with a specific user and merchant ID from the storage.
    /// Returns a boolean indicating the success of the operation.
    ///
    /// # Arguments
    ///
    /// * `user_id` - A string reference representing the user ID.
    /// * `merchant_id` - A string reference representing the merchant ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a boolean indicating the success of the operation or a `StorageError` if the operation fails.
    ///
    async fn delete_user_scoped_dashboard_metadata_by_merchant_id(
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
}
