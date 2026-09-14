use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    query_dsl::methods::{FilterDsl, SelectDsl},
    BoolExpressionMethods, ExpressionMethods,
};
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use crate::{
    errors,
    observability::{
        alerts_main::{AlertsMain, AlertsMainNew, AlertsMainUpdate, AlertsMainUpdateInternal},
        schema::alerts_main::dsl,
    },
    query::generics,
    DatabaseConnectionWithContext, StorageResult,
};

impl AlertsMainNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<AlertsMain> {
        generics::generic_insert(conn, self).await
    }
}

impl AlertsMain {
    pub async fn list_by_channel_and_ts_alert_window(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
        start: PrimitiveDateTime,
        end: PrimitiveDateTime,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::channel
                .eq(channel.to_owned())
                .and(dsl::ts_alert.ge(start))
                .and(dsl::ts_alert.le(end)),
            None,
            None,
            Some((dsl::ts_alert.desc(), dsl::id.desc())),
        )
        .await
    }

    pub async fn update_by_channel_and_id(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
        id: uuid::Uuid,
        update: AlertsMainUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::channel.eq(channel.to_owned()).and(dsl::id.eq(id)),
            AlertsMainUpdateInternal::from(update),
        )
        .await
    }

    pub async fn list_ids_by_channel_and_ids(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: &str,
        ids: Vec<uuid::Uuid>,
    ) -> StorageResult<Vec<uuid::Uuid>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let query = <Self as HasTable>::table()
            .select(dsl::id)
            .filter(dsl::channel.eq(channel.to_owned()).and(dsl::id.eq_any(ids)));

        generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
            conn.request_id(),
            conn.event_emitter(),
            generics::db_metrics::DatabaseOperation::Filter,
            query.load_async::<uuid::Uuid>(conn.raw_connection()),
        )
        .await
        .map_err(|e| error_stack::report!(e))
        .change_context(errors::DatabaseError::Others)
        .attach_printable("Failed to list announcement ids by channel")
    }
}
