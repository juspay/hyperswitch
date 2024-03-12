use std::sync::Arc;

#[cfg(feature = "olap")]
use common_utils::date_time;
use error_stack::{IntoReport, ResultExt};
use redis_interface::RedisConnectionPool;

use super::{AuthToken, UserAuthToken};
#[cfg(feature = "email")]
use crate::consts::{EMAIL_TOKEN_BLACKLIST_PREFIX, EMAIL_TOKEN_TIME_IN_SECS};
use crate::{
    consts::{JWT_TOKEN_TIME_IN_SECS, ROLE_BLACKLIST_PREFIX, USER_BLACKLIST_PREFIX},
    core::errors::{ApiErrorResponse, RouterResult},
    routes::app::AppStateInfo,
};
#[cfg(feature = "olap")]
use crate::{
    core::errors::{UserErrors, UserResult},
    routes::AppState,
    services::authorization as authz,
};

#[cfg(feature = "olap")]
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

#[cfg(feature = "olap")]
pub async fn insert_role_in_blacklist(state: &AppState, role_id: &str) -> UserResult<()> {
    let role_blacklist_key = format!("{}{}", ROLE_BLACKLIST_PREFIX, role_id);
    let expiry =
        expiry_to_i64(JWT_TOKEN_TIME_IN_SECS).change_context(UserErrors::InternalServerError)?;
    let redis_conn = get_redis_connection(state).change_context(UserErrors::InternalServerError)?;
    redis_conn
        .set_key_with_expiry(
            role_blacklist_key.as_str(),
            date_time::now_unix_timestamp(),
            expiry,
        )
        .await
        .change_context(UserErrors::InternalServerError)?;
    invalidate_role_cache(state, role_id)
        .await
        .change_context(UserErrors::InternalServerError)
}

#[cfg(feature = "olap")]
async fn invalidate_role_cache(state: &AppState, role_id: &str) -> RouterResult<()> {
    let redis_conn = get_redis_connection(state)?;
    redis_conn
        .delete_key(authz::get_cache_key_from_role_id(role_id).as_str())
        .await
        .map(|_| ())
        .change_context(ApiErrorResponse::InternalServerError)
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

pub async fn check_role_in_blacklist<A: AppStateInfo>(
    state: &A,
    role_id: &str,
    token_expiry: u64,
) -> RouterResult<bool> {
    let token = format!("{}{}", ROLE_BLACKLIST_PREFIX, role_id);
    let token_issued_at = expiry_to_i64(token_expiry - JWT_TOKEN_TIME_IN_SECS)?;
    let redis_conn = get_redis_connection(state)?;
    redis_conn
        .get_key::<Option<i64>>(token.as_str())
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .map(|timestamp| timestamp.map_or(false, |timestamp| timestamp > token_issued_at))
}

#[cfg(feature = "email")]
pub async fn insert_email_token_in_blacklist(state: &AppState, token: &str) -> UserResult<()> {
    let redis_conn = get_redis_connection(state).change_context(UserErrors::InternalServerError)?;
    let blacklist_key = format!("{}{token}", EMAIL_TOKEN_BLACKLIST_PREFIX);
    let expiry =
        expiry_to_i64(EMAIL_TOKEN_TIME_IN_SECS).change_context(UserErrors::InternalServerError)?;
    redis_conn
        .set_key_with_expiry(blacklist_key.as_str(), true, expiry)
        .await
        .change_context(UserErrors::InternalServerError)
}

#[cfg(feature = "email")]
pub async fn check_email_token_in_blacklist(state: &AppState, token: &str) -> UserResult<()> {
    let redis_conn = get_redis_connection(state).change_context(UserErrors::InternalServerError)?;
    let blacklist_key = format!("{}{token}", EMAIL_TOKEN_BLACKLIST_PREFIX);
    let key_exists = redis_conn
        .exists::<()>(blacklist_key.as_str())
        .await
        .change_context(UserErrors::InternalServerError)?;

    if key_exists {
        return Err(UserErrors::LinkInvalid.into());
    }
    Ok(())
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

#[async_trait::async_trait]
pub trait BlackList {
    async fn check_in_blacklist<A>(&self, state: &A) -> RouterResult<bool>
    where
        A: AppStateInfo + Sync;
}

#[async_trait::async_trait]
impl BlackList for AuthToken {
    async fn check_in_blacklist<A>(&self, state: &A) -> RouterResult<bool>
    where
        A: AppStateInfo + Sync,
    {
        Ok(
            check_user_in_blacklist(state, &self.user_id, self.exp).await?
                || check_role_in_blacklist(state, &self.role_id, self.exp).await?,
        )
    }
}

#[async_trait::async_trait]
impl BlackList for UserAuthToken {
    async fn check_in_blacklist<A>(&self, state: &A) -> RouterResult<bool>
    where
        A: AppStateInfo + Sync,
    {
        check_user_in_blacklist(state, &self.user_id, self.exp).await
    }
}
