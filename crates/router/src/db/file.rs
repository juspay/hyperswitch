use error_stack::IntoReport;

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

    async fn find_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
        file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError>;

    async fn delete_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
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
        /// Asynchronously inserts the file metadata into the database and returns the inserted file metadata if successful, or a StorageError if an error occurs.
    async fn insert_file_metadata(
        &self,
        file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        file.insert(&conn).await.map_err(Into::into).into_report()
    }

        /// Asynchronously finds file metadata by merchant ID and file ID. 
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID
    /// * `file_id` - A reference to a string representing the file ID
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the `FileMetadata` if successful, or a `StorageError` if an error occurs.
    ///
    async fn find_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
        file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::FileMetadata::find_by_merchant_id_file_id(&conn, merchant_id, file_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Deletes file metadata by the given merchant ID and file ID.
    /// Returns a boolean indicating whether the deletion was successful or not, along with any potential storage errors.
    async fn delete_file_metadata_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
        file_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::FileMetadata::delete_by_merchant_id_file_id(&conn, merchant_id, file_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously updates the metadata of a file with the provided file metadata update,
    /// using the provided file metadata. Returns a custom result containing the updated file 
    /// metadata if successful, otherwise returns a storage error.
    async fn update_file_metadata(
        &self,
        this: storage::FileMetadata,
        file_metadata: storage::FileMetadataUpdate,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, file_metadata)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl FileMetadataInterface for MockDb {
        /// Asynchronously inserts a new file metadata into the storage.
    ///
    /// # Arguments
    ///
    /// * `file` - The file metadata to be inserted into the storage.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the newly inserted file metadata on success, or a `StorageError` on failure.
    ///
    async fn insert_file_metadata(
        &self,
        _file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously finds the file metadata by merchant ID and file ID.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A string slice representing the merchant ID.
    /// * `_file_id` - A string slice representing the file ID.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the `FileMetadata` if found, otherwise a `StorageError`.
    ///
    /// # Errors
    ///
    /// If the function encounters a `MockDbError`, it returns a `StorageError`.
    ///
    async fn find_file_metadata_by_merchant_id_file_id(
        &self,
        _merchant_id: &str,
        _file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Deletes file metadata by merchant ID and file ID from the database.
    ///
    /// # Arguments
    ///
    /// * `_merchant_id` - A string reference representing the merchant ID
    /// * `_file_id` - A string reference representing the file ID
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a boolean value indicating the success of the operation, or a `StorageError` if an error occurred.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError` if the operation fails.
    ///
    async fn delete_file_metadata_by_merchant_id_file_id(
        &self,
        _merchant_id: &str,
        _file_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously updates the metadata for a file in the storage.
    ///
    /// # Arguments
    ///
    /// * `_this` - The existing file metadata to be updated.
    /// * `_file_metadata` - The updated file metadata to replace the existing one.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the updated file metadata if successful, otherwise a `StorageError`.
    ///
    async fn update_file_metadata(
        &self,
        _this: storage::FileMetadata,
        _file_metadata: storage::FileMetadataUpdate,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
