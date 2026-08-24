use super::{retry_stats_cluster_key::RetryStatsClusterKey, retry_stats_document::StatsDocument};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevenueRecoveryRetryStats {
    pub cluster_key: RetryStatsClusterKey,
    pub stats: StatsDocument,
}
