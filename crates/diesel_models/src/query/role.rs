use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use crate::{
    enums::RoleScope, query::generics, role::*, schema::roles::dsl, PgPooledConn, StorageResult,
};

impl RoleNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Role> {
        generics::generic_insert(conn, self).await
    }
}

impl Role {
    pub async fn find_by_role_id(conn: &PgPooledConn, role_id: &str) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id.eq(role_id.to_owned()),
        )
        .await
    }

    pub async fn find_by_role_id_in_merchant_scope(
        conn: &PgPooledConn,
        role_id: &str,
        merchant_id: &str,
        org_id: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id.eq(role_id.to_owned()).and(
                dsl::merchant_id.eq(merchant_id.to_owned()).or(dsl::org_id
                    .eq(org_id.to_owned())
                    .and(dsl::scope.eq(RoleScope::Organization))),
            ),
        )
        .await
    }

    pub async fn update_by_role_id(
        conn: &PgPooledConn,
        role_id: &str,
        role_update: RoleUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::role_id.eq(role_id.to_owned()),
            RoleUpdateInternal::from(role_update),
        )
        .await
    }

    pub async fn delete_by_role_id(conn: &PgPooledConn, role_id: &str) -> StorageResult<Self> {
        generics::generic_delete_one_with_result::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id.eq(role_id.to_owned()),
        )
        .await
    }

    pub async fn list_roles(
        conn: &PgPooledConn,
        merchant_id: &str,
        org_id: &str,
    ) -> StorageResult<Vec<Self>> {
        let predicate = dsl::merchant_id.eq(merchant_id.to_owned()).or(dsl::org_id
            .eq(org_id.to_owned())
            .and(dsl::scope.eq(RoleScope::Organization)));

        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            predicate,
            None,
            None,
            Some(dsl::last_modified_at.asc()),
        )
        .await
    }
}
