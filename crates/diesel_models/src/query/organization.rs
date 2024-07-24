use common_utils::id_type;
use diesel::{associations::HasTable, ExpressionMethods};

use crate::{
    organization::*, query::generics, schema::organization::dsl, PgPooledConn, StorageResult,
};

impl OrganizationNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Organization> {
        generics::generic_insert(conn, self).await
    }
}

impl Organization {
    pub async fn find_by_org_id(
        conn: &PgPooledConn,
        org_id: id_type::OrganizationId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(conn, dsl::org_id.eq(org_id))
            .await
    }

    pub async fn update_by_org_id(
        conn: &PgPooledConn,
        org_id: id_type::OrganizationId,
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
