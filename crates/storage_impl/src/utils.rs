use bb8::PooledConnection;
use diesel::PgConnection;
use error_stack::ResultExt;

use crate::{
    errors::{RedisErrorExt, StorageError},
    metrics, DatabaseStore,
};

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
        .change_context(StorageError::DatabaseConnectionError)
}

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
        .change_context(StorageError::DatabaseConnectionError)
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
            .map_or(true, |val: usize| len >= val)
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


/// Trait for converting from one foreign type to another
pub(crate) trait ForeignFrom<F> {
    /// Convert from a foreign type to the current type
    fn foreign_from(from: F) -> Self;
}

/// Trait for converting from one foreign type to another
pub(crate) trait ForeignTryFrom<F>: Sized {
    /// Custom error for conversion failure
    type Error;
    /// Convert from a foreign type to the current type and return an error if the conversion fails
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}


pub trait ForeignInto<T> {
    fn foreign_into(self) -> T;
}

pub trait ForeignTryInto<T> {
    type Error;

    fn foreign_try_into(self) -> Result<T, Self::Error>;
}

impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}

impl<F, T> ForeignTryInto<T> for F
where
    T: ForeignTryFrom<F>,
{
    type Error = <T as ForeignTryFrom<F>>::Error;

    fn foreign_try_into(self) -> Result<T, Self::Error> {
        T::foreign_try_from(self)
    }
}
