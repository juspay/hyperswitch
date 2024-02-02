use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    payment_intent::{
        PaymentIntent, PaymentIntentNew, PaymentIntentUpdate, PaymentIntentUpdateInternal,
    },
    schema::payment_intent::dsl,
    PgPooledConn, StorageResult,
};

impl PaymentIntentNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a PaymentIntent into the database using the provided PostgreSQL pooled connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a `PgPooledConn` which represents a pooled connection to a PostgreSQL database.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the inserted `PaymentIntent` if the insertion is successful, otherwise an error.
    /// 
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentIntent> {
        generics::generic_insert(conn, self).await
    }
}

impl PaymentIntent {
    #[instrument(skip(conn))]
        /// Asynchronously updates a payment intent in the database using the provided connection and payment intent update. 
    /// Returns a StorageResult containing the updated payment intent if successful, or an error if the update fails.
    pub async fn update(
        self,
        conn: &PgPooledConn,
        payment_intent: PaymentIntentUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::payment_id
                .eq(self.payment_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            PaymentIntentUpdateInternal::from(payment_intent),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            Ok(mut payment_intents) => payment_intents
                .pop()
                .ok_or(error_stack::report!(errors::DatabaseError::NotFound)),
        }
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record by payment ID and merchant ID in the database.
    pub async fn find_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds an optional record by the given payment ID and merchant ID in the database.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled database connection
    /// * `payment_id` - A reference to a string containing the payment ID
    /// * `merchant_id` - A reference to a string containing the merchant ID
    /// 
    /// # Returns
    /// 
    /// An asynchronous `StorageResult` containing an optional value of `Self`, where `Self` is the type implementing the method.
    /// 
    pub async fn find_optional_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
        )
        .await
    }
}
