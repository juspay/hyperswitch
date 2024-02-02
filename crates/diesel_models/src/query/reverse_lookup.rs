use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
    schema::reverse_lookup::dsl,
    PgPooledConn, StorageResult,
};

impl ReverseLookupNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts the current object into the database using the provided connection, and returns the result as a `StorageResult` containing a `ReverseLookup` object.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ReverseLookup> {
        generics::generic_insert(conn, self).await
    }
    #[instrument(skip(conn))]
        /// Asynchronously inserts a batch of ReverseLookup objects into the database using the provided database connection.
    ///
    /// # Arguments
    ///
    /// * `reverse_lookups` - A vector of ReverseLookup objects to be inserted into the database.
    /// * `conn` - A reference to a pooled database connection.
    ///
    /// # Returns
    ///
    /// This method returns a `StorageResult` indicating success or an error.
    ///
    pub async fn batch_insert(
        reverse_lookups: Vec<Self>,
        conn: &PgPooledConn,
    ) -> StorageResult<()> {
        generics::generic_insert::<_, _, ReverseLookup>(conn, reverse_lookups).await?;
        Ok(())
    }
}
impl ReverseLookup {
        /// Asynchronously finds a record in the database by its lookup ID using the given database connection.
    pub async fn find_by_lookup_id(lookup_id: &str, conn: &PgPooledConn) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::lookup_id.eq(lookup_id.to_owned()),
        )
        .await
    }
}
