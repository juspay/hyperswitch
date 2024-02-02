use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    capture::{Capture, CaptureNew, CaptureUpdate, CaptureUpdateInternal},
    errors,
    schema::captures::dsl,
    PgPooledConn, StorageResult,
};

impl CaptureNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new record into the database using the provided PostgreSQL pooled connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a PostgreSQL pooled connection
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the newly inserted `Capture` if successful, or an error if the insertion fails.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Capture> {
        generics::generic_insert(conn, self).await
    }
}

impl Capture {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given capture ID.
    pub async fn find_by_capture_id(conn: &PgPooledConn, capture_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::capture_id.eq(capture_id.to_owned()),
        )
        .await
    }
    #[instrument(skip(conn))]
        /// Updates the current object with the given capture ID using the provided database connection and capture update data.
    /// Returns a `StorageResult` containing the updated object on success, or an error on failure.
    pub async fn update_with_capture_id(
        self,
        conn: &PgPooledConn,
        capture: CaptureUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::capture_id.eq(self.capture_id.to_owned()),
            CaptureUpdateInternal::from(capture),
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

    #[instrument(skip(conn))]
        /// Asynchronously finds all records with a specific merchant ID, payment ID, and authorized attempt ID in the database.
    pub async fn find_all_by_merchant_id_payment_id_authorized_attempt_id(
        merchant_id: &str,
        payment_id: &str,
        authorized_attempt_id: &str,
        conn: &PgPooledConn,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::authorized_attempt_id
                .eq(authorized_attempt_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }
}
