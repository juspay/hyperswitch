use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    refund::{Refund, RefundNew, RefundUpdate, RefundUpdateInternal},
    schema::refund::dsl,
    PgPooledConn, StorageResult,
};

impl RefundNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a Refund into the database using the provided database connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled database connection
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the inserted `Refund` if successful, or an error if the insertion fails.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Refund> {
        generics::generic_insert(conn, self).await
    }
}

impl Refund {
    #[instrument(skip(conn))]
        /// Updates the current refund with the provided data in the database.
    /// 
    /// # Arguments
    /// * `conn` - The database connection
    /// * `refund` - The refund update data
    /// 
    /// # Returns
    /// The updated refund if successful, otherwise returns an error.
    pub async fn update(self, conn: &PgPooledConn, refund: RefundUpdate) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::refund_id
                .eq(self.refund_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            RefundUpdateInternal::from(refund),
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

    // This is required to be changed for KV.
    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given `merchant_id` and `refund_id`.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `merchant_id` - A string slice representing the merchant ID
    /// * `refund_id` - A string slice representing the refund ID
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the result of the operation
    pub async fn find_by_merchant_id_refund_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        refund_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::refund_id.eq(refund_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the provided merchant ID, connector refund ID, and connector name,
    /// and returns a result wrapped in a StorageResult enum.
    pub async fn find_by_merchant_id_connector_refund_id_connector(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_refund_id: &str,
        connector: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_refund_id.eq(connector_refund_id.to_owned()))
                .and(dsl::connector.eq(connector.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record by internal reference ID and merchant ID in the database.
    pub async fn find_by_internal_reference_id_merchant_id(
        conn: &PgPooledConn,
        internal_reference_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::internal_reference_id.eq(internal_reference_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a list of items by the given merchant ID and connector transaction ID
    pub async fn find_by_merchant_id_connector_transaction_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        connector_transaction_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::connector_transaction_id.eq(connector_transaction_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a list of items by payment ID and merchant ID in the database.
    pub async fn find_by_payment_id_merchant_id(
        conn: &PgPooledConn,
        payment_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_id.eq(payment_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }
}
