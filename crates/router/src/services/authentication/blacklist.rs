use std::sync::Arc;

use error_stack::{IntoReport, ResultExt};
use redis_interface::RedisConnectionPool;

#[cfg(feature = "olap")]
use crate::core::errors::{UserErrors, UserResult};
#[cfg(feature = "olap")]
use common_utils::date_time;
#[cfg(feature = "olap")]
use crate::routes::AppState;
use crate::{
    consts::{JWT_TOKEN_TIME_IN_SECS, USER_BLACKLIST_PREFIX},
    core::errors::{ApiErrorResponse, RouterResult},
    routes::app::AppStateInfo,
};

#[cfg(feature = "olap")]
pub async fn insert_user_in_blacklist(state: &AppState, user_id: &str) -> UserResult<()> {
    let token = format!("{}{}", USER_BLACKLIST_PREFIX, user_id);
    let expiry =
        expiry_to_i64(JWT_TOKEN_TIME_IN_SECS).change_context(UserErrors::InternalServerError)?;
    let redis_conn = get_redis_connection(state).change_context(UserErrors::InternalServerError)?;
    redis_conn
        .set_key_with_expiry(token.as_str(), date_time::now_unix_timestamp(), expiry)
        .await
        .change_context(UserErrors::InternalServerError)
}

pub async fn check_user_in_blacklist<A: AppStateInfo>(
    state: &A,
    user_id: &str,
    token_expiry: u64,
) -> RouterResult<bool> {
    let token = format!("{}{}", USER_BLACKLIST_PREFIX, user_id);
    let token_issued_at = expiry_to_i64(token_expiry - JWT_TOKEN_TIME_IN_SECS)?;
    let redis_conn = get_redis_connection(state)?;
    redis_conn
        .get_key::<Option<i64>>(token.as_str())
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .map(|timestamp| timestamp.map_or(false, |timestamp| timestamp > token_issued_at))
}

fn get_redis_connection<A: AppStateInfo>(state: &A) -> RouterResult<Arc<RedisConnectionPool>> {
    state
        .store()
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")
}

fn expiry_to_i64(expiry: u64) -> RouterResult<i64> {
    expiry
        .try_into()
        .into_report()
        .change_context(ApiErrorResponse::InternalServerError)
}
