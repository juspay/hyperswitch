use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{dsl::count_star, prelude::*};
use error_stack::report;

use crate::{
    errors::DatabaseError,
    observability::{
        alert_manager::thresholds::{
            ThresholdOverride, ThresholdOverrideNew, ThresholdUpsertOutcome,
        },
        schema::success_rate_threshold_overrides::{self, dsl},
    },
    DatabaseConnectionWithContext, StorageResult,
};

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
        let already_active = dsl::success_rate_threshold_overrides
            .filter(dsl::name.eq(self.name.clone()))
            .filter(dsl::product.eq(self.product.clone()))
            .filter(dsl::merchant_id.eq(self.merchant_id.clone()))
            .filter(dsl::profile_id.eq(self.profile_id.clone()))
            .select(dsl::is_deleted)
            .first_async::<bool>(connection)
            .await
            .optional()
            .map_err(|error| report!(error).change_context(DatabaseError::Others))?
            .is_some_and(|deleted| !deleted);

        if max_active_rules > 0 && !already_active {
            let active_count = dsl::success_rate_threshold_overrides
                .filter(dsl::is_deleted.eq(false))
                .select(count_star())
                .first_async::<i64>(connection)
                .await
                .map_err(|error| report!(error).change_context(DatabaseError::Others))?;
            if active_count >= max_active_rules {
                return Ok(ThresholdUpsertOutcome::ActiveRuleLimitReached);
            }
        }

        diesel::insert_into(success_rate_threshold_overrides::table)
            .values(self)
            .on_conflict((dsl::name, dsl::product, dsl::merchant_id, dsl::profile_id))
            .do_update()
            .set((
                dsl::min_volume.eq(diesel::upsert::excluded(dsl::min_volume)),
                dsl::min_impacted_volume.eq(diesel::upsert::excluded(dsl::min_impacted_volume)),
                dsl::tolerance.eq(diesel::upsert::excluded(dsl::tolerance)),
                dsl::diff_threshold.eq(diesel::upsert::excluded(dsl::diff_threshold)),
                dsl::updated_by.eq(diesel::upsert::excluded(dsl::updated_by)),
                dsl::last_updated_at.eq(diesel::upsert::excluded(dsl::last_updated_at)),
                dsl::is_deleted.eq(false),
            ))
            .returning(ThresholdOverride::as_returning())
            .get_result_async(connection)
            .await
            .map(ThresholdUpsertOutcome::Stored)
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }

    pub async fn tombstone(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<ThresholdOverride> {
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
                dsl::last_updated_at.eq(diesel::upsert::excluded(dsl::last_updated_at)),
                dsl::is_deleted.eq(true),
            ))
            .returning(ThresholdOverride::as_returning())
            .get_result_async(conn.raw_connection())
            .await
            .map_err(|error| report!(error).change_context(DatabaseError::Others))
    }
}
