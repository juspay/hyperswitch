use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    blocklist::{Blocklist, BlocklistNew},
    schema::blocklist::dsl,
    PgPooledConn, StorageResult,
};

impl BlocklistNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new record into the database using the provided connection
    /// and returns the inserted blocklist entry on success.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Blocklist> {
        generics::generic_insert(conn, self).await
    }
}

impl Blocklist {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given merchant_id and fingerprint_id.
    pub async fn find_by_merchant_id_fingerprint_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Retrieves a list of data items of a specific kind associated with a merchant, with optional limit and offset.
    pub async fn list_by_merchant_id_data_kind(
        conn: &PgPooledConn,
        merchant_id: &str,
        data_kind: common_enums::BlocklistDataKind,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::data_kind.eq(data_kind.to_owned())),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously retrieves a list of items from the storage that match the specified merchant ID.
    pub async fn list_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
            None,
            Some(dsl::created_at.desc()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Deletes a record from the database based on the provided merchant ID and fingerprint ID. This method is asynchronous and returns a StorageResult indicating the success of the operation.
    pub async fn delete_by_merchant_id_fingerprint_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        fingerprint_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::fingerprint_id.eq(fingerprint_id.to_owned())),
        )
        .await
    }
}
