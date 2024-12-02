use std::sync::Arc;

use common_utils::id_type;
use error_stack::ResultExt;
use redis_interface::RedisConnectionPool;
use router_env::logger;

use super::authentication::AuthToken;
use crate::{
    consts,
    core::errors::{ApiErrorResponse, RouterResult},
    routes::app::SessionStateInfo,
};

pub fn get_cache_key_from_fingerprint(fingerprint: &str) -> String {
    format!("{}{}", consts::CARD_TESTING_GUARD_PREFIX, fingerprint)
}


fn get_redis_connection<A: SessionStateInfo>(state: &A) -> RouterResult<Arc<RedisConnectionPool>> {
    state
        .store()
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")
}

pub async fn set_blocked_count_in_cache<A>(
    state: &A,
    fingerprint: &str,
    value: u64,
    expiry: i64,
) -> RouterResult<()>
where
    A: SessionStateInfo + Sync,
{
    let redis_conn = get_redis_connection(state)?;

    redis_conn
        .set_key_with_expiry(&get_cache_key_from_fingerprint(fingerprint), value, expiry)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
}

pub async fn get_blocked_count_from_cache<A>(
    state: &A, 
    fingerprint: &str
) -> RouterResult<Option<u64>>
where
    A: SessionStateInfo + Sync,
{
    let redis_conn = get_redis_connection(state)?;

    let value: Option<u64> = redis_conn
        .get_key(&get_cache_key_from_fingerprint(fingerprint))
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    Ok(value)
}

pub async fn increment_blocked_count_in_cache<A>(
    state: &A, 
    fingerprint: &str,
    expiry: i64,
) -> RouterResult<()>
where
    A: SessionStateInfo + Sync,
{
    let redis_conn = get_redis_connection(state)?;

    let value: Option<u64> = redis_conn
        .get_key(&get_cache_key_from_fingerprint(fingerprint))
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    let mut incremented_blocked_count: u64 = 1;

    if let Some(actual_value) = value {
        incremented_blocked_count = actual_value + 1;
    } 

    redis_conn
        .set_key_with_expiry(&get_cache_key_from_fingerprint(fingerprint), incremented_blocked_count, expiry)
        .await
        .change_context(ApiErrorResponse::InternalServerError)

}



