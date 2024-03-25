use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, result::Error as DieselError, ExpressionMethods,
    JoinOnDsl, QueryDsl,
};
use error_stack::ResultExt;
use router_env::logger;
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
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<User> {
        generics::generic_insert(conn, self).await
    }
}

impl User {
    pub async fn find_by_user_email(conn: &PgPooledConn, user_email: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            users_dsl::email.eq(user_email.to_owned()),
        )
        .await
    }

    pub async fn find_by_user_id(conn: &PgPooledConn, user_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            users_dsl::user_id.eq(user_id.to_owned()),
        )
        .await
    }

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

    pub async fn delete_by_user_id(conn: &PgPooledConn, user_id: &str) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            users_dsl::user_id.eq(user_id.to_owned()),
        )
        .await
    }

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
            .map_err(|err| match err.current_context() {
                DieselError::NotFound => err.change_context(errors::DatabaseError::NotFound),
                _ => err.change_context(errors::DatabaseError::Others),
            })
    }
}
