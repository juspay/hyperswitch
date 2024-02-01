use bb8::PooledConnection;
use data_models::errors::StorageError;
use diesel::PgConnection;
use error_stack::{IntoReport, ResultExt};

use crate::{errors::RedisErrorExt, metrics, DatabaseStore};

/// Asynchronously retrieves a connection to the PostgreSQL database based on the provided DatabaseStore.
/// If only OLAP is enabled, it gets a replica pool. If OLTP is enabled, or both OLAP and OLTP are enabled or disabled, it gets a master pool.
/// 
/// # Arguments
///
/// * `store` - A reference to a type that implements DatabaseStore trait
///
/// # Returns
///
/// An async result containing a PooledConnection to the PostgreSQL database or a StorageError if an error occurs.
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

    pool.get()
        .await
        .into_report()
        .change_context(StorageError::DatabaseConnectionError)
}

/// Asynchronously writes to the Postgres database using the provided DatabaseStore.
/// Returns a Result containing a pooled connection to the master database, or a StorageError if an error occurs.
pub async fn pg_connection_write<T: DatabaseStore>(
    store: &T,
) -> error_stack::Result<
    PooledConnection<'_, async_bb8_diesel::ConnectionManager<PgConnection>>,
    StorageError,
> {
    // Since all writes should happen to master DB only choose master DB.
    let pool = store.get_master_pool();

    pool.get()
        .await
        .into_report()
        .change_context(StorageError::DatabaseConnectionError)
}

/// Asynchronously tries to get a value from Redis, and if the value is not found in Redis, falls back to a database call to retrieve the value. 
/// 
/// # Arguments
/// 
/// * `redis_fut` - A future that resolves to a result from Redis.
/// * `database_call_closure` - A closure that represents the database call to be made if the value is not found in Redis.
/// 
/// # Returns
/// 
/// Returns a `Result` containing the retrieved value from either Redis or the database, or an error if the retrieval fails.
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
                metrics::KV_MISS.add(&metrics::CONTEXT, 1, &[]);
                database_call_closure().await
            }
            // Keeping the key empty here since the error would never go here.
            _ => Err(redis_error.to_redis_failed_response("")),
        },
    }
}
