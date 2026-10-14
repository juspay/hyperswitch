#[cfg(feature = "olap")]
use common_utils::date_time;
use error_stack::ResultExt;
use router_env::{logger, types::FlowMetric, Flow};

use super::{blacklist::get_redis_connection_for_global_tenant, AuthenticationType};
#[cfg(feature = "olap")]
use crate::core::errors::{UserErrors, UserResult};
use crate::{
    consts::USER_IDLE_SESSION_PREFIX,
    core::errors::{ApiErrorResponse, RouterResult},
    routes::SessionState,
};

fn get_idle_session_key(user_id: &str) -> String {
    format!("{USER_IDLE_SESSION_PREFIX}{user_id}")
}

/// Starts the idle timer of a user who has just signed in.
#[cfg(feature = "olap")]
pub async fn start_idle_session(
    state: &SessionState,
    user_id: &str,
    idle_timeout_in_secs: i64,
) -> UserResult<()> {
    get_redis_connection_for_global_tenant(state)
        .change_context(UserErrors::InternalServerError)?
        .set_key_with_expiry(
            &get_idle_session_key(user_id).as_str().into(),
            date_time::now_unix_timestamp(),
            idle_timeout_in_secs,
        )
        .await
        .change_context(UserErrors::InternalServerError)
}

/// Rejects a request made with a Dashboard login token once its user's idle
/// timer has run out, and restarts the timer otherwise.
pub async fn check_idle_session(
    state: &SessionState,
    auth_type: &AuthenticationType,
    flow: &impl FlowMetric,
) -> RouterResult<()> {
    let Some(idle_timeout_in_secs) = state.conf.user.idle_session_timeout_in_secs else {
        return Ok(());
    };
    let Some(user_id) = get_dashboard_session_user_id(auth_type, flow) else {
        return Ok(());
    };

    let redis_conn = get_redis_connection_for_global_tenant(state)?;
    let key = get_idle_session_key(user_id);

    let is_session_active = redis_conn
        .exists::<()>(&key.as_str().into())
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to check the dashboard idle session")?;

    if !is_session_active {
        logger::info!(user_id, "Dashboard session expired due to inactivity");
        return Err(ApiErrorResponse::ExpiredJwtToken.into());
    }

    redis_conn
        .set_expiry(&key.as_str().into(), idle_timeout_in_secs)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to extend the dashboard idle session")
}

/// Returns the user whose idle timer applies to a request, or `None` when the
/// request wasn't made with a Dashboard login token.
fn get_dashboard_session_user_id<'a>(
    auth_type: &'a AuthenticationType,
    flow: &impl FlowMetric,
) -> Option<&'a str> {
    match auth_type {
        AuthenticationType::OrganizationJwt { user_id, .. }
        | AuthenticationType::MerchantJwtWithProfileId { user_id, .. }
        | AuthenticationType::UserJwt { user_id }
        | AuthenticationType::MerchantJwt {
            user_id: Some(user_id),
            ..
        } => Some(user_id.as_str()),

        AuthenticationType::SinglePurposeOrLoginJwt {
            user_id,
            role_id: Some(_),
            ..
        } if flow.to_string() != Flow::Signout.to_string() => Some(user_id.as_str()),

        AuthenticationType::MerchantJwt { user_id: None, .. }
        | AuthenticationType::SinglePurposeOrLoginJwt { .. }
        | AuthenticationType::SinglePurposeJwt { .. }
        | AuthenticationType::EmbeddedJwt { .. }
        | AuthenticationType::ApiKey { .. }
        | AuthenticationType::AdminApiKey
        | AuthenticationType::AdminApiAuthWithMerchantId { .. }
        | AuthenticationType::BasicAuth { .. }
        | AuthenticationType::MerchantId { .. }
        | AuthenticationType::PublishableKey { .. }
        | AuthenticationType::SdkAuthorization { .. }
        | AuthenticationType::WebhookAuth { .. }
        | AuthenticationType::InternalMerchantIdProfileId { .. }
        | AuthenticationType::InternalApiKey
        | AuthenticationType::NoAuth => None,
    }
}
