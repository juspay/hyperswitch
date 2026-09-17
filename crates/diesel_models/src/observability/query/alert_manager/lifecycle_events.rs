use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use diesel::prelude::*;
use error_stack::report;
use time::PrimitiveDateTime;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::lifecycle_events::{LifecycleEvent, LifecycleEventNew},
        schema::alert_lifecycle_events::{self, dsl},
    },
    DatabaseConnectionWithContext, StorageResult,
};

impl LifecycleEvent {
    pub async fn list_overlapping(
        conn: &DatabaseConnectionWithContext<'_>,
        from: Option<PrimitiveDateTime>,
        to: Option<PrimitiveDateTime>,
    ) -> StorageResult<Vec<Self>> {
        let mut query = crate::list::into_boxed_list(dsl::alert_lifecycle_events);
        if let Some(from) = from {
            query = query.filter(dsl::last_seen.ge(from));
        }
        if let Some(to) = to {
            query = query.filter(dsl::first_seen.le(to));
        }

        query
            .order(dsl::alert_key)
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    pub async fn replace_batch_and_cleanup(
        rows: Vec<LifecycleEventNew>,
        retention_cutoff: PrimitiveDateTime,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<usize> {
        let persisted = rows.len();
        conn.raw_connection()
            .transaction_async(move |connection| async move {
                for row in rows {
                    diesel::query_dsl::methods::FilterDsl::filter(
                        diesel::insert_into(alert_lifecycle_events::table)
                            .values(row)
                            .on_conflict(dsl::alert_key)
                            .do_update()
                            .set((
                                dsl::detector.eq(diesel::upsert::excluded(dsl::detector)),
                                dsl::merchant_id.eq(diesel::upsert::excluded(dsl::merchant_id)),
                                dsl::profile_id.eq(diesel::upsert::excluded(dsl::profile_id)),
                                dsl::state.eq(diesel::upsert::excluded(dsl::state)),
                                dsl::first_seen.eq(diesel::upsert::excluded(dsl::first_seen)),
                                dsl::last_seen.eq(diesel::upsert::excluded(dsl::last_seen)),
                                dsl::recovered_at.eq(diesel::upsert::excluded(dsl::recovered_at)),
                                dsl::runs.eq(diesel::upsert::excluded(dsl::runs)),
                                dsl::severity.eq(diesel::upsert::excluded(dsl::severity)),
                                dsl::sr.eq(diesel::upsert::excluded(dsl::sr)),
                                dsl::failed.eq(diesel::upsert::excluded(dsl::failed)),
                                dsl::total.eq(diesel::upsert::excluded(dsl::total)),
                                dsl::connector.eq(diesel::upsert::excluded(dsl::connector)),
                                dsl::notified_at.eq(diesel::upsert::excluded(dsl::notified_at)),
                                dsl::ts_slack.eq(diesel::upsert::excluded(dsl::ts_slack)),
                                dsl::sent.eq(diesel::upsert::excluded(dsl::sent)),
                                dsl::last_updated_at
                                    .eq(diesel::upsert::excluded(dsl::last_updated_at)),
                            )),
                        diesel::upsert::excluded(dsl::last_updated_at).gt(dsl::last_updated_at),
                    )
                    .execute_async(&connection)
                    .await?;
                }

                diesel::delete(
                    dsl::alert_lifecycle_events.filter(dsl::last_updated_at.lt(retention_cutoff)),
                )
                .execute_async(&connection)
                .await?;

                Ok::<usize, diesel::result::Error>(persisted)
            })
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
