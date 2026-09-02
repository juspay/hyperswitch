use common_enums::StandardisedCode;
use common_utils::{
    errors::{CustomResult, ValidationError},
    types::keymanager,
};
pub use diesel_models::revenue_recovery_retry_stats::{
    RevenueRecoveryRetryStats, RevenueRecoveryRetryStatsNew,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::revenue_recovery::{
    retry_stats::RevenueRecoveryRetryStats as DomainRevenueRecoveryRetryStats,
    retry_stats_cluster_key::RetryStatsClusterKey, retry_stats_document::StatsDocument,
};
use hyperswitch_masking::{PeekInterface, Secret};
use router_env::{instrument, tracing};

use crate::{
    behaviour::Conversion, connection, errors::StorageError, kv_router_store::KVRouterStore,
    DatabaseStore, MockDb, RouterStore,
};

/// Parse a diesel row into the strongly-typed domain model.
fn domain_from_diesel_row(
    item: RevenueRecoveryRetryStats,
) -> CustomResult<DomainRevenueRecoveryRetryStats, ValidationError> {
    // The cluster key is layout-versioned and self-validating: a row written under an
    // unsupported layout (or a corrupt key) is rejected here rather than silently
    // resolving to the wrong cluster.
    let cluster_key = RetryStatsClusterKey::from_db_string(&item.cluster_key).ok_or_else(|| {
        report!(ValidationError::InvalidValue {
            message: format!(
                "revenue_recovery_retry_stats: unparseable cluster_key '{}'",
                item.cluster_key
            ),
        })
    })?;
    let stats = StatsDocument::from_json(item.stats.peek()).map_err(|error| {
        report!(ValidationError::InvalidValue {
            message: format!("revenue_recovery_retry_stats: unparseable stats document: {error}"),
        })
    })?;
    Ok(DomainRevenueRecoveryRetryStats { cluster_key, stats })
}

#[async_trait::async_trait]
impl Conversion for DomainRevenueRecoveryRetryStats {
    type DstType = RevenueRecoveryRetryStats;
    type NewDstType = RevenueRecoveryRetryStatsNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(RevenueRecoveryRetryStats {
            cluster_key: self.cluster_key.as_db_string(),
            stats: Secret::new(self.stats.to_json()),
        })
    }

    async fn convert_back(
        _state: &keymanager::KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError> {
        domain_from_diesel_row(item)
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(RevenueRecoveryRetryStatsNew {
            cluster_key: self.cluster_key.as_db_string(),
            stats: Secret::new(self.stats.to_json()),
        })
    }
}

/// Storage access for the revenue-recovery `revenue_recovery_retry_stats` doc-store.
#[async_trait::async_trait]
pub trait RevenueRecoveryRetryStatsInterface: Send + Sync {
    type Error;

    /// Fetch the stored stats document for a single cluster key (`None` when absent).
    /// Takes a [`RetryStatsClusterKey`] (serializing it internally) and returns the
    /// parsed domain model.
    async fn find_revenue_recovery_retry_stats_by_cluster_key(
        &self,
        cluster_key: &RetryStatsClusterKey,
    ) -> CustomResult<Option<DomainRevenueRecoveryRetryStats>, Self::Error>;

    /// Fetch the stats recorded against an error code alone. While only the error_code
    /// dimension is populated, leaves are stored under `error_code/UNK/UNK`
    /// ([`RetryStatsClusterKey::from_error_code`]); this queries that exact key.
    async fn find_revenue_recovery_retry_stats_by_error_code(
        &self,
        error_code: StandardisedCode,
    ) -> CustomResult<Option<DomainRevenueRecoveryRetryStats>, Self::Error> {
        self.find_revenue_recovery_retry_stats_by_cluster_key(
            &RetryStatsClusterKey::from_error_code(error_code),
        )
        .await
    }

    /// Insert a new stats document for a key that does not yet exist. Takes the domain
    /// model and serializes both columns internally.
    async fn insert_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: DomainRevenueRecoveryRetryStats,
    ) -> CustomResult<DomainRevenueRecoveryRetryStats, Self::Error>;

    /// Overwrite the stats document for a key that already exists. Takes the domain
    /// model and serializes both columns internally.
    async fn update_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: DomainRevenueRecoveryRetryStats,
    ) -> CustomResult<DomainRevenueRecoveryRetryStats, Self::Error>;
}

#[async_trait::async_trait]
impl<T: DatabaseStore> RevenueRecoveryRetryStatsInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn find_revenue_recovery_retry_stats_by_cluster_key(
        &self,
        cluster_key: &RetryStatsClusterKey,
    ) -> CustomResult<Option<DomainRevenueRecoveryRetryStats>, StorageError> {
        // Read from the write primary: callers do read-modify-write on the same row,
        // so a lagged replica can miss a just-committed INSERT and trigger a duplicate-key violation.
        let conn = connection::pg_connection_write(self).await?;
        RevenueRecoveryRetryStats::find_by_key(&conn, cluster_key.as_db_string())
            .await
            .map_err(|error| report!(StorageError::from(error)))?
            .map(domain_from_diesel_row)
            .transpose()
            .change_context(StorageError::DeserializationFailed)
    }

    #[instrument(skip_all)]
    async fn insert_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: DomainRevenueRecoveryRetryStats,
    ) -> CustomResult<DomainRevenueRecoveryRetryStats, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let inserted = RevenueRecoveryRetryStatsNew {
            cluster_key: revenue_recovery_retry_stats.cluster_key.as_db_string(),
            stats: Secret::new(revenue_recovery_retry_stats.stats.to_json()),
        }
        .insert(&conn)
        .await
        .map_err(|error| report!(StorageError::from(error)))?;
        domain_from_diesel_row(inserted).change_context(StorageError::DeserializationFailed)
    }

    #[instrument(skip_all)]
    async fn update_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: DomainRevenueRecoveryRetryStats,
    ) -> CustomResult<DomainRevenueRecoveryRetryStats, StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let updated = RevenueRecoveryRetryStats::update_stats(
            &conn,
            revenue_recovery_retry_stats.cluster_key.as_db_string(),
            Secret::new(revenue_recovery_retry_stats.stats.to_json()),
        )
        .await
        .map_err(|error| report!(StorageError::from(error)))?;
        domain_from_diesel_row(updated).change_context(StorageError::DeserializationFailed)
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> RevenueRecoveryRetryStatsInterface for KVRouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn find_revenue_recovery_retry_stats_by_cluster_key(
        &self,
        cluster_key: &RetryStatsClusterKey,
    ) -> CustomResult<Option<DomainRevenueRecoveryRetryStats>, StorageError> {
        self.router_store
            .find_revenue_recovery_retry_stats_by_cluster_key(cluster_key)
            .await
    }

    #[instrument(skip_all)]
    async fn insert_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: DomainRevenueRecoveryRetryStats,
    ) -> CustomResult<DomainRevenueRecoveryRetryStats, StorageError> {
        self.router_store
            .insert_revenue_recovery_retry_stats(revenue_recovery_retry_stats)
            .await
    }

    #[instrument(skip_all)]
    async fn update_revenue_recovery_retry_stats(
        &self,
        revenue_recovery_retry_stats: DomainRevenueRecoveryRetryStats,
    ) -> CustomResult<DomainRevenueRecoveryRetryStats, StorageError> {
        self.router_store
            .update_revenue_recovery_retry_stats(revenue_recovery_retry_stats)
            .await
    }
}

#[async_trait::async_trait]
impl RevenueRecoveryRetryStatsInterface for MockDb {
    type Error = StorageError;

    async fn find_revenue_recovery_retry_stats_by_cluster_key(
        &self,
        _cluster_key: &RetryStatsClusterKey,
    ) -> CustomResult<Option<DomainRevenueRecoveryRetryStats>, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn insert_revenue_recovery_retry_stats(
        &self,
        _revenue_recovery_retry_stats: DomainRevenueRecoveryRetryStats,
    ) -> CustomResult<DomainRevenueRecoveryRetryStats, StorageError> {
        Err(StorageError::MockDbError)?
    }

    async fn update_revenue_recovery_retry_stats(
        &self,
        _revenue_recovery_retry_stats: DomainRevenueRecoveryRetryStats,
    ) -> CustomResult<DomainRevenueRecoveryRetryStats, StorageError> {
        Err(StorageError::MockDbError)?
    }
}
