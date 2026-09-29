//! Monitoring policy shared by the identity and session endpoints.

use super::response::ApplicationResponse;
use actix_web::{
    cookie::{Cookie, SameSite},
    http::header::{HeaderMap, HeaderValue, SET_COOKIE},
};
use api_models::observability::monitoring::{GrafanaAuthRequest, GrafanaAuthResponse};
use error_stack::report;
use hyperswitch_masking::{PeekInterface, Secret};

use crate::{
    core::router_client::{RouterClient, RouterError},
    domain::monitoring::GrafanaLogin,
    errors::ObservabilityError,
    state::AppState,
};

const GRAFANA_PERMISSION: &str = "ProfileReconRuleRead";
const MAX_TOKEN_BYTES: usize = 8192;

impl From<RouterError> for ObservabilityError {
    fn from(error: RouterError) -> Self {
        match error {
            RouterError::InvalidCredential => Self::InvalidSession,
            RouterError::PermissionDenied => Self::MonitoringForbidden,
            RouterError::Unavailable => Self::RouterUnavailable,
        }
    }
}

pub async fn authorize(
    state: AppState,
    request: GrafanaAuthRequest,
) -> error_stack::Result<GrafanaAuthResponse, ObservabilityError> {
    authorize_with_client(state.router_transport.as_deref(), &request.token).await
}

/// Successful bootstrap sets a cookie only after Router validation; cache policy belongs to the gateway.
pub async fn session(
    state: AppState,
    token: error_stack::Result<Secret<String>, ObservabilityError>,
) -> error_stack::Result<ApplicationResponse, ObservabilityError> {
    let token = token?;
    authorize_with_client(state.router_transport.as_deref(), &token).await?;
    let cookie = Cookie::build("grafana_token", token.peek().clone())
        .path("/api/observability-plane/grafana")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict)
        .finish();
    let mut value = HeaderValue::from_str(&cookie.to_string())
        .map_err(|_| report!(ObservabilityError::InternalServerError))?;
    value.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, value);
    Ok(ApplicationResponse::NoContentWithHeaders { headers })
}

pub(crate) async fn authorize_with_client(
    client: Option<&RouterClient>,
    token: &Secret<String>,
) -> error_stack::Result<GrafanaAuthResponse, ObservabilityError> {
    if !valid_token_shape(token.peek()) {
        return Err(report!(ObservabilityError::InvalidSession));
    }
    let client = client.ok_or_else(|| report!(ObservabilityError::RouterUnavailable))?;
    client
        .authorize_token(token, GRAFANA_PERMISSION)
        .await
        .map_err(ObservabilityError::from)?;
    let email = client
        .get_user_email(token)
        .await
        .map_err(ObservabilityError::from)?;
    let login = GrafanaLogin::from_router_email(&email)
        .ok_or_else(|| report!(ObservabilityError::RouterUnavailable))?;
    Ok(GrafanaAuthResponse {
        grafana_login: login.into_string(),
    })
}

fn valid_token_shape(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= MAX_TOKEN_BYTES
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_token_shapes() {
        assert!(!valid_token_shape(""));
        assert!(!valid_token_shape(&"a".repeat(MAX_TOKEN_BYTES + 1)));
        assert!(!valid_token_shape("token\r\nAuthorization: evil"));
        assert!(!valid_token_shape("cookie; extra=value"));
        assert!(valid_token_shape("a.b_c-123.sig"));
    }
}
