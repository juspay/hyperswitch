use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use crate::{
    observability::{
        alerts_info::{AlertsInfo, AlertsInfoNew, AlertsInfoUpdate},
        schema::alerts_info::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl AlertsInfoNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertsInfo> {
        generics::generic_insert(conn, self).await
    }
}

impl AlertsInfo {
    pub async fn find_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: uuid::Uuid,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(conn, dsl::id.eq(id)).await
    }

    pub async fn find_optional_by_name_and_product(
        conn: &DatabaseConnectionWithContext<'_>,
        name: String,
        product: String,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::name.eq(name).and(dsl::product.eq(product)),
        )
        .await
    }

    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::name.is_not_null(),
            None,
            None,
            Some((dsl::product.asc(), dsl::name.asc())),
        )
        .await
    }

    pub async fn update_by_id(
        conn: &DatabaseConnectionWithContext<'_>,
        id: uuid::Uuid,
        update: AlertsInfoUpdate,
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
