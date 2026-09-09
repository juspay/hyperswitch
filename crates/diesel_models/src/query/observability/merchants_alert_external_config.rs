use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};
use error_stack::ResultExt;

use crate::{
    errors,
    observability::{
        merchants_alert_external_config::{
            MerchantsAlertExternalConfig, MerchantsAlertExternalConfigNew,
        },
        schema::merchants_alert_external_config::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl MerchantsAlertExternalConfigNew {
    /// Write the row for this `(name, product)`, replacing whatever was there.
    ///
    /// One statement, with the table's primary key as the conflict target. A read followed by an
    /// insert-or-update would race a concurrent writer, and naming a single column as the target
    /// would not compile against a composite key — which is exactly the mistake the key exists to
    /// make impossible.
    ///
    /// `DO UPDATE` rather than `DO NOTHING`: this route is how an alert is turned off, so a second
    /// call that silently kept the first call's value would be a switch that only works once.
    pub async fn upsert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<MerchantsAlertExternalConfig> {
        let query = diesel::insert_into(<MerchantsAlertExternalConfig as HasTable>::table())
            .values(self.clone())
            .on_conflict((dsl::name, dsl::product))
            .do_update()
            .set(self);

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
        .map_err(|error| error_stack::report!(error))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to upsert the alert enablement row")
    }
}

impl MerchantsAlertExternalConfig {
    pub async fn find_by_name_and_product(
        conn: &DatabaseConnectionWithContext<'_>,
        name: String,
        product: String,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::name.eq(name).and(dsl::product.eq(product)),
        )
        .await
    }

    /// Every enablement row. Unpaginated for the same reason the definition list is: one row per
    /// alert per product.
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
}
