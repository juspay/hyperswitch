use std::sync::Arc;

use common_utils::pii;
use error_stack::ResultExt;
use masking::ExposeInterface;
use redis_interface::RedisConnectionPool;
use totp_rs::{Algorithm, TOTP};

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::AppState,
};

pub fn generate_default_totp(
    email: pii::Email,
    secret: Option<masking::Secret<String>>,
) -> UserResult<TOTP> {
    let secret = secret
        .map(|sec| totp_rs::Secret::Encoded(sec.expose()))
        .unwrap_or_else(totp_rs::Secret::generate_secret)
        .to_bytes()
        .change_context(UserErrors::InternalServerError)?;

    TOTP::new(
        Algorithm::SHA1,
        consts::user::TOTP_DIGITS,
        consts::user::TOTP_TOLERANCE,
        consts::user::TOTP_VALIDITY_DURATION_IN_SECONDS,
        secret,
        Some(consts::user::TOTP_ISSUER_NAME.to_string()),
        email.expose().expose(),
    )
    .change_context(UserErrors::InternalServerError)
}

pub async fn check_totp_in_redis(state: &AppState, user_id: &str) -> UserResult<bool> {
    let redis_conn = get_redis_connection(state)?;
    let key = format!("{}{}", consts::user::TOTP_PREFIX, user_id);
    redis_conn
        .exists::<()>(&key)
        .await
        .change_context(UserErrors::InternalServerError)
}

fn get_redis_connection(state: &AppState) -> UserResult<Arc<RedisConnectionPool>> {
    state
        .store
        .get_redis_conn()
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get redis connection")
}

fn expiry_to_i64(expiry: u64) -> UserResult<i64> {
    i64::try_from(expiry).change_context(UserErrors::InternalServerError)
}

pub async fn insert_recovery_code_in_redis(state: &AppState, user_id: &str) -> UserResult<()> {
    let redis_conn = get_redis_connection(state)?;
    let key = format!("{}{}", consts::user::REDIS_RECOVERY_CODE_PREFIX, user_id);
    let expiry = expiry_to_i64(state.conf.user.two_factor_auth_expiry_in_secs)
        .change_context(UserErrors::InternalServerError)?;
    redis_conn
        .set_key_with_expiry(key.as_str(), true, expiry)
        .await
        .change_context(UserErrors::InternalServerError)
}
