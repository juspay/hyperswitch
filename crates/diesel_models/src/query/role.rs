use diesel::{associations::HasTable, ExpressionMethods};
use router_env::tracing::{self, instrument};

use crate::{query::generics, role::*, schema::roles::dsl, PgPooledConn, StorageResult};

impl RoleNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Role> {
        generics::generic_insert(conn, self).await
    }
}

impl Role {
    pub async fn find_by_role_id(conn: &PgPooledConn, role_id: String) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::role_id.eq(role_id),
        )
        .await
    }

    pub async fn delete_by_role_id(conn: &PgPooledConn, role_id: String) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(conn, dsl::role_id.eq(role_id))
            .await
    }
}
