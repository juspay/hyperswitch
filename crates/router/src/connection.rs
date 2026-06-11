use bb8::PooledConnection;
use diesel::PgConnection;
use error_stack::ResultExt;
use storage_impl::{errors as storage_errors, ReadPreference};

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
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    storage_errors::StorageError,
> {
    let pool = match store.get_read_preference() {
        ReadPreference::ReplicaDB => store.get_replica_pool(),
        ReadPreference::MasterDB => store.get_master_pool(),
    };

    pool.get()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}

pub async fn pg_accounts_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    storage_errors::StorageError,
> {
    let pool = match store.get_read_preference() {
        ReadPreference::ReplicaDB => store.get_accounts_replica_pool(),
        ReadPreference::MasterDB => store.get_accounts_master_pool(),
    };

    pool.get()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}

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
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}

pub async fn pg_accounts_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_accounts_master_pool();

    pool.get()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)
}
