use common_utils::pii;
use diesel::{associations::HasTable, ExpressionMethods};

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

    pub async fn find_users_by_user_ids(
        conn: &PgPooledConn,
        user_ids: Vec<String>,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<
            <Self as HasTable>::Table,
            _,
            <<Self as HasTable>::Table as diesel::Table>::PrimaryKey,
            _,
        >(conn, users_dsl::user_id.eq_any(user_ids), None, None, None)
        .await
    }
}
