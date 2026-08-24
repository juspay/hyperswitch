use super::{retry_stats_cluster_key::RetryStatsClusterKey, retry_stats_document::StatsDocument};

/// Domain representation of a persisted `revenue_recovery_retry_stats` row. Both
/// columns are held in their strongly-typed form — [`RetryStatsClusterKey`] instead
/// of the serialized `cluster_key` string, and [`StatsDocument`] instead of the raw
/// `stats` JSON — so callers query and mutate the DB in terms of domain types and
/// never hand-roll `as_db_string` / `from_json` at the call site.
///
/// The `Conversion` impl that performs those encodings lives in the `storage_impl`
/// crate (alongside the other domain<->diesel conversions), keeping this crate free
/// of a storage dependency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevenueRecoveryRetryStats {
    pub cluster_key: RetryStatsClusterKey,
    pub stats: StatsDocument,
}
