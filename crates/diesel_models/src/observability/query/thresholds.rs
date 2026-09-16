use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use diesel::{
    dsl::{count_star, sql},
    prelude::*,
    sql_types::Timestamp,
};
use error_stack::report;

use crate::{
    errors::DatabaseError,
    observability::{
        schema::success_rate_threshold_overrides::{self, dsl},
        thresholds::{ThresholdOverride, ThresholdOverrideNew, ThresholdUpsertOutcome},
    },
    DatabaseConnectionWithContext, StorageResult,
};

// All threshold mutations take this transaction-scoped lock. It makes the active-row count and
// replacement one serial operation even when the natural keys differ.
const THRESHOLD_MUTATION_LOCK: i64 = 7_428_915_301;

impl ThresholdOverride {
    pub async fn list_active(conn: &DatabaseConnectionWithContext<'_>) -> StorageResult<Vec<Self>> {
        dsl::success_rate_threshold_overrides
            .filter(dsl::is_deleted.eq(false))
            .order((dsl::merchant_id, dsl::profile_id, dsl::name, dsl::product))
            .select(Self::as_select())
            .load_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}

impl ThresholdOverrideNew {
    pub async fn upsert_with_limit(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        max_active_rules: i64,
    ) -> StorageResult<ThresholdUpsertOutcome> {
        let connection = conn.raw_connection();
        connection
            .transaction_async::<ThresholdUpsertOutcome, diesel::result::Error, _, _>(move |_| {
                Box::pin(async move {
                    diesel::sql_query(format!(
                        "SELECT pg_advisory_xact_lock({THRESHOLD_MUTATION_LOCK})"
                    ))
                    .execute_async(connection)
                    .await?;

                    let already_active = dsl::success_rate_threshold_overrides
                        .filter(dsl::name.eq(self.name.clone()))
                        .filter(dsl::product.eq(self.product.clone()))
                        .filter(dsl::merchant_id.eq(self.merchant_id.clone()))
                        .filter(dsl::profile_id.eq(self.profile_id.clone()))
                        .select(dsl::is_deleted)
                        .first_async::<bool>(connection)
                        .await
                        .optional()?
                        .is_some_and(|deleted| !deleted);

                    if max_active_rules > 0 && !already_active {
                        let active_count = dsl::success_rate_threshold_overrides
                            .filter(dsl::is_deleted.eq(false))
                            .select(count_star())
                            .first_async::<i64>(connection)
                            .await?;
                        if active_count >= max_active_rules {
                            return Ok(ThresholdUpsertOutcome::ActiveRuleLimitReached);
                        }
                    }

                    let row = diesel::insert_into(success_rate_threshold_overrides::table)
                        .values(self)
                        .on_conflict((dsl::name, dsl::product, dsl::merchant_id, dsl::profile_id))
                        .do_update()
                        .set((
                            dsl::min_volume.eq(diesel::upsert::excluded(dsl::min_volume)),
                            dsl::min_impacted_volume
                                .eq(diesel::upsert::excluded(dsl::min_impacted_volume)),
                            dsl::tolerance.eq(diesel::upsert::excluded(dsl::tolerance)),
                            dsl::diff_threshold.eq(diesel::upsert::excluded(dsl::diff_threshold)),
                            dsl::updated_by.eq(diesel::upsert::excluded(dsl::updated_by)),
                            dsl::last_updated_at
                                .eq(sql::<Timestamp>("date_trunc('second', CURRENT_TIMESTAMP)")),
                            dsl::is_deleted.eq(false),
                        ))
                        .returning(ThresholdOverride::as_returning())
                        .get_result_async(connection)
                        .await?;

                    Ok(ThresholdUpsertOutcome::Stored(row))
                })
            })
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    pub async fn tombstone(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<ThresholdOverride> {
        let connection = conn.raw_connection();
        connection
            .transaction_async(move |_| {
                Box::pin(async move {
                    diesel::sql_query(format!(
                        "SELECT pg_advisory_xact_lock({THRESHOLD_MUTATION_LOCK})"
                    ))
                    .execute_async(connection)
                    .await?;

                    diesel::insert_into(success_rate_threshold_overrides::table)
                        .values(self)
                        .on_conflict((dsl::name, dsl::product, dsl::merchant_id, dsl::profile_id))
                        .do_update()
                        .set((
                            dsl::min_volume.eq(Option::<f64>::None),
                            dsl::min_impacted_volume.eq(Option::<f64>::None),
                            dsl::tolerance.eq(Option::<f64>::None),
                            dsl::diff_threshold.eq(Option::<f64>::None),
                            dsl::updated_by.eq(diesel::upsert::excluded(dsl::updated_by)),
                            dsl::last_updated_at
                                .eq(sql::<Timestamp>("date_trunc('second', CURRENT_TIMESTAMP)")),
                            dsl::is_deleted.eq(true),
                        ))
                        .returning(ThresholdOverride::as_returning())
                        .get_result_async(connection)
                        .await
                })
            })
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
