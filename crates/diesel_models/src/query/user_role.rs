use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::tracing::{self, instrument};

use crate::{query::generics, schema::user_roles::dsl, user_role::*, PgPooledConn, StorageResult};

impl UserRoleNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts a new UserRole into the database using the provided PgPooledConn.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to PgPooledConn, which represents a pooled connection to the Postgres database.
    /// 
    /// # Returns
    /// 
    /// A StorageResult containing the inserted UserRole if successful, or an error if the insertion fails.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<UserRole> {
        generics::generic_insert(conn, self).await
    }
}

impl UserRole {
        /// Asynchronously finds a record in the database by the given user ID.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled Postgres connection.
    /// * `user_id` - A string representing the user ID to search for.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the found record if successful, or an error if the operation fails.
    pub async fn find_by_user_id(conn: &PgPooledConn, user_id: String) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::user_id.eq(user_id),
        )
        .await
    }

        /// Asynchronously finds a record by user ID and merchant ID in the database.
    pub async fn find_by_user_id_merchant_id(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }

        /// Updates a user role by user ID and merchant ID, using the provided update object. 
    /// Returns a StorageResult containing the updated user role if successful.
    pub async fn update_by_user_id_merchant_id(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: String,
        update: UserRoleUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::merchant_id.eq(merchant_id)),
            UserRoleUpdateInternal::from(update),
        )
        .await
    }

        /// Deletes a record from the database based on the provided user_id and merchant_id.
    ///
    /// # Arguments
    ///
    /// * `conn` - The database connection
    /// * `user_id` - The user ID of the record to be deleted
    /// * `merchant_id` - The merchant ID of the record to be deleted
    ///
    /// # Returns
    ///
    /// A `StorageResult` indicating whether the deletion was successful
    /// 
    pub async fn delete_by_user_id_merchant_id(
        conn: &PgPooledConn,
        user_id: String,
        merchant_id: String,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::user_id
                .eq(user_id)
                .and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }

        /// Retrieves a list of items from the database based on the specified user ID.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled database connection.
    /// * `user_id` - A String representing the user ID to filter the items by.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing a vector of items that match the specified user ID.
    pub async fn list_by_user_id(conn: &PgPooledConn, user_id: String) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::user_id.eq(user_id),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }
}
