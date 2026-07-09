use bb8::PooledConnection;
use diesel::PgConnection;
use error_stack::ResultExt;

use crate::{
    errors::{RedisErrorExt, StorageError},
    metrics, DatabaseStore,
};

// Deja replay (R1): the per-correlation DB routing hook, at the ACTUAL storage
// connection seam (the store methods acquire connections here, not via the
// router-crate copy). On a just-leased pg connection during replay, SET
// search_path to the active correlation's schema, derived from the store's
// request_id — a reliable, request-scoped value, NOT the ambient thread-local
// (bled at checkout). No-op outside replay / when the store carries no request id.
// The SET SQL is built by the library (`deja::replay_search_path_sql_for`).
#[cfg(feature = "deja")]
pub(crate) async fn deja_route_replay_schema<T: DatabaseStore>(
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

pub async fn pg_connection_read<T: DatabaseStore>(
    store: &T,
) -> error_stack::Result<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    StorageError,
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
        .change_context(StorageError::DatabaseConnectionError)?;
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&mut conn, store).await;
    Ok(conn)
}

pub async fn pg_connection_write<T: DatabaseStore>(
    store: &T,
) -> error_stack::Result<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut conn = pool
        .get()
        .await
        .change_context(StorageError::DatabaseConnectionError)?;
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&mut conn, store).await;
    Ok(conn)
}

pub async fn pg_accounts_connection_read<T: DatabaseStore>(
    store: &T,
) -> error_stack::Result<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    StorageError,
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
        .change_context(StorageError::DatabaseConnectionError)?;
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&mut conn, store).await;
    Ok(conn)
}

pub async fn pg_accounts_connection_write<T: DatabaseStore>(
    store: &T,
) -> error_stack::Result<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_accounts_master_pool();

    #[cfg_attr(not(feature = "deja"), allow(unused_mut))]
    let mut conn = pool
        .get()
        .await
        .change_context(StorageError::DatabaseConnectionError)?;
    #[cfg(feature = "deja")]
    deja_route_replay_schema(&mut conn, store).await;
    Ok(conn)
}

pub async fn try_redis_get_else_try_database_get<F, RFut, DFut, T>(
    redis_fut: RFut,
    database_call_closure: F,
) -> error_stack::Result<T, StorageError>
where
    F: FnOnce() -> DFut,
    RFut: futures::Future<Output = error_stack::Result<T, redis_interface::errors::RedisError>>,
    DFut: futures::Future<Output = error_stack::Result<T, StorageError>>,
{
    let redis_output = redis_fut.await;
    match redis_output {
        Ok(output) => Ok(output),
        Err(redis_error) => match redis_error.current_context() {
            redis_interface::errors::RedisError::NotFound => {
                metrics::KV_MISS.add(1, &[]);
                database_call_closure().await
            }
            // Keeping the key empty here since the error would never go here.
            _ => Err(redis_error.to_redis_failed_response("")),
        },
    }
}

use std::collections::HashSet;

use crate::UniqueConstraints;

fn union_vec<T>(mut kv_rows: Vec<T>, sql_rows: Vec<T>) -> Vec<T>
where
    T: UniqueConstraints,
{
    let mut kv_unique_keys = HashSet::new();

    kv_rows.iter().for_each(|v| {
        kv_unique_keys.insert(v.unique_constraints().concat());
    });

    sql_rows.into_iter().for_each(|v| {
        let unique_key = v.unique_constraints().concat();
        if !kv_unique_keys.contains(&unique_key) {
            kv_rows.push(v);
        }
    });

    kv_rows
}

pub async fn find_all_combined_kv_database<F, RFut, DFut, T>(
    redis_fut: RFut,
    database_call: F,
    limit: Option<i64>,
) -> error_stack::Result<Vec<T>, StorageError>
where
    T: UniqueConstraints,
    F: FnOnce() -> DFut,
    RFut:
        futures::Future<Output = error_stack::Result<Vec<T>, redis_interface::errors::RedisError>>,
    DFut: futures::Future<Output = error_stack::Result<Vec<T>, StorageError>>,
{
    let trunc = |v: &mut Vec<_>| {
        if let Some(l) = limit.and_then(|v| TryInto::try_into(v).ok()) {
            v.truncate(l);
        }
    };

    let limit_satisfies = |len: usize, limit: i64| {
        TryInto::try_into(limit)
            .ok()
            .is_none_or(|val: usize| len >= val)
    };

    let redis_output = redis_fut.await;
    match (redis_output, limit) {
        (Ok(mut kv_rows), Some(lim)) if limit_satisfies(kv_rows.len(), lim) => {
            trunc(&mut kv_rows);
            Ok(kv_rows)
        }
        (Ok(kv_rows), _) => database_call().await.map(|db_rows| {
            let mut res = union_vec(kv_rows, db_rows);
            trunc(&mut res);
            res
        }),
        (Err(redis_error), _) => match redis_error.current_context() {
            redis_interface::errors::RedisError::NotFound => {
                metrics::KV_MISS.add(1, &[]);
                database_call().await
            }
            // Keeping the key empty here since the error would never go here.
            _ => Err(redis_error.to_redis_failed_response("")),
        },
    }
}
