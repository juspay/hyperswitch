use common_utils::errors::{CustomResult};
use common_utils::pii::REDACTED;
use crate::services::{Store, MockDb};
use crate::cache::Cacheable;
use crate::db::cache::publish_and_redact;
use crate::{self as storage, cache, CardInfo, enums, EphemeralKeyNew, EphemeralKey};
use crate::{domain::behaviour::Conversion, connection};
use crate::AddressNew;
use crate::address::AddressUpdateInternal;
use error_stack::{IntoReport, ResultExt};
use crate::{domain, errors};
use crate::domain::CustomerUpdate;

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
    async fn insert_file_metadata(
        &self,
        file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        file.insert(&conn).await.map_err(Into::into).into_report()
    }

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
    async fn insert_file_metadata(
        &self,
        _file: storage::FileMetadataNew,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_file_metadata_by_merchant_id_file_id(
        &self,
        _merchant_id: &str,
        _file_id: &str,
    ) -> CustomResult<storage::FileMetadata, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_file_metadata_by_merchant_id_file_id(
        &self,
        _merchant_id: &str,
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
