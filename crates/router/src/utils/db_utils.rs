use crate::{
    core::errors::{self, utils::RedisErrorExt},
    routes::metrics,
};

/// Generates hscan field pattern. Suppose the field is pa_1234_ref_1211 it will generate
/// pa_1234_ref_*
pub fn generate_hscan_pattern_for_refund(sk: &str) -> String {
    sk.split('_')
        .take(3)
        .chain(["*"])
        .collect::<Vec<&str>>()
        .join("_")
}

// The first argument should be a future while the second argument should be a closure that returns a future for a database call
pub async fn try_redis_get_else_try_database_get<F, RFut, DFut, T>(
    redis_fut: RFut,
    database_call_closure: F,
) -> error_stack::Result<T, errors::StorageError>
where
    F: FnOnce() -> DFut,
    RFut: futures::Future<Output = error_stack::Result<T, redis_interface::errors::RedisError>>,
    DFut: futures::Future<Output = error_stack::Result<T, errors::StorageError>>,
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

pub async fn find_all_combined_kv_database<F, RFut, DFut, T>(
    redis_fut: RFut,
    database_call: F,
    limit: Option<i64>,
) -> error_stack::Result<Vec<T>, errors::StorageError>
where
    T: UniqueConstraints,
    F: FnOnce() -> DFut,
    RFut:
        futures::Future<Output = error_stack::Result<Vec<T>, redis_interface::errors::RedisError>>,
    DFut: futures::Future<Output = error_stack::Result<Vec<T>, errors::StorageError>>,
{
    let trunc = |v: &mut Vec<_>| if let Some(l)
        = limit.and_then(|v| TryInto::try_into(v).ok())
    { v.truncate(l); };

    let limit_satisfies = |len : usize, limit : i64| TryInto::try_into(limit).ok().map_or(true,  |val : usize| len >= val);
    
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
                metrics::KV_MISS.add(&metrics::CONTEXT, 1, &[]);
                database_call().await
            }
            // Keeping the key empty here since the error would never go here.
            _ => Err(redis_error.to_redis_failed_response("")),
        },
    }
}

use std::collections::HashSet;

use storage_impl::UniqueConstraints;

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
