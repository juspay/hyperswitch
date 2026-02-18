use std::ops::Deref;

use bb8::PooledConnection;
use common_utils::errors;
use diesel::PgConnection;
use error_stack::ResultExt;

use crate::database::pool_manager::PgPoolManager;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

pub struct OwnedPooledConnection(
    PooledConnection<'static, async_bb8_diesel::ConnectionManager<PgConnection>>,
);

impl Deref for OwnedPooledConnection {
    type Target = PooledConnection<'static, async_bb8_diesel::ConnectionManager<PgConnection>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for OwnedPooledConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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

pub async fn pg_connection_read<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, crate::errors::StorageError> {
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool_manager = store.get_replica_pool_manager();

    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool_manager = store.get_master_pool_manager();

    get_connection(pool_manager).await
}

pub async fn pg_connection_write<T: crate::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<OwnedPooledConnection, crate::errors::StorageError> {
    get_connection(store.get_master_pool_manager()).await
}

pub async fn get_connection(
    pool_manager: &PgPoolManager,
) -> errors::CustomResult<OwnedPooledConnection, crate::errors::StorageError> {
    let pool = pool_manager.get_pool();
    pool.get_owned()
        .await
        .map(OwnedPooledConnection)
        .change_context(crate::errors::StorageError::DatabaseConnectionError)
        .attach_printable("Failed to get database connection")
}
