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

    /// The definition for a name and product, if there is one.
    ///
    /// Optional rather than a not-found error: the caller asking is checking whether an alert
    /// exists before writing an enablement row for it, and "no" is an answer rather than a
    /// failure.
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

    /// Every definition, for the alert manager's per-run read and for the config screen's list.
    ///
    /// Unpaginated. There is one row per detector per product — tens of them, not thousands — and
    /// a page cursor would be a second thing to get right for a list that fits on a screen.
    ///
    /// The predicate is `name IS NOT NULL`, which every row satisfies because the column is `NOT
    /// NULL`. `generic_filter` requires a predicate and this is the honest spelling of "all of
    /// them"; a comparison against a sentinel id would exclude a real row the day one happened to
    /// match it.
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
