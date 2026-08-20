use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{associations::HasTable, ExpressionMethods, QueryDsl, TextExpressionMethods};
use error_stack::ResultExt;

use crate::{
    errors,
    revenue_recovery_retry_stats::{RevenueRecoveryRetryStats, RevenueRecoveryRetryStatsNew},
    schema_v2::revenue_recovery_retry_stats::dsl,
    PgPooledConn, StorageResult,
};

impl RevenueRecoveryRetryStatsNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<RevenueRecoveryRetryStats> {
        diesel::insert_into(<RevenueRecoveryRetryStats as HasTable>::table())
            .values(self)
            .get_result_async::<RevenueRecoveryRetryStats>(conn)
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("error inserting revenue_recovery_retry_stats row")
    }
}

impl RevenueRecoveryRetryStats {
    pub async fn update_distribution(
        conn: &PgPooledConn,
        cluster_key: String,
        distribution: serde_json::Value,
    ) -> StorageResult<Self> {
        diesel::update(<Self as HasTable>::table().filter(dsl::cluster_key.eq(cluster_key)))
            .set(dsl::distribution.eq(distribution))
            .get_result_async::<Self>(conn)
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("error updating revenue_recovery_retry_stats row")
    }

    pub async fn find_by_key(conn: &PgPooledConn, key: String) -> StorageResult<Option<Self>> {
        <Self as HasTable>::table()
            .filter(dsl::cluster_key.eq(key))
            .get_results_async::<Self>(conn)
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("error fetching revenue_recovery_retry_stats row by key")
            .map(|mut rows| rows.pop())
    }

    pub async fn find_by_key_prefix(
        conn: &PgPooledConn,
        key_prefix: &str,
    ) -> StorageResult<Vec<Self>> {
        <Self as HasTable>::table()
            .filter(dsl::cluster_key.like(format!("{key_prefix}%")))
            .get_results_async::<Self>(conn)
            .await
            .change_context(errors::DatabaseError::Others)
            .attach_printable("error scanning revenue_recovery_retry_stats rows by key prefix")
    }
}
