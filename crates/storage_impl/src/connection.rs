use common_utils::errors;
use error_stack::ResultExt;

pub type PgPool = diesel_async::pooled_connection::bb8::Pool<diesel_async::AsyncPgConnection>;

pub type PgPooledConn = diesel_async::AsyncPgConnection;

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

pub async fn pg_connection_read<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    diesel_async::pooled_connection::bb8::PooledConnection<
        'static,
        diesel_async::AsyncPgConnection,
    >,
    crate::errors::StorageError,
> {
    // If only OLAP is enabled get replica pool.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool = store.get_replica_pool();

    // If either one of these are true we need to get master pool.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP are disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool = store.get_master_pool();

    pool.get_owned()
        .await
        .change_context(crate::errors::StorageError::DatabaseConnectionError)
}

pub async fn pg_connection_write<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    diesel_async::pooled_connection::bb8::PooledConnection<
        'static,
        diesel_async::AsyncPgConnection,
    >,
    crate::errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    pool.get_owned()
        .await
        .change_context(crate::errors::StorageError::DatabaseConnectionError)
}
