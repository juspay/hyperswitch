use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::tracing::{self, instrument};

use crate::{
    errors, fraud_check::*, query::generics, schema::fraud_check::dsl, PgPooledConn, StorageResult,
};

impl FraudCheckNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new record into the database using the provided database connection.
    /// 
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the result of the insert operation, which is a `FraudCheck` object if successful
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<FraudCheck> {
        generics::generic_insert(conn, self).await
    }
}

impl FraudCheck {
    #[instrument(skip(conn))]
        /// Updates the fraud check with a specific attempt ID in the database using the provided connection.
    /// If the update is successful, it returns the updated fraud check. If there are no fields to update, it returns the original fraud check.
    /// If an error occurs during the update, it returns a StorageResult containing the error.
    pub async fn update_with_attempt_id(
        self,
        conn: &PgPooledConn,
        fraud_check: FraudCheckUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::attempt_id
                .eq(self.attempt_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            FraudCheckUpdateInternal::from(fraud_check),
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

        /// Asynchronously retrieves a record from the database based on the provided payment_id and merchant_id.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `payment_id` - A String representing the payment ID
    /// * `merchant_id` - A String representing the merchant ID
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the retrieved record, if successful.
    pub async fn get_with_payment_id(
        conn: &PgPooledConn,
        payment_id: String,
        merchant_id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id
                .eq(payment_id)
                .and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }

        /// Asynchronously fetches a record from the database with the provided payment_id and merchant_id if present.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - The database connection
    /// * `payment_id` - The payment_id to search for
    /// * `merchant_id` - The merchant_id to search for
    /// 
    /// # Returns
    /// 
    /// Returns a `StorageResult` with an `Option` containing the record if found, or `None` if not found.
    pub async fn get_with_payment_id_if_present(
        conn: &PgPooledConn,
        payment_id: String,
        merchant_id: String,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id
                .eq(payment_id)
                .and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }
}
