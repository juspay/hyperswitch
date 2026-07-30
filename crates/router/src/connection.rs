use common_utils::request_context::RequestContext;
use diesel_models::DatabaseConnectionWithContext;
use error_stack::ResultExt;
use storage_impl::{errors as storage_errors, DatabaseStore};

use crate::errors;

pub type PgPooledConn = diesel_models::PgPooledConn;

// Deja replay (R1): the minimal replay DB routing hook. On a just-leased pg
// connection during replay, route it to the active correlation's schema so
// per-test-case reads/writes stay isolated. The correlation is read from the
// STORE (`get_request_id`, a reliable request-scoped value set at ingress) — NOT
// the ambient thread-local, which is bled at checkout when connection acquisition
// resumes off the request's correlation span. Leases are per-op, so this fires on
// every pg op and overwrites any stale search_path a reused connection carries.
// No-op outside replay / when the store carries no request id. The `SET` SQL is
// built by the library (`deja::replay_search_path_sql_for`).
//
// Duplicated from `storage_impl::utils::deja_route_replay_schema` rather than
// called directly: that copy is `pub(crate)` to `storage_impl`, and this crate
// needs its own leased connection routed the same way. Takes
// `&DatabaseConnectionWithContext` rather than `&mut PooledConnection`: the
// underlying `RawPgConnection` wraps an `Arc<Mutex<_>>` internally, so
// `AsyncConnection::run` only needs `&self`.
#[cfg(feature = "deja")]
async fn deja_route_replay_schema<T: DatabaseStore>(
    conn: &DatabaseConnectionWithContext,
    store: &T,
) {
    use async_bb8_diesel::AsyncConnection;
    if !deja::replay_is_active() {
        return;
    }
    if let Some(corr) = store.get_request_id().as_deref() {
        let sql = deja::replay_search_path_sql_for(corr);
        let _ = conn
            .raw_connection()
            .run(move |c| diesel::connection::SimpleConnection::batch_execute(c, &sql))
            .await;
    }
}

pub async fn pg_connection_read<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<
    DatabaseConnectionWithContext,
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

    let connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;

    let conn = DatabaseConnectionWithContext::new(
        connection,
        store.request_id().map(str::to_owned),
        pool.event_emitter.clone(),
    );
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&conn, store).await;
    Ok(conn)
}

pub async fn pg_accounts_connection_read<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<
    DatabaseConnectionWithContext,
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

    let connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;

    let conn = DatabaseConnectionWithContext::new(
        connection,
        store.request_id().map(str::to_owned),
        pool.event_emitter.clone(),
    );
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&conn, store).await;
    Ok(conn)
}

pub async fn pg_connection_write<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<
    DatabaseConnectionWithContext,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    let connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;

    let conn = DatabaseConnectionWithContext::new(
        connection,
        store.request_id().map(str::to_owned),
        pool.event_emitter.clone(),
    );
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&conn, store).await;
    Ok(conn)
}

pub async fn pg_accounts_connection_write<T: DatabaseStore + RequestContext>(
    store: &T,
) -> errors::CustomResult<
    DatabaseConnectionWithContext,
    storage_errors::StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_accounts_master_pool();

    let connection = pool
        .pg_pool
        .get_owned()
        .await
        .change_context(storage_errors::StorageError::DatabaseConnectionError)?;

    let conn = DatabaseConnectionWithContext::new(
        connection,
        store.request_id().map(str::to_owned),
        pool.event_emitter.clone(),
    );
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&conn, store).await;
    Ok(conn)
}
