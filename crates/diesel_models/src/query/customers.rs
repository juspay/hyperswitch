use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    customers::{Customer, CustomerNew, CustomerUpdateInternal},
    errors,
    schema::customers::dsl,
    PgPooledConn, StorageResult,
};

impl CustomerNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new customer into the database using the provided database connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the newly inserted `Customer` if successful, or an error if the insertion fails.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Customer> {
        generics::generic_insert(conn, self).await
    }
}

impl Customer {
    #[instrument(skip(conn))]
        /// Updates a customer's information for a specific merchant by their IDs. If the update operation results in a DatabaseError::NoFieldsToUpdate, it will attempt to find the customer's information for the specified merchant. Returns a StorageResult containing the updated customer information if successful, or an error if the update operation fails.
    pub async fn update_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: String,
        merchant_id: String,
        customer: CustomerUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (customer_id.clone(), merchant_id.clone()),
            customer,
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        (customer_id, merchant_id),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
        /// Deletes a record from the database based on the provided customer_id and merchant_id.
    /// Returns a boolean indicating success or failure.
    pub async fn delete_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::customer_id
                .eq(customer_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record by the given customer ID and merchant ID in the database.
    pub async fn find_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
            conn,
            (customer_id.to_owned(), merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously retrieves a list of items from the storage by their merchant ID.
    pub async fn list_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            None,
            None,
            Some(dsl::created_at),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds an optional instance of the current type by the given customer ID and merchant ID in the database using the provided database connection. Returns a `StorageResult` containing either `Some` instance if found or `None` if not found.
    pub async fn find_optional_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            (customer_id.to_owned(), merchant_id.to_owned()),
        )
        .await
    }
}
