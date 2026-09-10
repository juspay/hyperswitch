use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use crate::{
    hierarchical_resource::{
        HierarchicalResource, HierarchicalResourceNew, HierarchicalResourceUpdateInternal,
    },
    query::generics,
    schema::hierarchical_resources::dsl,
    DatabaseConnectionWithContext, StorageResult,
};

impl HierarchicalResourceNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<HierarchicalResource> {
        generics::generic_insert(conn, self).await
    }
}

impl HierarchicalResource {
    pub async fn find_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: common_utils::id_type::ResourceId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(conn, dsl::id.eq(id)).await
    }

    pub async fn list_by_scope_id_and_resource_type(
        conn: &DatabaseConnectionWithContext<'_>,
        scope_id: String,
        resource_type: String,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::scope_id
                .eq(scope_id)
                .and(dsl::resource_type.eq(resource_type)),
            None,
            None,
            Some(dsl::created_at.asc()),
        )
        .await
    }

    pub async fn update_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: common_utils::id_type::ResourceId,
        update: HierarchicalResourceUpdateInternal,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(conn, dsl::id.eq(id), update)
        .await
    }
}
