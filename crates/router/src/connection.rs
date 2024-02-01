use bb8::PooledConnection;
use diesel::PgConnection;
use error_stack::{IntoReport, ResultExt};
use storage_impl::errors as storage_errors;

use crate::errors;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

/// Creates a Redis connection pool for the specified Redis settings
/// # Panics
///
/// Panics if failed to create a redis pool
#[allow(clippy::expect_used)]
/// Establishes a connection to a Redis server using the provided settings configuration.
///
/// # Arguments
///
/// * `conf` - A reference to the settings configuration containing the Redis connection details.
///
/// # Returns
///
/// A `RedisConnectionPool` that represents a pool of connections to the Redis server.
pub async fn redis_connection(
    conf: &crate::configs::settings::Settings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(&conf.redis)
        .await
        .expect("Failed to create Redis Connection Pool")
}

/// This method selects the appropriate database connection pool based on the configured features.
pub async fn pg_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    storage_errors::StorageError,
> {
    // If only OLAP is enabled get replica pool.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool = store.get_replica_pool();

    // If either one of these are true we need to get master pool.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP is disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool = store.get_master_pool();

    pool.get()
        .await
        .into_report()
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}

/// Asynchronously writes to the PostgreSQL database using the provided database store.
/// It first obtains the master database connection pool from the store, then gets a connection from the pool and awaits it. 
/// If successful, it changes the context of any potential connection error to a storage error and returns the obtained connection.
pub async fn pg_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    pool.get()
        .await
        .into_report()
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}
