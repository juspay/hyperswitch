use diesel_async::pooled_connection::bb8::PooledConnection;
use diesel_async::AsyncPgConnection;
use error_stack::ResultExt;
use storage_impl::errors as storage_errors;

use crate::errors;

pub type PgPool = diesel_async::pooled_connection::bb8::Pool<AsyncPgConnection>;

pub type PgPooledConn = AsyncPgConnection;

/// Creates a Redis connection pool for the specified Redis settings
/// # Panics
///
/// Panics if failed to create a redis pool
#[allow(clippy::expect_used)]
pub async fn redis_connection(
    conf: &crate::configs::Settings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(&conf.redis)
        .await
        .expect("Failed to create Redis Connection Pool")
}

pub async fn pg_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'static, AsyncPgConnection>,
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

    pool.get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}

pub async fn pg_accounts_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'static, AsyncPgConnection>,
    storage_errors::StorageError,
> {
    // If only OLAP is enabled get replica pool.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool = store.get_accounts_replica_pool();

    // If either one of these are true we need to get master pool.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP is disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool = store.get_accounts_master_pool();

    pool.get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}

pub async fn pg_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'static, AsyncPgConnection>,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    pool.get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}

pub async fn pg_accounts_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'static, AsyncPgConnection>,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_accounts_master_pool();

    pool.get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}
