use diesel::PgConnection;
use storage_impl::{connection::OwnedPooledConnection, errors as storage_errors};

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

pub async fn pg_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, storage_errors::StorageError> {
    storage_impl::connection::pg_connection_read(store).await
}

pub async fn pg_accounts_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, storage_errors::StorageError> {
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool_manager = store.get_accounts_replica_pool_manager();

    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool_manager = store.get_accounts_master_pool_manager();

    storage_impl::connection::get_connection(pool_manager).await
}

pub async fn pg_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, storage_errors::StorageError> {
    storage_impl::connection::pg_connection_write(store).await
}

pub async fn pg_accounts_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, storage_errors::StorageError> {
    let pool_manager = store.get_accounts_master_pool_manager();
    storage_impl::connection::get_connection(pool_manager).await
}
