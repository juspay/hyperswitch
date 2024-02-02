use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::report;
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    payout_attempt::{
        PayoutAttempt, PayoutAttemptNew, PayoutAttemptUpdate, PayoutAttemptUpdateInternal,
    },
    schema::payout_attempt::dsl,
    PgPooledConn, StorageResult,
};

impl PayoutAttemptNew {
    #[instrument(skip(conn))]
        /// Inserts a new PayoutAttempt into the database using the provided Postgres connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled Postgres connection
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the newly inserted PayoutAttempt if successful, or an error if the insertion fails.
    /// 
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PayoutAttempt> {
        generics::generic_insert(conn, self).await
    }
}

impl PayoutAttempt {
        /// Asynchronously finds a record by the specified merchant_id and payout_id in the database.
    pub async fn find_by_merchant_id_payout_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payout_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payout_id.eq(payout_id.to_owned())),
        )
        .await
    }

        /// Asynchronously updates a payout attempt in the database by the given merchant ID and payout ID,
    /// using the provided PayoutAttemptUpdate data. Returns a result containing the updated payout attempt,
    /// or an error if the update fails or the payout attempt is not found.
    pub async fn update_by_merchant_id_payout_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payout_id: &str,
        payout: PayoutAttemptUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payout_id.eq(payout_id.to_owned())),
            PayoutAttemptUpdateInternal::from(payout),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound).attach_printable("Error while updating payout")
        })
    }
}
