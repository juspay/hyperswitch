use bb8::PooledConnection;
use common_utils::errors;
use diesel::PgConnection;
use error_stack::{IntoReport, ResultExt};

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

/// Creates a Redis connection pool for the specified Redis settings
/// # Panics
///
/// Panics if failed to create a redis pool
#[allow(clippy::expect_used)]
/// Establishes a new Redis connection pool using the provided Redis settings.
pub async fn redis_connection(
    redis: &redis_interface::RedisSettings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(redis)
        .await
        .expect("Failed to create Redis Connection Pool")
}

/// Asynchronously retrieves a connection from the database pool based on the enabled features.
pub async fn pg_connection_read<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    crate::errors::StorageError,
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
        .change_context(crate::errors::StorageError::DatabaseConnectionError)
}

/// Asynchronously writes to the PostgreSQL database using the provided database store.
pub async fn pg_connection_write<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    crate::errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    pool.get()
        .await
        .into_report()
        .change_context(crate::errors::StorageError::DatabaseConnectionError)
}
