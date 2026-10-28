#[cfg(feature = "olap")]
use common_utils::date_time;
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    consts::USER_IDLE_SESSION_PREFIX,
    core::errors::{ApiErrorResponse, RouterResult},
    routes::SessionState,
};
#[cfg(feature = "olap")]
use crate::{
    core::errors::{UserErrors, UserResult},
    utils::user::{end_user_sessions, get_redis_connection_for_global_tenant},
};

fn get_idle_session_key(user_id: &str) -> String {
    format!("{USER_IDLE_SESSION_PREFIX}{user_id}")
}

/// Starts the idle timer of a user who has just signed in.
#[cfg(feature = "olap")]
pub async fn start_idle_session(
    state: &SessionState,
    user_id: &str,
    idle_timeout_in_secs: u32,
) -> UserResult<()> {
    get_redis_connection_for_global_tenant(state)?
        .set_key_with_expiry(
            &get_idle_session_key(user_id).as_str().into(),
            date_time::now_unix_timestamp(),
            i64::from(idle_timeout_in_secs),
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

/// Rejects a Dashboard login token once its user's idle timer has run out,
/// signing the user out the way signout does, and restarts the timer otherwise.
pub async fn check_idle_session(state: &SessionState, user_id: &str) -> RouterResult<()> {
    let Some(idle_timeout_in_secs) = state.conf.user.idle_session_timeout_in_secs else {
        return Ok(());
    };

    let redis_conn = state
        .global_store
        .get_redis_conn()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let key = get_idle_session_key(user_id);

    let is_session_active = redis_conn
        .exists::<()>(&key.as_str().into())
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to check the dashboard idle session")?;

    if !is_session_active {
        logger::info!(user_id, "Dashboard session expired due to inactivity");
        #[cfg(feature = "olap")]
        {
            let _ = end_user_sessions(state, user_id)
                .await
                .inspect_err(|error| logger::error!(?error, "Failed to end expired sessions"));
        }
        return Err(ApiErrorResponse::ExpiredJwtToken.into());
    }

    redis_conn
        .set_expiry(&key.as_str().into(), i64::from(idle_timeout_in_secs))
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to extend the dashboard idle session")
}
