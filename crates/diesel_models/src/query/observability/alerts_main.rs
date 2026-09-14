use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable,
    query_dsl::methods::{FilterDsl, SelectDsl},
    BoolExpressionMethods, ExpressionMethods,
};
use error_stack::ResultExt;

use crate::{
    errors,
    observability::{
        alerts_main::{AlertsMain, AlertsMainNew},
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
