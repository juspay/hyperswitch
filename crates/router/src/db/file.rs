use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait FileInterface {
    async fn insert_file(
        &self,
        file: storage::FileNew,
    ) -> CustomResult<storage::File, errors::StorageError>;

    async fn find_file_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
        file_id: &str,
    ) -> CustomResult<storage::File, errors::StorageError>;

    async fn delete_file_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
        file_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl FileInterface for Store {
    async fn insert_file(
        &self,
        file: storage::FileNew,
    ) -> CustomResult<storage::File, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        file.insert(&conn).await.map_err(Into::into).into_report()
    }

    async fn find_file_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
        file_id: &str,
    ) -> CustomResult<storage::File, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::File::find_by_merchant_id_file_id(&conn, merchant_id, file_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn delete_file_by_merchant_id_file_id(
        &self,
        merchant_id: &str,
        file_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::File::delete_by_merchant_id_file_id(&conn, merchant_id, file_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl FileInterface for MockDb {
    async fn insert_file(
        &self,
        _file: storage::FileNew,
    ) -> CustomResult<storage::File, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_file_by_merchant_id_file_id(
        &self,
        _merchant_id: &str,
        _file_id: &str,
    ) -> CustomResult<storage::File, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_file_by_merchant_id_file_id(
        &self,
        _merchant_id: &str,
        _file_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
