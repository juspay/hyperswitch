use async_bb8_diesel::AsyncRunQueryDsl;
use common_utils::errors::ReportSwitchExt;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::ResultExt;

use crate::{
    observability::{
        merchants_alert_external_config::{
            MerchantsAlertExternalConfig, MerchantsAlertExternalConfigNew,
            MerchantsAlertExternalConfigUpdate, MerchantsAlertExternalConfigUpdateInternal,
        },
        schema::merchants_alert_external_config::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl MerchantsAlertExternalConfigNew {
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        update: MerchantsAlertExternalConfigUpdate,
    ) -> StorageResult<MerchantsAlertExternalConfig> {
        let query = diesel::insert_into(<MerchantsAlertExternalConfig as HasTable>::table())
            .values(self)
            .on_conflict((dsl::name, dsl::product))
            .do_update()
            .set(MerchantsAlertExternalConfigUpdateInternal::from(update));

        generics::db_metrics::track_database_call::<
            <MerchantsAlertExternalConfig as HasTable>::Table,
            _,
            _,
        >(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Insert,
            query.get_result_async(conn.raw_connection()),
        )
        .await
        .attach_printable("Failed to upsert the alert enablement row")
        .switch()
    }
}

impl MerchantsAlertExternalConfig {
    pub async fn find_by_name_and_product(
        conn: &DatabaseConnectionWithContext<'_>,
        name: &str,
        product: &str,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::name
                .eq(name.to_owned())
                .and(dsl::product.eq(product.to_owned())),
        )
        .await
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
}
