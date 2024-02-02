use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    address::{Address, AddressNew, AddressUpdateInternal},
    errors,
    schema::address::dsl,
    PgPooledConn, StorageResult,
};

impl AddressNew {
    #[instrument(skip(conn))]
        /// Inserts an Address object into the database using the provided PgPooledConn connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a PgPooledConn, which represents a pooled connection to a Postgres database.
    /// 
    /// # Returns
    /// 
    /// The inserted Address object if the operation is successful, or an error if the operation fails.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Address> {
        generics::generic_insert(conn, self).await
    }
}

impl Address {
    #[instrument(skip(conn))]
        /// Asynchronously finds a record by its address ID in the database.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled PostgreSQL connection.
    /// * `address_id` - The ID of the address to search for.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the found record if successful, or an error if the operation fails.
    /// 
    pub async fn find_by_address_id<'a>(
            conn: &PgPooledConn,
            address_id: &str,
        ) -> StorageResult<Self> {
            generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, address_id.to_owned())
                .await
        }

    #[instrument(skip(conn))]
        /// Asynchronously updates an address in the database based on its ID. If the address with the given ID doesn't exist, an error is returned. If no fields are provided to update, the existing address is looked up and returned. 
    pub async fn update_by_address_id(
        conn: &PgPooledConn,
        address_id: String,
        address: AddressUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            address_id.clone(),
            address,
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => {
                    Err(error.attach_printable("Address with the given ID doesn't exist"))
                }
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        address_id.clone(),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
        /// Asynchronously updates the current instance in the database using the provided connection and address update internal data. 
    /// Returns a Result containing the updated instance if successful, or an error if the update fails.
    pub async fn update(
        self,
        conn: &PgPooledConn,
        address_update_internal: AddressUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::address_id.eq(self.address_id.clone()),
            address_update_internal,
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
        /// Deletes a record from the database by its address_id.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled database connection
    /// * `address_id` - The address_id of the record to be deleted
    ///
    /// # Returns
    ///
    /// A `StorageResult` indicating whether the record was successfully deleted
    pub async fn delete_by_address_id(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::address_id.eq(address_id.to_owned()),
        )
        .await
    }

        /// Update a record in the database by the specified merchant ID and customer ID, with the provided address information.
    /// This method performs a generic update operation on the database table associated with the struct, using the provided connection and criteria.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - The PostgreSQL connection pool
    /// * `customer_id` - The customer ID
    /// * `merchant_id` - The merchant ID
    /// * `address` - The updated address information
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing a vector of the updated records on success, or an error on failure.
    pub async fn update_by_merchant_id_customer_id(
        conn: &PgPooledConn,
        customer_id: &str,
        merchant_id: &str,
        address: AddressUpdateInternal,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::customer_id.eq(customer_id.to_owned())),
            address,
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given merchant ID, payment ID, and address ID.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `merchant_id` - A string slice representing the merchant ID
    /// * `payment_id` - A string slice representing the payment ID
    /// * `address_id` - A string slice representing the address ID
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the result of the operation
    ///
    pub async fn find_by_merchant_id_payment_id_address_id<'a>(
        conn: &PgPooledConn,
        merchant_id: &str,
        payment_id: &str,
        address_id: &str,
    ) -> StorageResult<Self> {
        match generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::payment_id
                .eq(payment_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned()))
                .and(dsl::address_id.eq(address_id.to_owned())),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NotFound => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        address_id.to_owned(),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds an optional instance of the current type by the specified address ID from the database.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled PostgreSQL connection
    /// * `address_id` - The address ID to search for
    ///
    /// # Returns
    ///
    /// An optional result containing the found instance, or None if no instance is found
    pub async fn find_optional_by_address_id<'a>(
        conn: &PgPooledConn,
        address_id: &str,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            address_id.to_owned(),
        )
        .await
    }
}
