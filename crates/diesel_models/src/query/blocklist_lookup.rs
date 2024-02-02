use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    blocklist_lookup::{BlocklistLookup, BlocklistLookupNew},
    schema::blocklist_lookup::dsl,
    PgPooledConn, StorageResult,
};

impl BlocklistLookupNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new record into the database using the given connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a `PgPooledConn` which represents a connection to the database.
    /// 
    /// # Returns
    /// 
    /// The result of the insertion operation, wrapped in a `StorageResult` which may contain a `BlocklistLookup` if successful.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<BlocklistLookup> {
        generics::generic_insert(conn, self).await
    }
}

impl BlocklistLookup {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the provided merchant_id and fingerprint.
    /// Returns a StorageResult containing the found record if successful, or an error if the operation fails.
    pub async fn find_by_merchant_id_fingerprint(
        conn: &PgPooledConn,
        merchant_id: &str,
        fingerprint: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::fingerprint.eq(fingerprint.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously deletes a record from the database table associated with the given type `Self`
    /// by matching the `merchant_id` and `fingerprint` fields with the provided values.
    pub async fn delete_by_merchant_id_fingerprint(
        conn: &PgPooledConn,
        merchant_id: &str,
        fingerprint: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::fingerprint.eq(fingerprint.to_owned())),
        )
        .await
    }
}
