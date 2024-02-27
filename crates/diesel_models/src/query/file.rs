use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
use crate::{
    errors,
    file::{FileMetadata, FileMetadataNew, FileMetadataUpdate, FileMetadataUpdateInternal},
    schema::file_metadata::dsl,
    PgPooledConn, StorageResult,
};

impl FileMetadataNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<FileMetadata> {
        generics::generic_insert(conn, self).await
    }
}

impl FileMetadata {
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
