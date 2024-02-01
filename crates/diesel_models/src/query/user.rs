use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, result::Error as DieselError, ExpressionMethods,
    JoinOnDsl, QueryDsl,
};
use error_stack::IntoReport;
use router_env::{
    logger,
    tracing::{self, instrument},
};
pub mod sample_data;

use crate::{
    errors::{self},
    query::generics,
    schema::{
        user_roles::{self, dsl as user_roles_dsl},
        users::dsl as users_dsl,
    },
    user::*,
    user_role::UserRole,
    PgPooledConn, StorageResult,
};

impl UserNew {
    #[instrument(skip(conn))]
        /// Asynchronously inserts the current user into the database using the provided database connection and returns a `StorageResult` containing the inserted user.
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<User> {
        generics::generic_insert(conn, self).await
    }
}

impl User {
        /// Asynchronously finds a record in the database by the user's email address.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled database connection.
    /// * `user_email` - The email address of the user to search for.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the found record if successful, or an error if the operation fails.
    pub async fn find_by_user_email(conn: &PgPooledConn, user_email: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            users_dsl::email.eq(user_email.to_owned()),
        )
        .await
    }

        /// Asynchronously finds a record in the database by the given user ID.
    /// 
    /// # Arguments
    /// 
    /// * `conn` - A reference to a pooled connection to the database.
    /// * `user_id` - A string slice representing the user ID to search for.
    /// 
    /// # Returns
    /// 
    /// A `StorageResult` containing the found record if successful, or an error if the operation fails.
    /// 
    pub async fn find_by_user_id(conn: &PgPooledConn, user_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            users_dsl::user_id.eq(user_id.to_owned()),
        )
        .await
    }

        /// This method updates a user in the database based on the user's ID. It takes a database connection, the user's ID, and the user update information as input parameters. It then uses the generic_update_with_unique_predicate_get_result function to perform the update operation and returns a StorageResult containing the updated user information.
    pub async fn update_by_user_id(
        conn: &PgPooledConn,
        user_id: &str,
        user_update: UserUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            users_dsl::user_id.eq(user_id.to_owned()),
            UserUpdateInternal::from(user_update),
        )
        .await
    }

        /// Updates a user's information based on their email address.
    /// 
    /// # Arguments
    /// * `conn` - A reference to a pooled PostgreSQL connection.
    /// * `user_email` - The email address of the user to be updated.
    /// * `user_update` - The new information to update the user with.
    /// 
    /// # Returns
    /// A `StorageResult` containing the result of the update operation.
    /// 
    pub async fn update_by_user_email(
        conn: &PgPooledConn,
        user_email: &str,
        user_update: UserUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            users_dsl::email.eq(user_email.to_owned()),
            UserUpdateInternal::from(user_update),
        )
        .await
    }

        /// Asynchronously deletes a record from the database table associated with the current struct based on the provided user ID.
    /// 
    /// # Arguments
    /// * `conn` - A reference to a pooled connection to the PostgreSQL database
    /// * `user_id` - A string slice representing the user ID of the record to be deleted
    /// 
    /// # Returns
    /// A `StorageResult` containing a boolean value indicating whether the deletion was successful or not
    pub async fn delete_by_user_id(conn: &PgPooledConn, user_id: &str) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            users_dsl::user_id.eq(user_id.to_owned()),
        )
        .await
    }

        /// Retrieves a list of joined users and their roles associated with a specific merchant ID from the database.
    pub async fn find_joined_users_and_roles_by_merchant_id(
        conn: &PgPooledConn,
        mid: &str,
    ) -> StorageResult<Vec<(Self, UserRole)>> {
        let query = Self::table()
            .inner_join(user_roles::table.on(user_roles_dsl::user_id.eq(users_dsl::user_id)))
            .filter(user_roles_dsl::merchant_id.eq(mid.to_owned()));

        logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

        query
            .get_results_async::<(Self, UserRole)>(conn)
            .await
            .into_report()
            .map_err(|err| match err.current_context() {
                DieselError::NotFound => err.change_context(errors::DatabaseError::NotFound),
                _ => err.change_context(errors::DatabaseError::Others),
            })
    }
}
