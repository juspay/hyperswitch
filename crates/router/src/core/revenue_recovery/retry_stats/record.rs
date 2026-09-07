use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_domain_models::revenue_recovery::{
    retry_stats::RevenueRecoveryRetryStats as DomainRevenueRecoveryRetryStats,
    retry_stats_cluster_key::RetryStatsClusterKey, retry_stats_document::StatsDocument,
};
use redis_interface::SetnxReply;
use router_env::{instrument, logger, tracing};
use storage_impl::{
    errors::StorageError, revenue_recovery_retry_stats::RevenueRecoveryRetryStatsInterface,
};

use super::events::RetryOutcomeEvent;
use crate::{
    core::configs::dimension_state,
    routes::{app::SessionStateInfo, SessionState},
};

impl RetryOutcomeEvent {
    /// Record this outcome into `revenue_recovery_retry_stats` under a per-key
    /// Redis lock so concurrent writers don't lose updates in the read-merge-write.
    /// Recording is best-effort: any failure is logged and swallowed so it never
    /// affects the payment/recovery flow that invoked it.
    #[instrument(skip_all)]
    pub async fn record(&self, state: &SessionState) {
        if let Err(error) = self.persist(state).await {
            logger::error!(
                cluster_key = %self.key.as_db_string(),
                ?error,
                "revenue_recovery_retry_stats: failed to persist retry outcome"
            );
        }
    }

    /// Fallible core of [`Self::record`]: skips silently when recording is disabled,
    /// otherwise merges this outcome into the stats document under the per-key lock.
    /// The recorded / lock-contended log lines are emitted by `persist_node`.
    async fn persist(&self, state: &SessionState) -> CustomResult<(), StorageError> {
        let dimensions: dimension_state::DimensionsGlobal = dimension_state::Dimensions::new();
        let enabled = dimensions
            .get_revrec_retry_stats_enabled(
                state.store.as_ref(),
                state.superposition_service.as_ref(),
                None,
            )
            .await;

        if !enabled {
            logger::info!("revenue_recovery_retry_stats: recording disabled via config");
            Ok(())
        } else {
            let redis_conn = state
                .store
                .get_redis_conn()
                .change_context(StorageError::KVError)?;
            let store = state.store.get_revenue_recovery_retry_stats_store();

            let retry_stats_lock = state.conf().revenue_recovery.retry_stats_lock;
            persist_node(
                store.as_ref(),
                &redis_conn,
                self,
                retry_stats_lock.lock_retries(),
                retry_stats_lock.delay_between_retries_in_milliseconds,
                retry_stats_lock.redis_lock_expiry_seconds,
            )
            .await
        }
    }
}

/// Replace the entire stats document for a cluster key with the given one —
/// used by the admin retry-stats migration CSV upload
///
/// Returns `Ok(true)` when the document was written, `Ok(false)` when the per-key
/// lock stayed contended for the whole retry budget (nothing was written — the
/// caller must NOT report the row as loaded).
#[instrument(skip_all)]
pub async fn replace_retry_stats_document(
    state: &SessionState,
    key: &RetryStatsClusterKey,
    stats: &StatsDocument,
) -> CustomResult<bool, StorageError> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(StorageError::KVError)?;
    let store = state.store.get_revenue_recovery_retry_stats_store();

    let retry_stats_lock = state.conf().revenue_recovery.retry_stats_lock;
    let replaced = with_retry_stats_lock(
        &redis_conn,
        &key.redis_locking_key(),
        retry_stats_lock.lock_retries(),
        retry_stats_lock.delay_between_retries_in_milliseconds,
        retry_stats_lock.redis_lock_expiry_seconds,
        replace_document(store.as_ref(), key, stats),
    )
    .await?;

    if replaced.is_none() {
        logger::warn!(
            cluster_key = %key.as_db_string(),
            "revenue_recovery_retry_stats: lock contended after retries, skipping document replace"
        );
    }
    Ok(replaced.is_some())
}

/// Insert (first sight) or overwrite (thereafter) the whole stats document for one
/// cluster key; only safe while holding that key's Redis lock.
async fn replace_document(
    store: &dyn RevenueRecoveryRetryStatsInterface<Error = StorageError>,
    key: &RetryStatsClusterKey,
    stats: &StatsDocument,
) -> CustomResult<(), StorageError> {
    // A row whose stored document is unreadable still exists — a replace repairs it,
    // so treat `DeserializationFailed` as row-present rather than failing the load.
    let row_exists = match store
        .find_revenue_recovery_retry_stats_by_cluster_key(key)
        .await
    {
        Ok(maybe_row) => maybe_row.is_some(),
        Err(error) if matches!(error.current_context(), StorageError::DeserializationFailed) => {
            true
        }
        Err(error) => return Err(error),
    };

    let record = DomainRevenueRecoveryRetryStats {
        cluster_key: key.clone(),
        stats: stats.clone(),
    };

    if row_exists {
        store.update_revenue_recovery_retry_stats(record).await?;
    } else {
        store.insert_revenue_recovery_retry_stats(record).await?;
    }

    Ok(())
}

/// Serialize the read-merge-write for a cluster key under a per-key Redis lock (SETNX
/// with expiry) so concurrent recorders don't lose updates
async fn persist_node(
    store: &dyn RevenueRecoveryRetryStatsInterface<Error = StorageError>,
    redis_conn: &redis_interface::RedisConnectionWithContext,
    event: &RetryOutcomeEvent,
    lock_retries: u32,
    delay_between_retries_in_milliseconds: u32,
    redis_lock_expiry_seconds: u32,
) -> CustomResult<(), StorageError> {
    let persisted = with_retry_stats_lock(
        redis_conn,
        &event.key.redis_locking_key(),
        lock_retries,
        delay_between_retries_in_milliseconds,
        redis_lock_expiry_seconds,
        merge_and_write(store, event),
    )
    .await?;

    match persisted {
        Some(()) => logger::info!(
            cluster_key = %event.key.as_db_string(),
            success = event.success,
            "revenue_recovery_retry_stats outcome recorded"
        ),
        // Lock was still contended after the retry budget; dropping this update is
        // acceptable for best-effort stats.
        None => logger::warn!(
            cluster_key = %event.key.as_db_string(),
            "revenue_recovery_retry_stats: lock contended after retries, skipping this update"
        ),
    }
    Ok(())
}

async fn merge_and_write(
    store: &dyn RevenueRecoveryRetryStatsInterface<Error = StorageError>,
    event: &RetryOutcomeEvent,
) -> CustomResult<(), StorageError> {
    // Resolve the current document and whether a row already exists. A row whose
    // stored document is corrupt (`DeserializationFailed`) is treated as
    // present-but-empty and rebuilt, rather than dropping the event.
    let (current_doc, row_exists) = match store
        .find_revenue_recovery_retry_stats_by_cluster_key(&event.key)
        .await
    {
        Ok(Some(row)) => (Some(row.stats), true),
        Ok(None) => (None, false),
        Err(error) if matches!(error.current_context(), StorageError::DeserializationFailed) => {
            logger::error!(
                cluster_key = %event.key.as_db_string(),
                ?error,
                "revenue_recovery_retry_stats: stored document is corrupt, rebuilding from empty"
            );
            // The row exists but is unreadable, so overwrite it rather than insert.
            (None, true)
        }
        Err(error) => return Err(error),
    };

    let record = DomainRevenueRecoveryRetryStats {
        cluster_key: event.key.clone(),
        stats: current_doc.unwrap_or_default().merge(&event.delta),
    };

    // The per-key redis lock plus the read above tell us whether the row exists, so we
    // insert on first sight and update thereafter — no DB-level upsert/conflict handling.
    if row_exists {
        store.update_revenue_recovery_retry_stats(record).await?;
    } else {
        store.insert_revenue_recovery_retry_stats(record).await?;
    }

    Ok(())
}

/// Acquire the per-cluster-key Redis lock (SETNX with expiry, bounded retries), run
/// `work` while holding it, then always release it. `Ok(Some(..))` = the lock was
/// held and `work` produced its result; `Ok(None)` = the lock stayed contended for
/// the whole retry budget and `work` never ran (the caller decides how to report
/// that; no write happens in that case).
async fn with_retry_stats_lock<T>(
    redis_conn: &redis_interface::RedisConnectionWithContext,
    lock_key: &str,
    lock_retries: u32,
    delay_between_retries_in_milliseconds: u32,
    redis_lock_expiry_seconds: u32,
    work: impl core::future::Future<Output = CustomResult<T, StorageError>>,
) -> CustomResult<Option<T>, StorageError> {
    let lock_token = uuid::Uuid::new_v4().to_string();
    let wait_duration =
        std::time::Duration::from_millis(u64::from(delay_between_retries_in_milliseconds));
    let mut acquired = false;
    for _retry in 0..lock_retries {
        match redis_conn
            .set_key_if_not_exists_with_expiry(
                &lock_key.into(),
                lock_token.clone(),
                Some(i64::from(redis_lock_expiry_seconds)),
            )
            .await
        {
            Ok(SetnxReply::KeySet) => {
                acquired = true;
                break;
            }
            Ok(SetnxReply::KeyNotSet) => {
                actix_web::rt::time::sleep(wait_duration).await;
            }
            Err(error) => {
                Err(error).change_context(StorageError::KVError)?;
            }
        }
    }

    if !acquired {
        return Ok(None);
    }
    let result = work.await;
    release_lock(redis_conn, lock_key, &lock_token).await;
    result.map(Some)
}

/// Release the lock only if we still own it, so we never delete a lock that has
/// already expired and been re-acquired by another writer.
async fn release_lock(
    redis_conn: &redis_interface::RedisConnectionWithContext,
    lock_key: &str,
    lock_token: &str,
) {
    let owned = matches!(
        redis_conn.get_key::<Option<String>>(&lock_key.into()).await,
        Ok(Some(stored)) if stored == lock_token
    );
    if owned {
        if let Err(error) = redis_conn.delete_key(&lock_key.into()).await {
            logger::warn!(lock_key = %lock_key, ?error, "revenue_recovery_retry_stats: failed to release lock");
        }
    }
}
