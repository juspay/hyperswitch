use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    file::{FileMetadata, FileMetadataNew, FileMetadataUpdate, FileMetadataUpdateInternal},
    schema::file_metadata::dsl,
    PgPooledConn, StorageResult,
};

impl FileMetadataNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new record into the database using the provided database connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled database connection
    /// 
    /// # Returns
    /// 
    /// A `FileMetadata` representing the newly inserted record, wrapped in a `StorageResult`
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<FileMetadata> {
        generics::generic_insert(conn, self).await
    }
}

impl FileMetadata {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the storage by the given merchant ID and file ID.
    /// Returns a StorageResult containing the found record, if any.
    pub async fn find_by_merchant_id_file_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        file_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::file_id.eq(file_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously deletes a record from the storage by matching the merchant_id and file_id.
    pub async fn delete_by_merchant_id_file_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        file_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::file_id.eq(file_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously updates the file metadata in the storage using the provided connection. It matches the file_id with the given file_metadata and updates the corresponding fields, returning the updated storage result. If no fields are found to update, it returns the original file metadata. If an error occurs during the update process, it returns the error.
    pub async fn update(
        self,
        conn: &PgPooledConn,
        file_metadata: FileMetadataUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::file_id.eq(self.file_id.to_owned()),
            FileMetadataUpdateInternal::from(file_metadata),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }
}
