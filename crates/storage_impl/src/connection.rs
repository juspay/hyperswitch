use std::{ops::Deref, sync::Arc, time::Duration};

use bb8::PooledConnection;
use common_utils::errors;
use diesel::PgConnection;
use error_stack::ResultExt;

use crate::database::pool_manager::{is_connection_error_retryable, PgPoolManager};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

/// A wrapper that holds both the pool Arc and the connection.
/// This ensures the pool stays alive for the lifetime of the connection.
pub struct OwnedPooledConnection {
    /// Keep the pool alive
    _pool: Arc<PgPool>,
    /// The actual connection - uses ManuallyDrop to control drop order
    conn: Option<PooledConnection<'static, async_bb8_diesel::ConnectionManager<PgConnection>>>,
}

impl OwnedPooledConnection {
    /// Creates a new OwnedPooledConnection.
    ///
    /// # Safety
    /// The caller must ensure that `conn` was obtained from `pool`.
    fn new(
        pool: Arc<PgPool>,
        conn: PooledConnection<'static, async_bb8_diesel::ConnectionManager<PgConnection>>,
    ) -> Self {
        Self {
            _pool: pool,
            conn: Some(conn),
        }
    }
}

impl Deref for OwnedPooledConnection {
    type Target = PooledConnection<'static, async_bb8_diesel::ConnectionManager<PgConnection>>;

    #[allow(clippy::unwrap_used)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: conn is always Some until drop
        self.conn.as_ref().unwrap()
    }
}

impl std::ops::DerefMut for OwnedPooledConnection {
    #[allow(clippy::unwrap_used)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: conn is always Some until drop
        self.conn.as_mut().unwrap()
    }
}

impl Drop for OwnedPooledConnection {
    fn drop(&mut self) {
        // Drop the connection first, then the pool Arc
        // This ensures the connection is returned to the pool before the pool reference is dropped
        self.conn.take();
    }
}

/// Creates a Redis connection pool for the specified Redis settings
/// # Panics
///
/// Panics if failed to create a redis pool
#[allow(clippy::expect_used)]
pub async fn redis_connection(
    redis: &redis_interface::RedisSettings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(redis)
        .await
        .expect("Failed to create Redis Connection Pool")
}

/// Gets a database connection for read operations with automatic retry and failover recovery.
///
/// This function:
/// 1. Gets the current pool from the pool manager (atomically loaded)
/// 2. Attempts to get a connection with retry logic
/// 3. Notifies the pool manager of failures to trigger pool recreation if needed
pub async fn pg_connection_read<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, crate::errors::StorageError> {
    // If only OLAP is enabled get replica pool manager.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let (pool_manager, pool_type) = (store.get_replica_pool_manager(), "replica");

    // If either one of these are true we need to get master pool manager.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP is disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let (pool_manager, pool_type) = (store.get_master_pool_manager(), "master");

    router_env::logger::debug!(
        pool_type = pool_type,
        "[CONNECTION] pg_connection_read() - requesting connection from {} pool",
        pool_type
    );

    get_connection_with_retry(pool_manager, "read").await
}

/// Gets a database connection for write operations with automatic retry and failover recovery.
///
/// This function:
/// 1. Gets the current pool from the pool manager (atomically loaded)
/// 2. Attempts to get a connection with retry logic
/// 3. Notifies the pool manager of failures to trigger pool recreation if needed
pub async fn pg_connection_write<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, crate::errors::StorageError> {
    // Since all writes should happen to master DB only choose master pool manager.
    let pool_manager = store.get_master_pool_manager();

    router_env::logger::debug!(
        "[CONNECTION] pg_connection_write() - requesting connection from master pool"
    );

    get_connection_with_retry(pool_manager, "write").await
}

/// Internal helper function that implements retry logic for getting connections.
///
/// ## Flow:
/// 1. Load current pool from ArcSwap (lock-free, atomic)
/// 2. Try to get a connection
/// 3. On retryable error: wait with exponential backoff, retry
/// 4. After all retries exhausted: notify pool manager (may trigger recreation)
/// 5. Return connection or error
pub async fn get_connection_with_retry(
    pool_manager: &PgPoolManager,
    operation_type: &str,
) -> errors::CustomResult<OwnedPooledConnection, crate::errors::StorageError> {
    let config = pool_manager.recovery_config();
    let mut delay_ms = config.initial_retry_delay_ms;

    router_env::logger::debug!(
        operation_type = operation_type,
        max_retries = config.max_retries,
        initial_delay_ms = config.initial_retry_delay_ms,
        "[CONNECTION] get_connection_with_retry() - starting connection acquisition (max_retries={})",
        config.max_retries
    );

    for attempt in 0..=config.max_retries {
        let pool = pool_manager.get_pool();

        router_env::logger::debug!(
            operation_type = operation_type,
            attempt = attempt + 1,
            max_retries = config.max_retries + 1,
            pool_connections = pool.state().connections,
            pool_idle_connections = pool.state().idle_connections,
            "[CONNECTION] attempt {}/{} - calling pool.get_owned()",
            attempt + 1,
            config.max_retries + 1
        );

        let connection_timeout = Duration::from_secs(config.connection_attempt_timeout_secs);
        let connection_result = tokio::time::timeout(connection_timeout, pool.get_owned()).await;

        match connection_result {
            Ok(Ok(conn)) => {
                router_env::logger::debug!(
                    operation_type = operation_type,
                    attempt = attempt + 1,
                    "[CONNECTION] SUCCESS - got connection on attempt {}",
                    attempt + 1
                );
                // Success! Reset failure counter
                pool_manager.notify_connection_success();
                return Ok(OwnedPooledConnection::new(pool, conn));
            }
            Ok(Err(e)) => {
                // Pool returned an error (connection acquisition failed)
                let is_retryable = is_connection_error_retryable(&e);

                if attempt < config.max_retries && is_retryable {
                    router_env::logger::warn!(
                        operation_type = operation_type,
                        attempt = attempt + 1,
                        max_retries = config.max_retries,
                        delay_ms = delay_ms,
                        is_retryable = is_retryable,
                        error = ?e,
                        "[CONNECTION] RETRY - attempt {} failed with retryable error, waiting {}ms before retry",
                        attempt + 1,
                        delay_ms
                    );

                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(config.max_retry_delay_ms);
                    continue;
                }

                // All retries exhausted or non-retryable error
                router_env::logger::error!(
                    operation_type = operation_type,
                    attempt = attempt + 1,
                    is_retryable = is_retryable,
                    error = ?e,
                    "[CONNECTION] FAILED - all retries exhausted or non-retryable error, notifying pool manager"
                );

                pool_manager.notify_connection_failure();

                return Err(e)
                    .change_context(crate::errors::StorageError::DatabaseConnectionError)
                    .attach_printable("Failed to get database connection");
            }
            Err(_timeout) => {
                // Connection attempt timed out
                router_env::logger::warn!(
                    operation_type = operation_type,
                    attempt = attempt + 1,
                    max_retries = config.max_retries,
                    timeout_secs = config.connection_attempt_timeout_secs,
                    "[CONNECTION] TIMEOUT - attempt {} timed out after {}s, treating as retryable",
                    attempt + 1,
                    config.connection_attempt_timeout_secs
                );

                if attempt < config.max_retries {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(config.max_retry_delay_ms);
                    continue;
                }

                // All retries exhausted
                router_env::logger::error!(
                    operation_type = operation_type,
                    attempt = attempt + 1,
                    "[CONNECTION] FAILED - all retries exhausted due to timeouts, notifying pool manager"
                );

                pool_manager.notify_connection_failure();

                return Err(error_stack::report!(
                    crate::errors::StorageError::DatabaseConnectionError
                ))
                .attach_printable("Connection attempt timed out after all retries");
            }
        }
    }

    // Should not reach here, but just in case
    router_env::logger::error!(
        operation_type = operation_type,
        "[CONNECTION] UNEXPECTED - retry loop exited without returning"
    );
    Err(error_stack::report!(
        crate::errors::StorageError::DatabaseConnectionError
    ))
    .attach_printable("Connection retry loop exited unexpectedly")
}
