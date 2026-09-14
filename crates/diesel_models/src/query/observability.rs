pub mod alerts_dicts;
pub mod alerts_info;
pub mod alerts_intermediate;
pub mod alerts_main;
pub mod merchants_alert_external;
pub mod merchants_alert_external_config;
pub mod merchants_alert_external_dimension;
pub mod notification_reads;

use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    sql_types::{Integer, Text},
    Table,
};
use error_stack::ResultExt;

use crate::{errors, query::generics, DatabaseConnectionWithContext, StorageResult};

const LIFECYCLE_STATE_LOCK_NAMESPACE: i32 = 23_404;

const INSTANCES_LOCK_NAMESPACE: i32 = 23_405;

const DIMENSIONS_LOCK_NAMESPACE: i32 = 23_406;

async fn advisory_xact_lock<T: Table>(
    conn: &DatabaseConnectionWithContext<'_>,
    namespace: i32,
    key: &str,
) -> StorageResult<()> {
    let query = diesel::sql_query("SELECT pg_advisory_xact_lock($1, hashtext($2))")
        .bind::<Integer, _>(namespace)
        .bind::<Text, _>(key.to_owned());

    generics::db_metrics::track_database_call::<T, _, _>(
        conn.request_id(),
        conn.event_emitter(),
        generics::db_metrics::DatabaseOperation::FindOne,
        query.execute_async(conn.raw_connection()),
    )
    .await
    .map(|_| ())
    .map_err(|e| error_stack::report!(e))
    .change_context(errors::DatabaseError::Others)
    .attach_printable("Failed to take a transaction-level advisory lock")
}
