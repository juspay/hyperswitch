use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    file::{FileMetadata, FileMetadataNew},
    schema::file_metadata::dsl,
    PgPooledConn, StorageResult,
};

impl FileMetadataNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<FileMetadata> {
        generics::generic_insert(conn, self).await
    }
}

impl FileMetadata {
    #[instrument(skip(conn))]
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
}
