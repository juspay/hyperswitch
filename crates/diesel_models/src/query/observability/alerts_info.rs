use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::ReportSwitchExt;
use diesel::{
    associations::HasTable, BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl,
};
use error_stack::ResultExt;

use crate::{
    observability::{
        alerts_info::{AlertsInfo, AlertsInfoNew, AlertsInfoUpdate, AlertsInfoUpdateInternal},
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
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, id).await
    }

    pub async fn find_optional_is_enabled_by_name_product(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        product: &str,
    ) -> StorageResult<Option<Option<bool>>> {
        let query = <Self as HasTable>::table()
            .filter(
                dsl::name
                    .eq(name.to_owned())
                    .and(dsl::product.eq(product.to_owned())),
            )
            .select(dsl::is_enabled);

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::FindOne,
            query.get_result_async(conn.raw_connection()),
        )
        .await
        .optional()
        .attach_printable("Failed to find whether the alert definition is enabled")
        .switch()
    }

    pub async fn list(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::name.ne_all(vec![""]),
            None,
            None,
            Some((dsl::product.asc(), dsl::name.asc())),
        )
        .await
    }

    pub async fn list_is_enabled(
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<Vec<(String, String, Option<bool>)>> {
        let query = <Self as HasTable>::table().select((dsl::name, dsl::product, dsl::is_enabled));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Filter,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .attach_printable("Failed to list whether each alert definition is enabled")
        .switch()
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
        >(conn, dsl::id.eq(id), AlertsInfoUpdateInternal::from(update))
        .await
    }
}
