//! Monitoring policy shared by the identity and session endpoints.

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

/// Successful bootstrap returns the validated credential, never an unvalidated header value.
pub async fn session(
    state: AppState,
    token: error_stack::Result<Secret<String>, ObservabilityError>,
) -> error_stack::Result<Secret<String>, ObservabilityError> {
    let token = token?;
    authorize_with_client(state.router_transport.as_deref(), &token).await?;
    Ok(token)
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
