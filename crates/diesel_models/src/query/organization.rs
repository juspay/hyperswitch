use diesel::{associations::HasTable, ExpressionMethods};
use router_env::tracing::{self, instrument};

use crate::{
    organization::*, query::generics, schema::organization::dsl, PgPooledConn, StorageResult,
};

impl OrganizationNew {
    #[instrument(skip(conn))]
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Organization> {
        generics::generic_insert(conn, self).await
    }
}

impl Organization {
    pub async fn find_by_org_id(conn: &PgPooledConn, org_id: String) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(conn, dsl::org_id.eq(org_id))
            .await
    }

    pub async fn update_by_org_id(
        conn: &PgPooledConn,
        org_id: String,
        update: OrganizationUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::org_id.eq(org_id),
            OrganizationUpdateInternal::from(update),
        )
        .await
    }
}
