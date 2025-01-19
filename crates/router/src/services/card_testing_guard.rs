use std::sync::Arc;

use error_stack::ResultExt;
use redis_interface::RedisConnectionPool;

use crate::{
    core::errors::{ApiErrorResponse, RouterResult},
    routes::app::SessionStateInfo,
};

fn get_redis_connection<A: SessionStateInfo>(state: &A) -> RouterResult<Arc<RedisConnectionPool>> {
    state
        .store()
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")
}

pub async fn set_blocked_count_in_cache<A>(
    state: &A,
    cache_key: &str,
    value: u64,
    expiry: i64,
) -> RouterResult<()>
where
    A: SessionStateInfo + Sync,
{
    let redis_conn = get_redis_connection(state)?;

    redis_conn
        .set_key_with_expiry(cache_key, value, expiry)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
}

pub async fn get_blocked_count_from_cache<A>(
    state: &A, 
    cache_key: &str
) -> RouterResult<Option<u64>>
where
    A: SessionStateInfo + Sync,
{
    let redis_conn = get_redis_connection(state)?;

    let value: Option<u64> = redis_conn
        .get_key(cache_key)
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    Ok(value)
}

pub async fn increment_blocked_count_in_cache<A>(
    state: &A, 
    cache_key: &str,
    expiry: i64,
) -> RouterResult<()>
where
    A: SessionStateInfo + Sync,
{
    let redis_conn = get_redis_connection(state)?;

    let value: Option<u64> = redis_conn
        .get_key(cache_key)
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    let mut incremented_blocked_count: u64 = 1;

    if let Some(actual_value) = value {
        incremented_blocked_count = actual_value + 1;
    } 

    redis_conn
        .set_key_with_expiry(cache_key, incremented_blocked_count, expiry)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
}



