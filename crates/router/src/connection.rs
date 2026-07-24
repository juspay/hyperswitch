use bb8::PooledConnection;
use diesel::PgConnection;
use error_stack::ResultExt;
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

// Deja replay (R1): the minimal replay DB routing hook. On a just-leased pg
// connection during replay, route it to the active correlation's schema so
// per-test-case reads/writes stay isolated. The correlation is read from the
// STORE (`get_request_id`, a reliable request-scoped value set at ingress) — NOT
// the ambient thread-local, which is bled at checkout when connection acquisition
// resumes off the request's correlation span. Leases are per-op, so this fires on
// every pg op and overwrites any stale search_path a reused connection carries.
// No-op outside replay / when the store carries no request id. The `SET` SQL is
// built by the library (`deja::replay_search_path_sql_for`).
#[cfg(feature = "deja")]
async fn deja_route_replay_schema<T: storage_impl::DatabaseStore>(
    conn: &mut PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    store: &T,
) {
    use async_bb8_diesel::AsyncConnection;
    if !deja::replay_is_active() {
        return;
    }
    if let Some(corr) = store.get_request_id().as_deref() {
        let sql = deja::replay_search_path_sql_for(corr);
        let _ = conn
            .run(move |c| diesel::connection::SimpleConnection::batch_execute(c, &sql))
            .await;
    }
}

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

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut conn = pool
        .get()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&mut conn, store).await;
    Ok(conn)
}

pub async fn pg_accounts_connection_read<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
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

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut conn = pool
        .get()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&mut conn, store).await;
    Ok(conn)
}

pub async fn pg_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut conn = pool
        .get()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&mut conn, store).await;
    Ok(conn)
}

pub async fn pg_accounts_connection_write<T: storage_impl::DatabaseStore>(
    store: &T,
) -> errors::CustomResult<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_accounts_master_pool();

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut conn = pool
        .get()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&mut conn, store).await;
    Ok(conn)
}
