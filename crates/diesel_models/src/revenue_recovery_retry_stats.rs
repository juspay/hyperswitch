use diesel::{Identifiable, Queryable, Selectable};
use hyperswitch_masking::Secret;

use crate::schema_v2::revenue_recovery_retry_stats;

#[derive(
    Clone, Debug, Queryable, Identifiable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = revenue_recovery_retry_stats, check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(cluster_key))]
pub struct RevenueRecoveryRetryStats {
    pub cluster_key: String,
    pub stats: Secret<serde_json::Value>,
}

#[derive(Clone, Debug, diesel::Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = revenue_recovery_retry_stats)]
pub struct RevenueRecoveryRetryStatsNew {
    pub cluster_key: String,
    pub stats: Secret<serde_json::Value>,
}
