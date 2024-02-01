use std::sync::Arc;

#[cfg(feature = "olap")]
use common_utils::date_time;
use error_stack::{IntoReport, ResultExt};
use redis_interface::RedisConnectionPool;

use crate::{
    consts::{JWT_TOKEN_TIME_IN_SECS, USER_BLACKLIST_PREFIX},
    core::errors::{ApiErrorResponse, RouterResult},
    routes::app::AppStateInfo,
};
#[cfg(feature = "olap")]
use crate::{
    core::errors::{UserErrors, UserResult},
    routes::AppState,
};

#[cfg(feature = "olap")]
/// Inserts a user ID into the blacklist with an expiry time based on the JWT token time in seconds.
pub async fn insert_user_in_blacklist(state: &AppState, user_id: &str) -> UserResult<()> {
    let user_blacklist_key = format!("{}{}", USER_BLACKLIST_PREFIX, user_id);
    let expiry =
        expiry_to_i64(JWT_TOKEN_TIME_IN_SECS).change_context(UserErrors::InternalServerError)?;
    let redis_conn = get_redis_connection(state).change_context(UserErrors::InternalServerError)?;
    redis_conn
        .set_key_with_expiry(
            user_blacklist_key.as_str(),
            date_time::now_unix_timestamp(),
            expiry,
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

/// Asynchronously checks if a user is in the blacklist by retrieving the token of the user from Redis, 
/// comparing its timestamp with the token expiry and returning a boolean value based on the comparison result.
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

/// Retrieves a Redis connection from the application state and returns it as an `Arc` wrapped in a `RouterResult`.
fn get_redis_connection<A: AppStateInfo>(state: &A) -> RouterResult<Arc<RedisConnectionPool>> {
    state
        .store()
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")
}

/// Converts the given `expiry` value of type `u64` to an `i64` value and returns a `RouterResult` with the converted value.
///
/// # Arguments
///
/// * `expiry` - A value of type `u64` representing the expiry time.
///
/// # Returns
///
/// A `RouterResult` containing the converted `i64` value. If the conversion is successful, the `RouterResult` will contain the converted value. If an error occurs during the conversion, an `ApiErrorResponse` with the status code `InternalServerError` will be returned.
fn expiry_to_i64(expiry: u64) -> RouterResult<i64> {
    expiry
        .try_into()
        .into_report()
        .change_context(ApiErrorResponse::InternalServerError)
}
