use api_models::oidc::OidcTokenError;
use common_utils::ext_traits::StringExt;
use error_stack::{report, ResultExt};
use url::Url;

use crate::{
    consts::oidc::{AUTH_CODE_TTL_IN_SECS, REDIS_AUTH_CODE_PREFIX},
    core::errors::ApiErrorResponse,
    routes::app::SessionState,
    types::domain::user::oidc::AuthCodeData,
    utils::user as user_utils,
};

fn get_auth_code_key(auth_code: &str) -> String {
    format!("{REDIS_AUTH_CODE_PREFIX}{auth_code}")
}

pub fn validate_client_id_match(registered_client_id: &str, provided_client_id: &str) -> bool {
    registered_client_id.trim() == provided_client_id.trim()
}

pub fn validate_redirect_uri_match(registered_uri: &str, provided_uri: &str) -> bool {
    let registered = registered_uri.trim();
    let provided = provided_uri.trim();

    match (Url::parse(registered), Url::parse(provided)) {
        (Ok(registered_url), Ok(provided_url)) => registered_url == provided_url,
        _ => registered == provided,
    }
}

pub fn build_oidc_redirect_url(
    redirect_uri: &str,
    auth_code: &str,
    state: &str,
) -> error_stack::Result<String, ApiErrorResponse> {
    let base_url = Url::parse(redirect_uri).map_err(|_| {
        report!(ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid redirect_uri in OIDC authorize request")
    })?;

    let url = Url::parse_with_params(base_url.as_str(), &[("code", auth_code), ("state", state)])
        .map_err(|_| {
        report!(ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to build redirect URL with query parameters")
    })?;

    Ok(url.to_string())
}

pub async fn set_auth_code_in_redis(
    state: &SessionState,
    auth_code: &str,
    auth_code_data: &AuthCodeData,
) -> error_stack::Result<(), ApiErrorResponse> {
    let connection = user_utils::get_redis_connection_for_global_tenant(state)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let key = get_auth_code_key(auth_code);
    connection
        .serialize_and_set_key_with_expiry(&key.into(), auth_code_data, AUTH_CODE_TTL_IN_SECS)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to set authorization code in redis")
}

pub async fn get_auth_code_from_redis(
    state: &SessionState,
    code: &str,
) -> error_stack::Result<AuthCodeData, ApiErrorResponse> {
    let connection = user_utils::get_redis_connection_for_global_tenant(state)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let key = get_auth_code_key(code);
    let auth_code_data_string = connection
        .get_key::<Option<String>>(&key.into())
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get authorization code from redis")?
        .ok_or_else(|| ApiErrorResponse::OidcTokenError {
            error: OidcTokenError::InvalidGrant,
            description: "Invalid or expired authorization code".into(),
        })?;

    auth_code_data_string
        .parse_struct("AuthCodeData")
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to deserialize authorization code data")
}

pub async fn delete_auth_code_from_redis(
    state: &SessionState,
    code: &str,
) -> error_stack::Result<(), ApiErrorResponse> {
    let connection = user_utils::get_redis_connection_for_global_tenant(state)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;
    let key = get_auth_code_key(code);
    connection
        .delete_key(&key.into())
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete authorization code from redis")
        .map(|_| ())
}
