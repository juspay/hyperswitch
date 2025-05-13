use common_utils::pii;
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface};
use totp_rs::{Algorithm, TOTP};

use crate::{
    consts,
    core::errors::{UserErrors, UserResult},
    routes::SessionState,
};

pub fn generate_default_totp(
    email: pii::Email,
    secret: Option<masking::Secret<String>>,
    issuer: String,
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
        Some(issuer),
        email.expose().expose(),
    )
    .change_context(UserErrors::InternalServerError)
}

pub async fn check_totp_in_redis(state: &SessionState, user_id: &str) -> UserResult<bool> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    let key = format!("{}{}", consts::user::REDIS_TOTP_PREFIX, user_id);
    redis_conn
        .exists::<()>(&key.into())
        .await
        .change_context(UserErrors::InternalServerError)
}

pub async fn check_recovery_code_in_redis(state: &SessionState, user_id: &str) -> UserResult<bool> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    let key = format!("{}{}", consts::user::REDIS_RECOVERY_CODE_PREFIX, user_id);
    redis_conn
        .exists::<()>(&key.into())
        .await
        .change_context(UserErrors::InternalServerError)
}

pub async fn insert_totp_in_redis(state: &SessionState, user_id: &str) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    let key = format!("{}{}", consts::user::REDIS_TOTP_PREFIX, user_id);
    redis_conn
        .set_key_with_expiry(
            &key.as_str().into(),
            common_utils::date_time::now_unix_timestamp(),
            state.conf.user.two_factor_auth_expiry_in_secs,
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

pub async fn insert_totp_secret_in_redis(
    state: &SessionState,
    user_id: &str,
    secret: &masking::Secret<String>,
) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .set_key_with_expiry(
            &get_totp_secret_key(user_id).into(),
            secret.peek(),
            consts::user::REDIS_TOTP_SECRET_TTL_IN_SECS,
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

pub async fn get_totp_secret_from_redis(
    state: &SessionState,
    user_id: &str,
) -> UserResult<Option<masking::Secret<String>>> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .get_key::<Option<String>>(&get_totp_secret_key(user_id).into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|secret| secret.map(Into::into))
}

pub async fn delete_totp_secret_from_redis(state: &SessionState, user_id: &str) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .delete_key(&get_totp_secret_key(user_id).into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|_| ())
}

fn get_totp_secret_key(user_id: &str) -> String {
    format!("{}{}", consts::user::REDIS_TOTP_SECRET_PREFIX, user_id)
}

pub async fn insert_recovery_code_in_redis(state: &SessionState, user_id: &str) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    let key = format!("{}{}", consts::user::REDIS_RECOVERY_CODE_PREFIX, user_id);
    redis_conn
        .set_key_with_expiry(
            &key.as_str().into(),
            common_utils::date_time::now_unix_timestamp(),
            state.conf.user.two_factor_auth_expiry_in_secs,
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

pub async fn delete_totp_from_redis(state: &SessionState, user_id: &str) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    let key = format!("{}{}", consts::user::REDIS_TOTP_PREFIX, user_id);
    redis_conn
        .delete_key(&key.into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|_| ())
}

pub async fn delete_recovery_code_from_redis(
    state: &SessionState,
    user_id: &str,
) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    let key = format!("{}{}", consts::user::REDIS_RECOVERY_CODE_PREFIX, user_id);
    redis_conn
        .delete_key(&key.into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|_| ())
}

fn get_totp_attempts_key(user_id: &str) -> String {
    format!("{}{}", consts::user::REDIS_TOTP_ATTEMPTS_PREFIX, user_id)
}
fn get_recovery_code_attempts_key(user_id: &str) -> String {
    format!(
        "{}{}",
        consts::user::REDIS_RECOVERY_CODE_ATTEMPTS_PREFIX,
        user_id
    )
}

pub async fn insert_totp_attempts_in_redis(
    state: &SessionState,
    user_id: &str,
    user_totp_attempts: u8,
) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .set_key_with_expiry(
            &get_totp_attempts_key(user_id).into(),
            user_totp_attempts,
            consts::user::REDIS_TOTP_ATTEMPTS_TTL_IN_SECS,
        )
        .await
        .change_context(UserErrors::InternalServerError)
}
pub async fn get_totp_attempts_from_redis(state: &SessionState, user_id: &str) -> UserResult<u8> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .get_key::<Option<u8>>(&get_totp_attempts_key(user_id).into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|v| v.unwrap_or(0))
}

pub async fn insert_recovery_code_attempts_in_redis(
    state: &SessionState,
    user_id: &str,
    user_recovery_code_attempts: u8,
) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .set_key_with_expiry(
            &get_recovery_code_attempts_key(user_id).into(),
            user_recovery_code_attempts,
            consts::user::REDIS_RECOVERY_CODE_ATTEMPTS_TTL_IN_SECS,
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

pub async fn get_recovery_code_attempts_from_redis(
    state: &SessionState,
    user_id: &str,
) -> UserResult<u8> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .get_key::<Option<u8>>(&get_recovery_code_attempts_key(user_id).into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|v| v.unwrap_or(0))
}

pub async fn delete_totp_attempts_from_redis(
    state: &SessionState,
    user_id: &str,
) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .delete_key(&get_totp_attempts_key(user_id).into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|_| ())
}

pub async fn delete_recovery_code_attempts_from_redis(
    state: &SessionState,
    user_id: &str,
) -> UserResult<()> {
    let redis_conn = super::get_redis_connection_for_global_tenant(state)?;
    redis_conn
        .delete_key(&get_recovery_code_attempts_key(user_id).into())
        .await
        .change_context(UserErrors::InternalServerError)
        .map(|_| ())
}
