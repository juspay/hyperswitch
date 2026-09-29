//! Monitoring policy shared by the identity and session endpoints.

use api_models::observability::monitoring::GrafanaAuthResponse;
use hyperswitch_masking::{PeekInterface, Secret};

use crate::{
    core::router_client::{RouterClient, RouterError},
    domain::monitoring::GrafanaLogin,
    state::AppState,
};

const GRAFANA_PERMISSION: &str = "ProfileReconRuleRead";
const MAX_TOKEN_BYTES: usize = 8192;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthFailure {
    InvalidCredential,
    PermissionDenied,
    RouterUnavailable,
}

impl From<RouterError> for AuthFailure {
    fn from(error: RouterError) -> Self {
        match error {
            RouterError::InvalidCredential => Self::InvalidCredential,
            RouterError::PermissionDenied => Self::PermissionDenied,
            RouterError::Unavailable => Self::RouterUnavailable,
        }
    }
}

pub async fn authorize(
    state: &AppState,
    token: Secret<String>,
) -> Result<GrafanaAuthResponse, AuthFailure> {
    authorize_with_client(state.router_transport.as_deref(), &token).await
}

pub(crate) async fn authorize_with_client(
    client: Option<&RouterClient>,
    token: &Secret<String>,
) -> Result<GrafanaAuthResponse, AuthFailure> {
    if !valid_token_shape(token.peek()) {
        return Err(AuthFailure::InvalidCredential);
    }
    let client = client.ok_or(AuthFailure::RouterUnavailable)?;
    client.authorize_token(token, GRAFANA_PERMISSION).await?;
    let email = client.get_user_email(token).await?;
    let login = GrafanaLogin::from_router_email(&email).ok_or(AuthFailure::RouterUnavailable)?;
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
