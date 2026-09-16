use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, debug_query, expression_methods::PgSortExpressionMethods, pg::Pg,
    BoolExpressionMethods, ExpressionMethods, QueryDsl,
};
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    errors::DatabaseError,
    observability::{
        alerts_info::{AlertsInfo, AlertsInfoNew},
        schema::alerts_info::dsl,
    },
    query::{
        generics,
        generics::db_metrics::{track_database_call, DatabaseOperation},
    },
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
    /// The most recently updated row for `(name, product)`, whatever `is_enabled` is set to.
    ///
    /// Used to check a `merchants_alert_external_config` create or update against a defined
    /// alert, replacing r-apps' `BEFORE INSERT OR UPDATE` trigger.
    pub async fn find_latest_by_name_product(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        product: &str,
    ) -> StorageResult<Option<Self>> {
        let query = crate::list::into_boxed_list(
            Self::table()
                .filter(
                    dsl::name
                        .eq(name.to_owned())
                        .and(dsl::product.eq(product.to_owned())),
                )
                .order(dsl::last_updated_at.desc().nulls_last())
                .limit(1),
        );

        logger::debug!(query = %debug_query::<Pg, _>(&query).to_string());

        let rows: Vec<Self> = track_database_call::<Self, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            DatabaseOperation::Filter,
            query.get_results_async(conn.raw_connection()),
        )
        .await
        .change_context(DatabaseError::Others)
        .attach_printable("Failed to find alerts_info by name and product")?;

        Ok(rows.into_iter().next())
    }
}
