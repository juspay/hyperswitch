use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_domain_models::revenue_recovery::retry_stats::RevenueRecoveryRetryStats as DomainRevenueRecoveryRetryStats;
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
    /// Redis key of the per-cluster-key lock guarding this event's read-merge-write.
    fn get_redis_locking_key(&self) -> String {
        format!(
            "revenue_recovery_retry_stats_lock:{}",
            self.key.as_db_string()
        )
    }

    /// Record this outcome into `revenue_recovery_retry_stats` under a per-key
    /// Redis lock so concurrent writers don't lose updates in the read-merge-write.
    /// Recording is best-effort: any failure is logged and swallowed so it never
    /// affects the payment/recovery flow that invoked it.
    #[instrument(skip_all)]
    pub async fn record(&self, state: &SessionState) {
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
            return;
        }

        let redis_conn = match state.store.get_redis_conn() {
            Ok(conn) => conn,
            Err(error) => {
                logger::error!(
                    ?error,
                    "revenue_recovery_retry_stats: unable to acquire redis connection"
                );
                return;
            }
        };
        let store = state.store.get_revenue_recovery_retry_stats_store();

        let db_key = self.key.as_db_string();

        let retry_stats_lock = state.conf().revenue_recovery.retry_stats_lock;
        let lock_retries = retry_stats_lock.lock_retries();
        let delay_between_retries_in_milliseconds =
            retry_stats_lock.delay_between_retries_in_milliseconds;
        let redis_lock_expiry_seconds = retry_stats_lock.redis_lock_expiry_seconds;

        match persist_node(
            store.as_ref(),
            &redis_conn,
            self,
            lock_retries,
            delay_between_retries_in_milliseconds,
            redis_lock_expiry_seconds,
        )
        .await
        {
            Ok(()) => logger::info!(
                cluster_key = %db_key,
                success = self.success,
                "revenue_recovery_retry_stats outcome recorded"
            ),
            Err(error) => logger::error!(
                cluster_key = %db_key,
                ?error,
                "revenue_recovery_retry_stats: failed to persist retry outcome"
            ),
        }
    }
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
    let db_key = event.key.as_db_string();
    let lock_key = event.get_redis_locking_key();
    let lock_token = uuid::Uuid::new_v4().to_string();

    let wait_duration =
        std::time::Duration::from_millis(u64::from(delay_between_retries_in_milliseconds));
    let mut acquired = false;
    for _retry in 0..lock_retries {
        match redis_conn
            .set_key_if_not_exists_with_expiry(
                &lock_key.as_str().into(),
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

    if acquired {
        let result = merge_and_write(store, event).await;
        release_lock(redis_conn, &lock_key, &lock_token).await;
        result
    } else {
        // Lock was still contended after the retry budget; dropping this update is
        // acceptable for best-effort stats.
        logger::warn!(
            cluster_key = %db_key,
            "revenue_recovery_retry_stats: lock contended after retries, skipping this update"
        );
        Ok(())
    }
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
