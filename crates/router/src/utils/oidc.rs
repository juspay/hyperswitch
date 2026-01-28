use common_utils::ext_traits::StringExt;
use error_stack::{report, ResultExt};
use masking::PeekInterface;
use url::Url;

use crate::{
    consts::oidc::{AUTH_CODE_TTL_IN_SECS, REDIS_AUTH_CODE_PREFIX},
    core::errors::oidc::{OidcErrors, OidcResult},
    routes::app::SessionState,
    services::{api::ApplicationResponse, encryption},
    types::domain::user::oidc::AuthCodeData,
    utils::user as user_utils,
};

#[inline]
fn get_auth_code_key(auth_code: &str) -> String {
    format!("{REDIS_AUTH_CODE_PREFIX}{auth_code}")
}

#[inline]
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
) -> OidcResult<String> {
    Url::parse(redirect_uri)
        .map_err(|_| {
            report!(OidcErrors::InvalidRequest)
                .attach_printable("Invalid redirect_uri in OIDC authorize request")
        })
        .and_then(|base_url| {
            Url::parse_with_params(base_url.as_str(), &[("code", auth_code), ("state", state)])
                .map_err(|_| {
                    report!(OidcErrors::ServerError)
                        .attach_printable("Failed to build redirect URL with query parameters")
                })
        })
        .map(|url| url.to_string())
}

pub async fn set_auth_code_in_redis(
    state: &SessionState,
    auth_code: &str,
    auth_code_data: &AuthCodeData,
) -> OidcResult<()> {
    let connection = user_utils::get_redis_connection_for_global_tenant(state)
        .change_context(OidcErrors::ServerError)
        .attach_printable("Failed to get redis connection")?;
    let key = get_auth_code_key(auth_code);
    connection
        .serialize_and_set_key_with_expiry(&key.into(), auth_code_data, AUTH_CODE_TTL_IN_SECS)
        .await
        .change_context(OidcErrors::ServerError)
        .attach_printable("Failed to set authorization code in redis")
}

pub async fn get_auth_code_from_redis(
    state: &SessionState,
    code: &str,
) -> OidcResult<AuthCodeData> {
    let connection = user_utils::get_redis_connection_for_global_tenant(state)
        .change_context(OidcErrors::ServerError)
        .attach_printable("Failed to get redis connection")?;
    let key = get_auth_code_key(code);
    let auth_code_data_string = connection
        .get_key::<Option<String>>(&key.into())
        .await
        .change_context(OidcErrors::ServerError)
        .attach_printable("Failed to get authorization code from redis")?
        .ok_or_else(|| report!(OidcErrors::InvalidGrant))?;

    auth_code_data_string
        .parse_struct("AuthCodeData")
        .change_context(OidcErrors::ServerError)
        .attach_printable("Failed to deserialize authorization code data")
}

pub async fn delete_auth_code_from_redis(state: &SessionState, code: &str) -> OidcResult<()> {
    let connection = user_utils::get_redis_connection_for_global_tenant(state)
        .change_context(OidcErrors::ServerError)
        .attach_printable("Failed to get redis connection")?;
    let key = get_auth_code_key(code);
    connection
        .delete_key(&key.into())
        .await
        .change_context(OidcErrors::ServerError)
        .attach_printable("Failed to delete authorization code from redis")
        .map(|_| ())
}

#[cfg(feature = "v1")]
pub fn build_redirection_response(redirect_url: String) -> ApplicationResponse<()> {
    use api_models::payments::RedirectionResponse;
    ApplicationResponse::JsonForRedirection(RedirectionResponse {
        headers: Vec::new(),
        return_url: String::new(),
        http_method: String::new(),
        params: Vec::new(),
        return_url_with_query_params: redirect_url,
    })
}

#[cfg(feature = "v2")]
pub fn build_redirection_response(redirect_url: String) -> ApplicationResponse<()> {
    use api_models::payments::RedirectionResponse;
    ApplicationResponse::JsonForRedirection(RedirectionResponse {
        return_url_with_query_params: redirect_url,
    })
}

/// Sign OIDC JWT tokens with RS256
pub async fn sign_oidc_token<T>(
    state: &SessionState,
    claims: T,
    error_prefix: &str,
) -> OidcResult<String>
where
    T: serde::Serialize,
{
    let signing_key = state
        .conf
        .oidc
        .get_inner()
        .get_signing_key()
        .ok_or_else(|| {
            report!(OidcErrors::ServerError).attach_printable("No signing key configured for OIDC")
        })?;

    let payload_bytes = serde_json::to_vec(&claims)
        .change_context(OidcErrors::ServerError)
        .attach_printable(format!("Failed to serialize {} claims", error_prefix))?;

    encryption::jws_sign_payload(
        &payload_bytes,
        &signing_key.kid,
        signing_key.private_key.peek().as_bytes(),
    )
    .await
    .change_context(OidcErrors::ServerError)
    .attach_printable(format!("Failed to sign {}", error_prefix))
}
