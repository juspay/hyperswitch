use common_utils::errors::CustomResult;
pub use diesel_models::revenue_recovery_retry_stats::{
    RevenueRecoveryRetryStats, RevenueRecoveryRetryStatsNew,
};
use error_stack::report;
use router_env::{instrument, tracing};

use crate::{
    connection, errors::StorageError, kv_router_store::KVRouterStore, DatabaseStore, MockDb,
    RouterStore,
};

/// Storage access for the revenue-recovery `revenue_recovery_retry_stats` doc-store.
#[async_trait::async_trait]
pub trait RevenueRecoveryRetryStatsInterface: Send + Sync {
    type Error;

    /// Fetch the stored stats document for a single cluster key (`None` when absent).
    async fn find_revenue_recovery_retry_stats_by_key(
        &self,
        key: String,
    ) -> CustomResult<Option<RevenueRecoveryRetryStats>, Self::Error>;

    /// Insert a new stats document for a key that does not yet exist.
    async fn insert_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: RevenueRecoveryRetryStatsNew,
    ) -> CustomResult<RevenueRecoveryRetryStats, Self::Error>;

    /// Overwrite the stats document for a key that already exists.
    async fn update_revenue_recovery_retry_stats(
        &self,
        cluster_key: String,
        distribution: serde_json::Value,
    ) -> CustomResult<RevenueRecoveryRetryStats, Self::Error>;
}

#[async_trait::async_trait]
impl<T: DatabaseStore> RevenueRecoveryRetryStatsInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn find_revenue_recovery_retry_stats_by_key(
        &self,
        key: String,
    ) -> CustomResult<Option<RevenueRecoveryRetryStats>, StorageError> {
        // Read from the write primary: callers do read-modify-write on the same row,
        // so a lagged replica can miss a just-committed INSERT and trigger a duplicate-key violation.
        let conn = connection::pg_connection_write(self).await?;
        RevenueRecoveryRetryStats::find_by_key(&conn, key)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: RevenueRecoveryRetryStatsNew,
    ) -> CustomResult<RevenueRecoveryRetryStats, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        revenue_recovery_retry_stats
            .insert(&conn)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_revenue_recovery_retry_stats(
        &self,
        cluster_key: String,
        distribution: serde_json::Value,
    ) -> CustomResult<RevenueRecoveryRetryStats, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        RevenueRecoveryRetryStats::update_distribution(&conn, cluster_key, distribution)
            .await
            .map_err(|error| report!(StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> RevenueRecoveryRetryStatsInterface for KVRouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn find_revenue_recovery_retry_stats_by_key(
        &self,
        key: String,
    ) -> CustomResult<Option<RevenueRecoveryRetryStats>, StorageError> {
        self.router_store
            .find_revenue_recovery_retry_stats_by_key(key)
            .await
    }

    #[instrument(skip_all)]
    async fn insert_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: RevenueRecoveryRetryStatsNew,
    ) -> CustomResult<RevenueRecoveryRetryStats, StorageError> {
        self.router_store
            .insert_revenue_recovery_retry_stats(revenue_recovery_retry_stats)
            .await
    }

    #[instrument(skip_all)]
    async fn update_revenue_recovery_retry_stats(
        &self,
        cluster_key: String,
        distribution: serde_json::Value,
    ) -> CustomResult<RevenueRecoveryRetryStats, StorageError> {
        self.router_store
            .update_revenue_recovery_retry_stats(cluster_key, distribution)
            .await
    }
}

#[async_trait::async_trait]
impl RevenueRecoveryRetryStatsInterface for MockDb {
    type Error = StorageError;

    async fn find_revenue_recovery_retry_stats_by_key(
        &self,
        _key: String,
    ) -> CustomResult<Option<RevenueRecoveryRetryStats>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn insert_revenue_recovery_retry_stats(
        &self,
        _revenue_recovery_retry_stats: RevenueRecoveryRetryStatsNew,
    ) -> CustomResult<RevenueRecoveryRetryStats, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_revenue_recovery_retry_stats(
        &self,
        _key: String,
        _distribution: serde_json::Value,
    ) -> CustomResult<RevenueRecoveryRetryStats, StorageError> {
        Err(StorageError::MockDbError)?
    }
}
