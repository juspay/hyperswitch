use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, query_dsl::JoinOnDsl, result::Error as DieselError,
    ExpressionMethods, QueryDsl,
};
use error_stack::{report, IntoReport};
use router_env::{
    logger,
    tracing::{self, instrument},
};

use crate::{
    errors::{self},
    query::generics,
    schema::{
        user_roles::{self, dsl as user_roles_dsl},
        users::dsl,
    },
    user::*,
    user_role::UserRole,
    PgPooledConn, StorageResult,
};

impl UserNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<User> {
        generics::generic_insert(conn, self).await
    }
}

impl User {
    pub async fn find_by_user_email(conn: &PgPooledConn, user_email: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::email.eq(user_email.to_owned()),
        )
        .await
    }

    pub async fn find_by_user_id(conn: &PgPooledConn, user_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::user_id.eq(user_id.to_owned()),
        )
        .await
    }

    pub async fn update_by_user_email(
        conn: &PgPooledConn,
        user_email: &str,
        user: UserUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::email.eq(user_email.to_owned()),
            UserUpdateInternal::from(user),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound).attach_printable("Error while updating user")
        })
    }

    pub async fn update_by_user_id(
        conn: &PgPooledConn,
        user_id: &str,
        user: UserUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::user_id.eq(user_id.to_owned()),
            UserUpdateInternal::from(user),
        )
        .await?
        .first()
        .cloned()
        .ok_or_else(|| {
            report!(errors::DatabaseError::NotFound).attach_printable("Error while updating user")
        })
    }

    pub async fn delete_by_user_id(conn: &PgPooledConn, user_id: &str) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::user_id.eq(user_id.to_owned()),
        )
        .await
    }
}

impl User {
    pub async fn find_joined_users_and_roles_by_merchant_id(
        conn: &PgPooledConn,
        mid: &str,
    ) -> StorageResult<Vec<(Self, UserRole)>> {
        let query = Self::table()
            .inner_join(user_roles::table.on(user_roles_dsl::user_id.eq(dsl::user_id)))
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

    pub async fn find_joined_user_and_role_by_user_id_and_merchant_id(
        conn: &PgPooledConn,
        user_id: &str,
        merchant_id: &str,
    ) -> StorageResult<(Self, UserRole)> {
        let query = Self::table()
            .inner_join(user_roles::table.on(user_roles_dsl::user_id.eq(dsl::user_id)))
            .filter(user_roles_dsl::user_id.eq(user_id.to_owned()))
            .filter(user_roles_dsl::merchant_id.eq(merchant_id.to_owned()));

        logger::debug!(query = %debug_query::<diesel::pg::Pg,_>(&query).to_string());

        query
            .get_result_async::<(Self, UserRole)>(conn)
            .await
            .into_report()
            .map_err(|err| match err.current_context() {
                DieselError::NotFound => err.change_context(errors::DatabaseError::NotFound),
                _ => err.change_context(errors::DatabaseError::Others),
            })
    }
}
