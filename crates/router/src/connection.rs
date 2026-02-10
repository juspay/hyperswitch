use diesel::PgConnection;
use storage_impl::connection::OwnedPooledConnection;
use storage_impl::errors as storage_errors;

use crate::errors;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

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

/// Gets a database connection for read operations with automatic retry and failover recovery.
pub async fn pg_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, storage_errors::StorageError> {
    storage_impl::connection::pg_connection_read(store).await
}

/// Gets a database connection for read operations on the accounts database
/// with automatic retry and failover recovery.
pub async fn pg_accounts_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, storage_errors::StorageError> {
    // If only OLAP is enabled get replica pool manager.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let (pool_manager, pool_type) = (store.get_accounts_replica_pool_manager(), "accounts_replica");

    // If either one of these are true we need to get master pool manager.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let (pool_manager, pool_type) = (store.get_accounts_master_pool_manager(), "accounts_master");

    router_env::logger::debug!(
        pool_type = pool_type,
        "[CONNECTION] pg_accounts_connection_read() - requesting connection from {} pool",
        pool_type
    );

    storage_impl::connection::get_connection_with_retry(pool_manager, "accounts_read").await
}

/// Gets a database connection for write operations with automatic retry and failover recovery.
pub async fn pg_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, storage_errors::StorageError> {
    storage_impl::connection::pg_connection_write(store).await
}

/// Gets a database connection for write operations on the accounts database
/// with automatic retry and failover recovery.
pub async fn pg_accounts_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, storage_errors::StorageError> {
    // Since all writes should happen to master DB only choose master pool manager.
    let pool_manager = store.get_accounts_master_pool_manager();

    router_env::logger::debug!(
        "[CONNECTION] pg_accounts_connection_write() - requesting connection from accounts_master pool"
    );

    storage_impl::connection::get_connection_with_retry(pool_manager, "accounts_write").await
}
