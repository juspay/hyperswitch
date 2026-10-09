use error_stack::report;
use router_env::{instrument, tracing};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait FileMetadataInterface {
    async fn insert_file_metadata(
        &self,
        file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError>;

    async fn find_file_metadata_by_processor_merchant_id_file_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError>;

    async fn delete_file_metadata_by_processor_merchant_id_file_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;

    async fn update_file_metadata(
        &self,
        this: storage::FileMetadata,
        file_metadata: storage::FileMetadataUpdate,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError>;
}

#[async_trait::async_trait]
impl FileMetadataInterface for Store {
    #[instrument(skip_all)]
    async fn insert_file_metadata(
        &self,
        file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        file.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_file_metadata_by_processor_merchant_id_file_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        // Stagger release fallback: first try processor_merchant_id, if not found fallback to merchant_id
        // For old records processor_merchant_id is NULL, so we use merchant_id (which has the same value)
        let result = storage::FileMetadata::find_by_processor_merchant_id_file_id(
            &conn,
            processor_merchant_id,
            file_id,
        )
        .await;

        match result {
            Ok(file) => Ok(file),
            Err(error) => {
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) {
                    storage::FileMetadata::find_by_merchant_id_file_id(
                        &conn,
                        processor_merchant_id,
                        file_id,
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                } else {
                    Err(report!(errors::StorageError::from(error)))
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn delete_file_metadata_by_processor_merchant_id_file_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        // Stagger release fallback: first try processor_merchant_id, if not found fallback to merchant_id
        // For old records processor_merchant_id is NULL, so we use merchant_id (which has the same value)
        let result = storage::FileMetadata::delete_by_processor_merchant_id_file_id(
            &conn,
            processor_merchant_id,
            file_id,
        )
        .await;

        match result {
            Ok(success) => Ok(success),
            Err(error) => {
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) {
                    storage::FileMetadata::delete_by_merchant_id_file_id(
                        &conn,
                        processor_merchant_id,
                        file_id,
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                } else {
                    Err(report!(errors::StorageError::from(error)))
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn update_file_metadata(
        &self,
        this: storage::FileMetadata,
        file_metadata: storage::FileMetadataUpdate,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, file_metadata)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl FileMetadataInterface for MockDb {
    async fn insert_file_metadata(
        &self,
        _file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_file_metadata_by_processor_merchant_id_file_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_file_metadata_by_processor_merchant_id_file_id(
        &self,
        _processor_merchant_id: &common_utils::id_type::MerchantId,
        _file_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_file_metadata(
        &self,
        _this: storage::FileMetadata,
        _file_metadata: storage::FileMetadataUpdate,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
