use diesel::{associations::HasTable, ExpressionMethods};
use error_stack::report;
use router_env::tracing::{self, instrument};

use crate::{
    errors::{self},
    query::generics,
    schema::users::dsl,
    user::*,
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
