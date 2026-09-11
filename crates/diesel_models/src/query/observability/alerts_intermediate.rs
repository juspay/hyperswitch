use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{
    associations::HasTable, sql_types::Integer, upsert::excluded, ExpressionMethods, QueryDsl,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::{errors, query::generics, DatabaseConnectionWithContext, StorageResult};

const LIFECYCLE_LOCK_NAMESPACE: i32 = 23_404;

pub async fn lock_lifecycle_state(
    conn: &DatabaseConnectionWithContext<'_>,
    channel: i32,
) -> StorageResult<()> {
    diesel::sql_query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind::<Integer, _>(LIFECYCLE_LOCK_NAMESPACE)
        .bind::<Integer, _>(channel)
        .execute_async(conn.raw_connection())
        .await
        .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
        .attach_printable("Error while locking the lifecycle state")?;

    Ok(())
}

macro_rules! alert_state_queries {
    ($module:ident, $table:ident) => {
        pub mod $module {
            use super::*;
            use crate::observability::{
                alerts_intermediate::{$module::AlertState, AlertStateRow},
                schema::$table::dsl,
            };

            impl AlertState {
                pub async fn list(
                    conn: &DatabaseConnectionWithContext<'_>,
                ) -> StorageResult<Vec<AlertStateRow>> {
                    generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
                        conn,
                        dsl::id_intermediate.is_not_null(),
                        None,
                        None,
                        Some((dsl::ts_alert.asc(), dsl::id_intermediate.asc())),
                    )
                    .await
                    .map(|rows| rows.into_iter().map(AlertStateRow::from).collect())
                }

                pub async fn latest_last_updated_at(
                    conn: &DatabaseConnectionWithContext<'_>,
                ) -> StorageResult<Option<PrimitiveDateTime>> {
                    let query =
                        <Self as HasTable>::table().select(diesel::dsl::max(dsl::last_updated_at));

                    generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
                        conn.request_id(),
                        conn.event_emitter(),
                        generics::db_metrics::DatabaseOperation::Filter,
                        query.get_result_async::<Option<PrimitiveDateTime>>(conn.raw_connection()),
                    )
                    .await
                    .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
                    .attach_printable("Error while reading the lifecycle state watermark")
                }

                pub async fn delete_absent(
                    conn: &DatabaseConnectionWithContext<'_>,
                    keep: Vec<uuid::Uuid>,
                ) -> StorageResult<usize> {
                    let query = diesel::delete(
                        <Self as HasTable>::table().filter(dsl::id_intermediate.ne_all(keep)),
                    );

                    generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
                        conn.request_id(),
                        conn.event_emitter(),
                        generics::db_metrics::DatabaseOperation::Delete,
                        query.execute_async(conn.raw_connection()),
                    )
                    .await
                    .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
                    .attach_printable("Error while removing lifecycle state rows")
                }

                pub async fn upsert_all(
                    conn: &DatabaseConnectionWithContext<'_>,
                    rows: Vec<AlertStateRow>,
                ) -> StorageResult<usize> {
                    if rows.is_empty() {
                        return Ok(0);
                    }

                    let query = diesel::insert_into(<Self as HasTable>::table())
                        .values(rows.into_iter().map(Self::from).collect::<Vec<_>>())
                        .on_conflict(dsl::id_intermediate)
                        .do_update()
                        .set((
                            dsl::id.eq(excluded(dsl::id)),
                            dsl::name.eq(excluded(dsl::name)),
                            dsl::product.eq(excluded(dsl::product)),
                            dsl::dimensions.eq(excluded(dsl::dimensions)),
                            dsl::ts_slack.eq(excluded(dsl::ts_slack)),
                            dsl::ts_alert.eq(excluded(dsl::ts_alert)),
                            dsl::latest_ts_alert.eq(excluded(dsl::latest_ts_alert)),
                            dsl::max_duration.eq(excluded(dsl::max_duration)),
                            dsl::other_metrics.eq(excluded(dsl::other_metrics)),
                            dsl::metadata.eq(excluded(dsl::metadata)),
                            dsl::metadata_alert_details.eq(excluded(dsl::metadata_alert_details)),
                            dsl::rca_metadata.eq(excluded(dsl::rca_metadata)),
                            dsl::group_id.eq(excluded(dsl::group_id)),
                            dsl::priority.eq(excluded(dsl::priority)),
                            dsl::last_updated_at.eq(excluded(dsl::last_updated_at)),
                            dsl::recovered_ts.eq(excluded(dsl::recovered_ts)),
                        ));

                    generics::db_metrics::track_database_call::<<Self as HasTable>::Table, _, _>(
                        conn.request_id(),
                        conn.event_emitter(),
                        generics::db_metrics::DatabaseOperation::Insert,
                        query.execute_async(conn.raw_connection()),
                    )
                    .await
                    .map_err(|error| report!(error).change_context(errors::DatabaseError::Others))
                    .attach_printable("Error while saving lifecycle state rows")
                }
            }
        }
    };
}

alert_state_queries!(slack, alerts_intermediate);
alert_state_queries!(xyne, alerts_intermediate_xyne);
