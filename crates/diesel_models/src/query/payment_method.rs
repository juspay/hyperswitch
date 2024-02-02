use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods, Table};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    payment_method::{self, PaymentMethod, PaymentMethodNew},
    schema::payment_methods::dsl,
    PgPooledConn, StorageResult,
};

impl PaymentMethodNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a PaymentMethod into the database using the provided database connection pool.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a `PgPooledConn` database connection
    /// 
    /// # Returns
    /// 
    /// An asynchronous `StorageResult` containing the inserted `PaymentMethod` if successful, or an error if the insertion fails.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<PaymentMethod> {
        generics::generic_insert(conn, self).await
    }
}

impl PaymentMethod {
    #[instrument(skip(conn))]
        /// Deletes a row from the database based on the provided payment method ID.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - The database connection
    /// * `payment_method_id` - The ID of the payment method to be used as a filter for deletion
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the result of the deletion operation
    pub async fn delete_by_payment_method_id(
        conn: &PgPooledConn,
        payment_method_id: String,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, Self>(
            conn,
            dsl::payment_method_id.eq(payment_method_id),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Deletes a record from the database based on the provided merchant ID and payment method ID.
    ///
    /// # Arguments
    ///
    /// * `conn` - A connection to the database
    /// * `merchant_id` - The ID of the merchant
    /// * `payment_method_id` - The ID of the payment method
    ///
    /// # Returns
    ///
    /// A result indicating success or failure of the delete operation
    pub async fn delete_by_merchant_id_payment_method_id(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_method_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, Self>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::payment_method_id.eq(payment_method_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database based on the given payment method ID.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `payment_method_id` - A reference to the payment method ID to search by
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the found record, if any
    ///
    pub async fn find_by_payment_method_id(
        conn: &PgPooledConn,
        payment_method_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_method_id.eq(payment_method_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously queries the database to find all records with a specific merchant ID.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `merchant_id` - The merchant ID to filter records by
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing a vector of records that match the specified merchant ID
    ///
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
            None,
            None,
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds records by customer_id and merchant_id in the database.
    pub async fn find_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::customer_id
                .eq(customer_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
            None,
            None,
            None,
        )
        .await
    }

        /// Asynchronously updates the current object with the given payment method ID using the provided database connection.
    pub async fn update_with_payment_method_id(
        self,
        conn: &PgPooledConn,
        payment_method: payment_method::PaymentMethodUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::payment_method_id.eq(self.payment_method_id.to_owned()),
            payment_method::PaymentMethodUpdateInternal::from(payment_method),
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
}
