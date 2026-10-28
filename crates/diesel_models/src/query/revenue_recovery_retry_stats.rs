use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl};
use error_stack::ResultExt;
use hyperswitch_masking::Secret;

use super::generics::{self, db_metrics::DatabaseOperation};
use crate::{
    errors,
    revenue_recovery_retry_stats::{RevenueRecoveryRetryStats, RevenueRecoveryRetryStatsNew},
    schema_v2::revenue_recovery_retry_stats::dsl,
    DatabaseConnectionWithContext, StorageResult,
};

impl RevenueRecoveryRetryStatsNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<RevenueRecoveryRetryStats> {
        conn.track_get_result::<<RevenueRecoveryRetryStats as HasTable>::Table, _, RevenueRecoveryRetryStats>(
            DatabaseOperation::Insert,
            diesel::insert_into(<RevenueRecoveryRetryStats as HasTable>::table()).values(self),
        )
        .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("error inserting revenue_recovery_retry_stats row")
    }
}

impl RevenueRecoveryRetryStats {
    pub async fn update_stats(
        conn: &DatabaseConnectionWithContext<'_>,
        cluster_key: String,
        stats: Secret<serde_json::Value>,
    ) -> StorageResult<Self> {
        conn.track_get_result::<<Self as HasTable>::Table, _, Self>(
            DatabaseOperation::Update,
            diesel::update(<Self as HasTable>::table().filter(dsl::cluster_key.eq(cluster_key)))
                .set(dsl::stats.eq(stats)),
        )
        .await
        .change_context(errors::DatabaseError::Others)
        .attach_printable("error updating revenue_recovery_retry_stats row")
    }

    pub async fn find_by_key(
        conn: &DatabaseConnectionWithContext<'_>,
        key: String,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::cluster_key.eq(key),
        )
        .await
    }
}
