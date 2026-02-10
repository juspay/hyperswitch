//! PostgreSQL connection pool manager with automatic failover recovery.
//!
//! This module provides a robust connection pool wrapper that handles database failovers
//! (e.g., during blue-green deployments) using lock-free atomic pool swapping.
//!
//! ## How it works:
//! 1. Requests that fail with retryable errors are retried with exponential backoff
//! 2. After consecutive failures exceed a threshold, a new pool is created
//! 3. The old pool is atomically swapped with the new one (lock-free)
//! 4. The old pool is gracefully drained in the background

use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use arc_swap::ArcSwap;
use async_bb8_diesel::ConnectionManager;
use bb8::{Pool, RunError};
use common_utils::DbConnectionParams;
use diesel::PgConnection;
use error_stack::ResultExt;
use router_env::logger;
use tokio::sync::Mutex;

use crate::{
    config::Database,
    errors::{StorageError, StorageResult},
};

/// Type alias for the bb8 PostgreSQL connection pool
pub type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Configuration for pool recovery behavior
#[derive(Debug, Clone)]
pub struct PoolRecoveryConfig {
    /// Maximum number of retries before giving up on a single request
    pub max_retries: u32,
    /// Initial retry delay (doubles with each retry)
    pub initial_retry_delay_ms: u64,
    /// Maximum retry delay cap
    pub max_retry_delay_ms: u64,
    /// Number of consecutive failures across requests before triggering pool recreation
    pub failure_threshold: u32,
    /// Timeout for draining old pool connections
    pub old_pool_drain_timeout_secs: u64,
}

impl Default for PoolRecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_retry_delay_ms: 10,
            max_retry_delay_ms: 100,
            failure_threshold: 5,
            old_pool_drain_timeout_secs: 5,
        }
    }
}

/// A wrapper around PgPool that provides automatic failover recovery.
///
/// Uses `ArcSwap` for lock-free atomic pool swapping, allowing connections
/// to be served without blocking during pool recreation.
pub struct PgPoolManager {
    /// The current active pool (atomically swappable)
    pool: Arc<ArcSwap<PgPool>>,
    /// Database configuration for creating new pools
    db_config: Database,
    /// Schema name for the database
    schema: String,
    /// Whether to use test transactions
    test_transaction: bool,
    /// Recovery configuration
    recovery_config: PoolRecoveryConfig,
    /// Counter for consecutive failures (used to trigger pool recreation)
    consecutive_failures: AtomicU32,
    /// Mutex to prevent multiple simultaneous pool recreations
    recreation_lock: Arc<Mutex<()>>,
}

impl std::fmt::Debug for PgPoolManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgPoolManager")
            .field("schema", &self.schema)
            .field("test_transaction", &self.test_transaction)
            .field("recovery_config", &self.recovery_config)
            .field(
                "consecutive_failures",
                &self.consecutive_failures.load(Ordering::Relaxed),
            )
            .finish()
    }
}

impl Clone for PgPoolManager {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            db_config: self.db_config.clone(),
            schema: self.schema.clone(),
            test_transaction: self.test_transaction,
            recovery_config: self.recovery_config.clone(),
            consecutive_failures: AtomicU32::new(self.consecutive_failures.load(Ordering::Relaxed)),
            recreation_lock: Arc::clone(&self.recreation_lock),
        }
    }
}

impl PgPoolManager {
    /// Creates a new PgPoolManager with the given configuration.
    pub async fn new(
        db_config: Database,
        schema: String,
        test_transaction: bool,
        recovery_config: Option<PoolRecoveryConfig>,
    ) -> StorageResult<Self> {
        let config = recovery_config.clone().unwrap_or_default();
        
        logger::info!(
            schema = %schema,
            pool_size = db_config.pool_size,
            min_idle = db_config.min_idle,
            connection_timeout = db_config.connection_timeout,
            max_retries = config.max_retries,
            failure_threshold = config.failure_threshold,
            "[POOL_MANAGER] new() - creating initial pool with ArcSwap wrapper"
        );
        
        let pool = create_pool(&db_config, &schema, test_transaction).await?;
        
        logger::info!(
            schema = %schema,
            pool_state = ?pool.state(),
            "[POOL_MANAGER] new() - initial pool created successfully, wrapping in ArcSwap"
        );

        Ok(Self {
            pool: Arc::new(ArcSwap::from_pointee(pool)),
            db_config,
            schema,
            test_transaction,
            recovery_config: config,
            consecutive_failures: AtomicU32::new(0),
            recreation_lock: Arc::new(Mutex::new(())),
        })
    }

    /// Returns an Arc to the underlying pool for direct access.
    ///
    /// This provides a reference to the current pool that can be used for
    /// getting connections. The pool reference is atomically loaded, ensuring
    /// lock-free access even during pool recreation.
    pub fn get_pool(&self) -> Arc<PgPool> {
        // Load returns Guard<Arc<T>>, we clone the inner Arc
        let pool = Arc::clone(&self.pool.load());
        
        logger::debug!(
            schema = %self.schema,
            pool_state = ?pool.state(),
            consecutive_failures = self.consecutive_failures.load(Ordering::Relaxed),
            "[POOL_MANAGER] get_pool() called - returning current pool atomically"
        );
        
        pool
    }

    /// Returns the recovery configuration.
    pub fn recovery_config(&self) -> &PoolRecoveryConfig {
        &self.recovery_config
    }

    /// Notifies the manager that a connection attempt failed.
    ///
    /// This increments the failure counter and may trigger pool recreation
    /// if the failure threshold is exceeded.
    pub fn notify_connection_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
        let threshold = self.recovery_config.failure_threshold;

        if failures >= threshold {
            logger::warn!(
                schema = %self.schema,
                consecutive_failures = failures,
                threshold = threshold,
                "[POOL_MANAGER] notify_connection_failure() - THRESHOLD EXCEEDED! failures={} >= threshold={}, triggering pool recreation",
                failures,
                threshold
            );
            self.trigger_pool_recreation();
        } else {
            logger::warn!(
                schema = %self.schema,
                consecutive_failures = failures,
                threshold = threshold,
                remaining_until_recreation = threshold - failures,
                "[POOL_MANAGER] notify_connection_failure() - failure count incremented to {}/{} (need {} more to trigger recreation)",
                failures,
                threshold,
                threshold - failures
            );
        }
    }

    /// Notifies the manager that a connection was successful.
    ///
    /// This resets the failure counter.
    pub fn notify_connection_success(&self) {
        let prev_failures = self.consecutive_failures.swap(0, Ordering::Relaxed);
        
        if prev_failures > 0 {
            logger::debug!(
                schema = %self.schema,
                previous_failures = prev_failures,
                "[POOL_MANAGER] notify_connection_success() - resetting failure counter from {} to 0",
                prev_failures
            );
        }
    }

    /// Triggers pool recreation in a background task.
    ///
    /// This is a non-blocking operation - the current pool continues serving
    /// requests while the new pool is being created.
    fn trigger_pool_recreation(&self) {
        logger::info!(
            schema = %self.schema,
            "[POOL_MANAGER] trigger_pool_recreation() - spawning background task for pool recreation"
        );
        
        let manager = self.clone();

        tokio::spawn(async move {
            manager.recreate_pool().await;
        });
    }

    /// Recreates the connection pool and atomically swaps it with the old one.
    ///
    /// This method:
    /// 1. Acquires a lock to prevent multiple simultaneous recreations
    /// 2. Creates a new pool
    /// 3. Atomically swaps the old pool with the new one
    /// 4. Drains the old pool in the background
    async fn recreate_pool(&self) {
        // Try to acquire the lock - if someone else is already recreating, skip
        let Ok(_guard) = self.recreation_lock.try_lock() else {
            logger::debug!(
                schema = %self.schema,
                "[POOL_MANAGER] recreate_pool() - another recreation already in progress, skipping"
            );
            return;
        };

        logger::info!(
            schema = %self.schema,
            "[POOL_MANAGER] recreate_pool() - acquired recreation lock, starting pool creation"
        );

        // Create new pool
        let new_pool =
            match create_pool(&self.db_config, &self.schema, self.test_transaction).await {
                Ok(pool) => {
                    logger::info!(
                        schema = %self.schema,
                        pool_state = ?pool.state(),
                        "[POOL_MANAGER] recreate_pool() - new pool created successfully"
                    );
                    pool
                }
                Err(e) => {
                    logger::error!(
                        schema = %self.schema,
                        error = ?e,
                        "[POOL_MANAGER] recreate_pool() - FAILED to create new pool"
                    );
                    return;
                }
            };

        // Atomically swap the pools
        let old_pool = self.pool.swap(Arc::new(new_pool));

        // Reset failure counter
        self.consecutive_failures.store(0, Ordering::Relaxed);

        logger::info!(
            schema = %self.schema,
            old_pool_state = ?old_pool.state(),
            "[POOL_MANAGER] recreate_pool() - ATOMIC SWAP COMPLETE! New pool is now active, failure counter reset"
        );

        // Drain old pool in background with timeout
        let drain_timeout = self.recovery_config.old_pool_drain_timeout_secs;
        let schema_for_drain = self.schema.clone();
        tokio::spawn(async move {
            logger::debug!(
                schema = %schema_for_drain,
                old_pool_state = ?old_pool.state(),
                "[POOL_MANAGER] drain_old_pool - starting drain of old pool"
            );

            // Wait for old pool connections to be returned and closed
            // The pool will be dropped when all references are gone
            let drain_result = tokio::time::timeout(
                Duration::from_secs(drain_timeout),
                drain_pool_connections(old_pool),
            )
            .await;

            match drain_result {
                Ok(()) => logger::debug!(
                    schema = %schema_for_drain,
                    "[POOL_MANAGER] drain_old_pool - old pool drained successfully"
                ),
                Err(_) => logger::warn!(
                    schema = %schema_for_drain,
                    timeout_secs = drain_timeout,
                    "[POOL_MANAGER] drain_old_pool - old pool drain timed out, connections may be forcefully closed"
                ),
            }
        });
    }
}

/// Creates a new PostgreSQL connection pool with the given configuration.
async fn create_pool(
    database: &Database,
    schema: &str,
    test_transaction: bool,
) -> StorageResult<PgPool> {
    use async_bb8_diesel::AsyncConnection;
    use bb8::CustomizeConnection;

    let database_url = database.get_database_url(schema);
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    let mut pool_builder = Pool::builder()
        .max_size(database.pool_size)
        .min_idle(database.min_idle)
        .queue_strategy(database.queue_strategy.into())
        .connection_timeout(Duration::from_secs(database.connection_timeout))
        .max_lifetime(database.max_lifetime.map(Duration::from_secs));

    if test_transaction {
        #[derive(Debug)]
        struct TestTransaction;

        #[async_trait::async_trait]
        impl
            CustomizeConnection<
                async_bb8_diesel::Connection<PgConnection>,
                async_bb8_diesel::ConnectionError,
            > for TestTransaction
        {
            #[allow(clippy::unwrap_used)]
            async fn on_acquire(
                &self,
                conn: &mut async_bb8_diesel::Connection<PgConnection>,
            ) -> Result<(), async_bb8_diesel::ConnectionError> {
                use diesel::Connection;

                conn.run(|conn| {
                    conn.begin_test_transaction().unwrap();
                    Ok(())
                })
                .await
            }
        }

        pool_builder = pool_builder.connection_customizer(Box::new(TestTransaction));
    }

    pool_builder
        .build(manager)
        .await
        .change_context(StorageError::InitializationError)
        .attach_printable("Failed to create PostgreSQL connection pool")
}

/// Drains connections from an old pool by waiting for it to become empty.
async fn drain_pool_connections(pool: Arc<PgPool>) {
    // The pool will naturally drain as connections are returned
    // We just wait a bit for in-flight requests to complete
    tokio::time::sleep(Duration::from_secs(5)).await;

    // At this point, the Arc should be the only reference
    // When it's dropped, the pool will close remaining connections
    drop(pool);
}

/// Determines if an error is retryable (connection-related).
///
/// Retryable errors include:
/// - Connection timeouts
/// - Connection closed/reset
/// - Read-only transaction errors (happens during failover)
/// - Network errors
///
/// This function can be used by callers to implement their own retry logic
/// when getting connections from the pool.
pub fn is_retryable_error<E: std::fmt::Debug>(error: &RunError<E>) -> bool {
    let (is_retryable, reason) = match error {
        RunError::TimedOut => (true, "pool timeout (TimedOut)"),
        RunError::User(e) => {
            let error_str = format!("{:?}", e).to_lowercase();

            // Connection-related errors
            if error_str.contains("connection") {
                (true, "connection error")
            } else if error_str.contains("closed") {
                (true, "connection closed")
            } else if error_str.contains("reset") {
                (true, "connection reset")
            } else if error_str.contains("broken pipe") {
                (true, "broken pipe")
            } else if error_str.contains("timed out") || error_str.contains("timeout") {
                (true, "timeout")
            // Read-only transaction error (happens during failover to replica)
            } else if error_str.contains("read-only") || error_str.contains("readonly") {
                (true, "read-only transaction (failover indicator)")
            // Network errors
            } else if error_str.contains("network") {
                (true, "network error")
            } else if error_str.contains("io error") {
                (true, "I/O error")
            } else if error_str.contains("eof") {
                (true, "EOF")
            // PostgreSQL specific
            } else if error_str.contains("server closed") {
                (true, "server closed connection")
            } else if error_str.contains("terminating connection") {
                (true, "terminating connection")
            } else if error_str.contains("connection refused") {
                (true, "connection refused")
            } else {
                (false, "non-retryable error")
            }
        }
    };

    logger::debug!(
        is_retryable = is_retryable,
        reason = reason,
        error = ?error,
        "[POOL_MANAGER] is_retryable_error() - {} ({})",
        if is_retryable { "RETRYABLE" } else { "NOT RETRYABLE" },
        reason
    );

    is_retryable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_error_timeout() {
        let error: RunError<String> = RunError::TimedOut;
        assert!(is_retryable_error(&error));
    }

    #[test]
    fn test_is_retryable_error_connection_closed() {
        let error: RunError<String> = RunError::User("connection closed".to_string());
        assert!(is_retryable_error(&error));
    }

    #[test]
    fn test_is_retryable_error_read_only() {
        let error: RunError<String> =
            RunError::User("cannot execute INSERT in a read-only transaction".to_string());
        assert!(is_retryable_error(&error));
    }

    #[test]
    fn test_is_retryable_error_non_retryable() {
        let error: RunError<String> = RunError::User("unique violation".to_string());
        assert!(!is_retryable_error(&error));
    }
}
