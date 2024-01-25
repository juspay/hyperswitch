use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use router_env::tracing::{self, instrument};

use crate::{query::generics, schema::user_roles::dsl, user_role::*, PgPooledConn, StorageResult};

impl UserRoleNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<UserRole> {
        generics::generic_insert(conn, self).await
    }
}

impl UserRole {
    pub async fn find_by_user_id(conn: &PgPooledConn, user_id: String) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::user_id.eq(user_id),
        )
        .await
    }

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
