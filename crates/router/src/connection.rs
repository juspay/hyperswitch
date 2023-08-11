use bb8::PooledConnection;
use diesel::PgConnection;
use error_stack::{IntoReport, ResultExt};

use crate::errors;

pub type PgPool = bb8::Pool<async_bb8_diesel::ConnectionManager<PgConnection>>;

pub type PgPooledConn = async_bb8_diesel::Connection<PgConnection>;

#[allow(clippy::expect_used)]
pub async fn redis_connection(
    conf: &crate::configs::settings::Settings,
) -> redis_interface::RedisConnectionPool {
    redis_interface::RedisConnectionPool::new(&conf.redis)
        .await
        .expect("Failed to create Redis Connection Pool")
}

pub async fn pg_connection_read(
    store: &crate::services::Store,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    errors::StorageError,
> {
    // If only OLAP is enabled get replica pool.
    #[cfg(all(feature = "olap", not(feature = "oltp")))]
    let pool = &store.diesel_store.replica_pool;

    // If either one of these are true we need to get master pool.
    //  1. Only OLTP is enabled.
    //  2. Both OLAP and OLTP is enabled.
    //  3. Both OLAP and OLTP is disabled.
    #[cfg(any(
        all(not(feature = "olap"), feature = "oltp"),
        all(feature = "olap", feature = "oltp"),
        all(not(feature = "olap"), not(feature = "oltp"))
    ))]
    let pool = &store.diesel_store.master_pool;

    pool.get()
        .await
        .into_report()
        .change_context(errors::StorageError::DatabaseConnectionError)
}

pub async fn pg_connection_write(
    store: &crate::services::Store,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = &store.diesel_store.master_pool;

    pool.get()
        .await
        .into_report()
        .change_context(errors::StorageError::DatabaseConnectionError)
}
