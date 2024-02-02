use diesel::{associations::HasTable, ExpressionMethods, Table};
use router_env::{instrument, tracing};

use super::generics;
use crate::{
    errors,
    merchant_account::{MerchantAccount, MerchantAccountNew, MerchantAccountUpdateInternal},
    schema::merchant_account::dsl,
    PgPooledConn, StorageResult,
};

impl MerchantAccountNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a MerchantAccount into the database using the provided database connection.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<MerchantAccount> {
        generics::generic_insert(conn, self).await
    }
}

impl MerchantAccount {
    #[instrument(skip(conn))]
        /// Asynchronously updates the current instance of the struct in the database with the provided `merchant_account`, using the given database connection `conn`. 
    /// Returns a `StorageResult` with the updated instance on success, or an error on failure. 
    pub async fn update(
        self,
        conn: &PgPooledConn,
        merchant_account: MerchantAccountUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            self.id,
            merchant_account,
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

        /// Asynchronously updates a specific merchant account with the given fields in the database using the provided connection.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled Postgres connection
    /// * `merchant_id` - A string slice representing the merchant ID
    /// * `merchant_account` - A `MerchantAccountUpdateInternal` struct containing the fields to be updated
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the result of the update operation
    pub async fn update_with_specific_fields(
        conn: &PgPooledConn,
        merchant_id: &str,
        merchant_account: MerchantAccountUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            merchant_account,
        )
        .await
    }

        /// Asynchronously deletes a record from the database by the given merchant_id.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled connection to the database
    /// * `merchant_id` - A string slice representing the merchant_id of the record to be deleted
    ///
    /// # Returns
    ///
    /// A `StorageResult` indicating whether the record was successfully deleted
    ///
    pub async fn delete_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip(conn))]
        /// Asynchronously finds a record in the database by the given merchant ID.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - The database connection pool
    /// * `merchant_id` - The merchant ID to search for
    /// 
    /// # Returns
    /// 
    /// A Result containing the found record or an error if the operation fails.
    pub async fn find_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
        )
        .await
    }

    #[instrument(skip_all)]
        /// Asynchronously finds a record in the database using the given publishable key.
    ///
    /// # Arguments
    ///
    /// * `conn` - A reference to a pooled connection to the database.
    /// * `publishable_key` - A string slice representing the publishable key to search for.
    ///
    /// # Returns
    ///
    /// A `StorageResult` containing the found record if successful, or an error if the operation fails.
    ///
    pub async fn find_by_publishable_key(
        conn: &PgPooledConn,
        publishable_key: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::publishable_key.eq(publishable_key.to_owned()),
        )
        .await
    }

    #[instrument(skip_all)]
        /// Retrieves a list of items filtered by the provided organization ID from the database.
    pub async fn list_by_organization_id(
        conn: &PgPooledConn,
        organization_id: &str,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::organization_id.eq(organization_id.to_owned()),
            None,
            None,
            None,
        )
        .await
    }

    #[instrument(skip_all)]
        /// Asynchronously retrieves multiple merchant accounts based on the provided list of merchant IDs.
    pub async fn list_multiple_merchant_accounts(
        conn: &PgPooledConn,
        merchant_ids: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as Table>::PrimaryKey,
            _,
        >(
            conn,
            dsl::merchant_id.eq_any(merchant_ids),
            None,
            None,
            None,
        )
        .await
    }
}
