use common_utils::pii;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use masking::Secret;

pub mod sample_data;
pub mod theme;

use crate::{
    query::generics, schema::users::dsl as users_dsl, user::*, PgPooledConn, StorageResult,
};

impl UserNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<User> {
        generics::generic_insert(conn, self).await
    }
}

impl User {
    pub async fn find_active_by_user_email(
        conn: &PgPooledConn,
        user_email: &pii::Email,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            users_dsl::email
                .eq(user_email.to_owned())
                .and(users_dsl::is_active.eq(true)),
        )
        .await
    }

    pub async fn find_by_user_email(
        conn: &PgPooledConn,
        user_email: &pii::Email,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            users_dsl::email.eq(user_email.to_owned()),
        )
        .await
    }

    pub async fn find_active_by_user_id(conn: &PgPooledConn, user_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            users_dsl::user_id
                .eq(user_id.to_owned())
                .and(users_dsl::is_active.eq(true)),
        )
        .await
    }

    pub async fn update_active_by_user_id(
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
            users_dsl::user_id
                .eq(user_id.to_owned())
                .and(users_dsl::is_active.eq(true)),
            UserUpdateInternal::from(user_update),
        )
        .await
    }

    pub async fn update_active_by_user_email(
        conn: &PgPooledConn,
        user_email: &pii::Email,
        user_update: UserUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            users_dsl::email
                .eq(user_email.to_owned())
                .and(users_dsl::is_active.eq(true)),
            UserUpdateInternal::from(user_update),
        )
        .await
    }

    pub async fn find_active_users_by_user_ids(
        conn: &PgPooledConn,
        user_ids: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as diesel::Table>::PrimaryKey,
            _,
        >(
            conn,
            users_dsl::user_id
                .eq_any(user_ids)
                .and(users_dsl::is_active.eq(true)),
            None,
            None,
            None,
        )
        .await
    }

    pub async fn reactivate_by_user_id(
        conn: &PgPooledConn,
        user_id: &str,
        new_name: Option<String>,
        new_password: Option<Secret<String>>,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            users_dsl::user_id
                .eq(user_id.to_owned())
                .and(users_dsl::is_active.eq(false)),
            UserUpdateInternal::from(UserUpdate::ActiveUpdate {
                new_name,
                new_password,
            }),
        )
        .await
    }
}
