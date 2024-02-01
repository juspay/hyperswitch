use diesel::{associations::HasTable, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    locker_mock_up::{LockerMockUp, LockerMockUpNew},
    schema::locker_mock_up::dsl,
    PgPooledConn, StorageResult,
};

impl LockerMockUpNew {
    #[instrument(skip(conn))]
        /// Inserts a LockerMockUp into the database using the provided database connection.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<LockerMockUp> {
        generics::generic_insert(conn, self).await
    }
}

impl LockerMockUp {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given card ID and returns it as a result.
    pub async fn find_by_card_id(conn: &PgPooledConn, card_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::card_id.eq(card_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously deletes a record from the database by the given card ID.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled connection to the database.
    /// * `card_id` - A string slice representing the card ID of the record to be deleted.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the result of the delete operation, where `Ok` holds the deleted record
    /// and `Err` holds the error encountered during the delete operation.
    pub async fn delete_by_card_id(conn: &PgPooledConn, card_id: &str) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::card_id.eq(card_id.to_owned()),
        )
        .await
    }
}
