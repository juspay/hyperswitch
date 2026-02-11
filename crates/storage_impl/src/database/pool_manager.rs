//! PostgreSQL connection pool manager with automatic failover recovery.
//!
//! This module provides a robust connection pool wrapper that handles database failovers
//! (e.g., during blue-green deployments) using lock-free atomic pool swapping.
//!
//! ## How failover detection works:
//!
//! ### Layer 1: Connection Acquisition (handled by bb8)
//! - Pool timeout, connection refused, etc.
//! - Detected by `is_connection_error_retryable()` in connection.rs
//! - Rarely triggers during failover since existing connections appear healthy
//!
//! ### Layer 2: Query Execution (the critical one for failover)
//! - "cannot execute INSERT in a read-only transaction" errors
//! - Detected by `is_failover_error()` in this module
//! - Called from error handling in storage methods via `check_and_handle_failover_error()`
//!
//! ## Recovery Flow:
//! 1. Query fails with "read-only transaction" error
//! 2. Error handler calls `check_and_handle_failover_error(error_msg)`
//! 3. If failover detected, immediately trigger pool recreation (no threshold)
//! 4. New pool is created in background with fresh connections to the new primary
//! 5. Old pool is atomically swapped out and drained
//! 6. Subsequent requests get connections from the new pool

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
use router_env::{logger, tracing::Instrument};
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
    /// Timeout for each connection attempt (should be much shorter than gateway timeout)
    pub connection_attempt_timeout_secs: u64,
}

impl Default for PoolRecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_retry_delay_ms: 10,
            max_retry_delay_ms: 100,
            failure_threshold: 2,  // Reduced from 5 - trigger recreation faster
            old_pool_drain_timeout_secs: 5,
            connection_attempt_timeout_secs: 5,  // 5 seconds per attempt, so 4 attempts = 20s max
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

    /// Checks if a query execution error indicates a database failover and triggers pool recreation if needed.
    /// 
    /// This is the main entry point for failover detection at the query execution layer.
    /// It should be called when a query fails with an error. If the error indicates that
    /// the database has failed over to a read-only replica, this will immediately trigger
    /// pool recreation without waiting for any threshold.
    ///
    /// ## Example usage in storage methods:
    /// ```ignore
    /// .map_err(|error| {
    ///     let error_msg = format!("{:?}", error);
    ///     pool_manager.check_and_handle_failover_error(&error_msg);
    ///     // continue with error conversion...
    /// })
    /// ```
    ///
    /// Returns `true` if the error was a failover indicator and pool recreation was triggered.
    pub fn check_and_handle_failover_error(&self, error_message: &str) -> bool {
        if is_failover_error(error_message) {
            logger::error!(
                schema = %self.schema,
                error_snippet = &error_message[..error_message.len().min(200)],
                "[FAILOVER] DATABASE FAILOVER DETECTED! Triggering immediate pool recreation. \
                 Error indicates connection to read-only replica. Current request will fail, \
                 subsequent requests will use fresh pool with connections to new primary."
            );
            
            // Immediately trigger pool recreation - don't wait for any threshold
            // This is critical for fast recovery after failover
            self.trigger_pool_recreation();
            
            true
        } else {
            false
        }
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
            db_host = %self.db_config.host,
            "[POOL_MANAGER] recreate_pool() - acquired recreation lock, waiting for DNS propagation before creating new pool"
        );

        // RDS cluster endpoint DNS TTL is typically 5 seconds, but local caches may hold longer
        // Retry with increasing delays to ensure DNS has propagated to new primary
        let dns_delays = [5, 10, 15]; // Total max wait: 30 seconds
        
        for (attempt, delay_secs) in dns_delays.iter().enumerate() {
            logger::info!(
                schema = %self.schema,
                db_host = %self.db_config.host,
                attempt = attempt + 1,
                delay_secs = delay_secs,
                "[POOL_MANAGER] recreate_pool() - waiting {}s for DNS propagation (attempt {}/{})",
                delay_secs,
                attempt + 1,
                dns_delays.len()
            );

            tokio::time::sleep(Duration::from_secs(*delay_secs)).await;

            // Create new pool and validate it's writable
            let new_pool = match create_pool(&self.db_config, &self.schema, self.test_transaction).await {
                Ok(pool) => pool,
                Err(e) => {
                    logger::error!(
                        schema = %self.schema,
                        error = ?e,
                        attempt = attempt + 1,
                        "[POOL_MANAGER] recreate_pool() - FAILED to create new pool, will retry"
                    );
                    continue;
                }
            };

            // Validate the new pool connects to a writable primary
            match validate_pool_is_writable(&new_pool).await {
                Ok(true) => {
                    logger::info!(
                        schema = %self.schema,
                        pool_state = ?new_pool.state(),
                        attempt = attempt + 1,
                        "[POOL_MANAGER] recreate_pool() - new pool VALIDATED as writable!"
                    );

                    // Atomically swap the pools
                    let old_pool = self.pool.swap(Arc::new(new_pool));
                    self.consecutive_failures.store(0, Ordering::Relaxed);

                    logger::info!(
                        schema = %self.schema,
                        old_pool_state = ?old_pool.state(),
                        "[POOL_MANAGER] recreate_pool() - ATOMIC SWAP COMPLETE! New pool is now active"
                    );

                    // Drain old pool in background
                    let drain_timeout = self.recovery_config.old_pool_drain_timeout_secs;
                    let _schema_for_drain = self.schema.clone();
                    tokio::spawn(async move {
                        let _ = tokio::time::timeout(
                            Duration::from_secs(drain_timeout),
                            drain_pool_connections(old_pool),
                        ).await;
                    }.in_current_span());

                    return; // Success!
                }
                Ok(false) => {
                    logger::warn!(
                        schema = %self.schema,
                        attempt = attempt + 1,
                        "[POOL_MANAGER] recreate_pool() - new pool still connecting to READ-ONLY replica, will retry"
                    );
                    // Drop this pool and retry
                    continue;
                }
                Err(e) => {
                    logger::warn!(
                        schema = %self.schema,
                        error = %e,
                        attempt = attempt + 1,
                        "[POOL_MANAGER] recreate_pool() - failed to validate pool, will retry"
                    );
                    continue;
                }
            }
        }

        logger::error!(
            schema = %self.schema,
            "[POOL_MANAGER] recreate_pool() - FAILED after all retries! DNS may not have propagated yet. \
             Will retry on next failover error."
        );
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
        }.in_current_span());
    }
}

/// Validates that a pool connects to a writable (primary) database.
/// Returns Ok(true) if writable, Ok(false) if read-only, Err on connection failure.
#[allow(unused_qualifications)]
async fn validate_pool_is_writable(pool: &PgPool) -> Result<bool, String> {
    use async_bb8_diesel::AsyncRunQueryDsl;
    use diesel::prelude::*;
    use diesel::sql_query;
    use diesel::sql_types::Bool;
    
    #[derive(QueryableByName, Debug)]
    struct ReadOnlyCheck {
        #[diesel(sql_type = Bool)]
        is_read_only: bool,
    }

    let conn = pool.get().await.map_err(|e| format!("Failed to get connection: {:?}", e))?;
    
    // Check PostgreSQL's transaction_read_only setting
    let result: Result<ReadOnlyCheck, _> = 
        sql_query("SELECT current_setting('transaction_read_only')::boolean AS is_read_only")
            .get_result_async(&*conn)
            .await;

    match result {
        Ok(check) => {
            if check.is_read_only {
                logger::debug!("[POOL_VALIDATE] Database is in READ-ONLY mode");
                Ok(false)
            } else {
                logger::debug!("[POOL_VALIDATE] Database is WRITABLE");
                Ok(true)
            }
        }
        Err(e) => Err(format!("Query error: {:?}", e)),
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

/// Checks if an error message indicates a database failover condition.
///
/// This is the primary function for detecting failover at the **query execution layer**.
/// It should be called when a database query fails to determine if the error indicates
/// that the database has failed over to a read-only replica or is otherwise unavailable.
///
/// ## Failover Indicators Detected:
/// - "read-only transaction" / "readonly" - Connection went to a read replica
/// - "cannot execute" with write operations - PostgreSQL write blocked on replica
/// - "server closed" / "connection reset" - Server terminated the connection
/// - "terminating connection" - PostgreSQL shutdown/failover in progress
/// - "broken pipe" - Connection forcefully closed
/// - "connection refused" - New primary not yet available at old address
///
/// ## Usage:
/// This is called internally by `PgPoolManager::check_and_handle_failover_error()`.
/// Storage methods should use `diesel_error_to_data_error_with_failover_check()`.
pub fn is_failover_error(error_message: &str) -> bool {
    let error_lower = error_message.to_lowercase();
    
    // Primary failover indicator: read-only transaction error
    // This is the most common signal during blue-green deployment
    if error_lower.contains("read-only") || error_lower.contains("readonly") {
        logger::warn!(
            matched_pattern = "read-only/readonly",
            "[FAILOVER_DETECTION] MATCHED: read-only transaction error - primary failover indicator"
        );
        return true;
    }
    
    // PostgreSQL-specific failover indicators
    if error_lower.contains("cannot execute") && 
       (error_lower.contains("insert") || error_lower.contains("update") || 
        error_lower.contains("delete") || error_lower.contains("write")) {
        logger::warn!(
            matched_pattern = "cannot execute write operation",
            "[FAILOVER_DETECTION] MATCHED: cannot execute write operation on replica"
        );
        return true;
    }
    
    // Connection forcefully terminated (server-side failover)
    if error_lower.contains("server closed") {
        logger::warn!(
            matched_pattern = "server closed",
            "[FAILOVER_DETECTION] MATCHED: server closed the connection"
        );
        return true;
    }
    
    if error_lower.contains("terminating connection") {
        logger::warn!(
            matched_pattern = "terminating connection",
            "[FAILOVER_DETECTION] MATCHED: PostgreSQL terminating connection"
        );
        return true;
    }
    
    if error_lower.contains("connection reset") {
        logger::warn!(
            matched_pattern = "connection reset",
            "[FAILOVER_DETECTION] MATCHED: connection reset by peer"
        );
        return true;
    }
    
    if error_lower.contains("broken pipe") {
        logger::warn!(
            matched_pattern = "broken pipe",
            "[FAILOVER_DETECTION] MATCHED: broken pipe"
        );
        return true;
    }
    
    // Connection refused (DNS/load balancer already pointed to new primary but pool has old connections)
    if error_lower.contains("connection refused") {
        logger::warn!(
            matched_pattern = "connection refused",
            "[FAILOVER_DETECTION] MATCHED: connection refused"
        );
        return true;
    }
    
    // PostgreSQL standby/replica mode indicators
    if error_lower.contains("hot standby") || error_lower.contains("recovery mode") {
        logger::warn!(
            matched_pattern = "hot standby/recovery mode",
            "[FAILOVER_DETECTION] MATCHED: database in standby/recovery mode"
        );
        return true;
    }
    
    // Not a failover error
    logger::debug!(
        error_snippet = %error_lower.chars().take(150).collect::<String>(),
        "[FAILOVER_DETECTION] No failover pattern matched in error"
    );
    
    false
}

/// Determines if a connection acquisition error is retryable.
///
/// **Note:** This only handles errors during `pool.get()` - connection acquisition.
/// During typical failover, connections in the pool are still TCP-connected, so this
/// rarely triggers. The main failover detection happens via `is_failover_error()`.
///
/// This function handles:
/// - Pool timeout (all connections busy)
/// - Connection errors during checkout
/// - Network errors
pub fn is_connection_error_retryable<E: std::fmt::Debug>(error: &RunError<E>) -> bool {
    let (is_retryable, reason) = match error {
        RunError::TimedOut => (true, "pool timeout"),
        RunError::User(e) => {
            let error_str = format!("{:?}", e).to_lowercase();

            if error_str.contains("connection") || 
               error_str.contains("closed") || 
               error_str.contains("reset") ||
               error_str.contains("broken pipe") ||
               error_str.contains("timed out") || 
               error_str.contains("timeout") ||
               error_str.contains("network") ||
               error_str.contains("io error") ||
               error_str.contains("eof") ||
               error_str.contains("refused") {
                (true, "connection/network error")
            } else {
                (false, "non-retryable")
            }
        }
    };

    if is_retryable {
        logger::debug!(
            reason = reason,
            error = ?error,
            "[POOL_MANAGER] connection error is retryable: {}",
            reason
        );
    }

    is_retryable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_failover_error_read_only() {
        assert!(is_failover_error("cannot execute INSERT in a read-only transaction"));
        assert!(is_failover_error("ERROR: cannot execute UPDATE in a read-only transaction"));
        assert!(is_failover_error("readonly mode"));
    }

    #[test]
    fn test_is_failover_error_connection_terminated() {
        assert!(is_failover_error("server closed the connection unexpectedly"));
        assert!(is_failover_error("terminating connection due to administrator command"));
        assert!(is_failover_error("connection reset by peer"));
        assert!(is_failover_error("broken pipe"));
    }

    #[test]
    fn test_is_failover_error_standby() {
        assert!(is_failover_error("cannot execute in read-only hot standby mode"));
        assert!(is_failover_error("database is in recovery mode"));
    }

    #[test]
    fn test_is_failover_error_non_failover() {
        assert!(!is_failover_error("unique violation"));
        assert!(!is_failover_error("foreign key constraint"));
        assert!(!is_failover_error("not found"));
        assert!(!is_failover_error("syntax error"));
    }

    #[test]
    fn test_is_connection_error_retryable_timeout() {
        let error: RunError<String> = RunError::TimedOut;
        assert!(is_connection_error_retryable(&error));
    }

    #[test]
    fn test_is_connection_error_retryable_connection() {
        let error: RunError<String> = RunError::User("connection closed".to_string());
        assert!(is_connection_error_retryable(&error));
    }

    #[test]
    fn test_is_connection_error_retryable_non_retryable() {
        let error: RunError<String> = RunError::User("unique violation".to_string());
        assert!(!is_connection_error_retryable(&error));
    }
}
