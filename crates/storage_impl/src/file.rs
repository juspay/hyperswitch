use common_utils::errors::CustomResult;
use diesel_models::file as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::file::FileMetadataInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> FileMetadataInterface for RouterStore<T> {
    type Error = errors::StorageError;

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
    async fn find_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::FileMetadata::find_by_merchant_id_file_id(&conn, merchant_id, file_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        file_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::FileMetadata::delete_by_merchant_id_file_id(&conn, merchant_id, file_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
