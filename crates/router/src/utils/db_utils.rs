use crate::{core::errors, routes::metrics};

#[cfg(feature = "kv_store")]
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
) -> errors::CustomResult<T, errors::StorageError>
where
    F: FnOnce() -> DFut,
    RFut: futures::Future<Output = errors::CustomResult<T, redis_interface::errors::RedisError>>,
    DFut: futures::Future<Output = errors::CustomResult<T, errors::StorageError>>,
{
    let redis_output = redis_fut.await;
    match redis_output {
        Ok(output) => Ok(output),
        Err(redis_error) => match redis_error.current_context() {
            redis_interface::errors::RedisError::NotFound => {
                metrics::KV_MISS.add(&metrics::CONTEXT, 1, &[]);
                database_call_closure().await
            }
            _ => Err(redis_error.change_context(errors::StorageError::KVError)),
        },
    }
}
