//! Decide whether a Control Center credential may become a Grafana login.

use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use hyperswitch_masking::{PeekInterface, Secret};
use url::Url;

use crate::{
    core::router_client::{RouterCallState, RouterClient},
    domain::monitoring::GrafanaLogin,
};

const MAX_TOKEN_BYTES: usize = 8192;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthFailure {
    InvalidCredential,
    PermissionDenied,
    RouterUnavailable,
}

pub async fn authorize(
    base_url: &Url,
    transport: &RouterCallState,
    token: Secret<String>,
) -> Result<GrafanaLogin, AuthFailure> {
    if !valid_token_shape(token.peek()) {
        return Err(AuthFailure::InvalidCredential);
    }
    let client =
        RouterClient::new(base_url.as_str(), &token).map_err(|_| AuthFailure::RouterUnavailable)?;
    let email = client
        .authorize_and_get_email(transport, token)
        .await
        .map_err(|error| map_router_error(&error))?;
    GrafanaLogin::from_router_email(&email).ok_or(AuthFailure::RouterUnavailable)
}

fn valid_token_shape(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= MAX_TOKEN_BYTES
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn map_router_error(error: &MicroserviceClientError) -> AuthFailure {
    match &error.kind {
        MicroserviceClientErrorKind::Upstream { status: 401, .. } => AuthFailure::InvalidCredential,
        MicroserviceClientErrorKind::Upstream { status: 403, .. }
            if error.operation.contains("AuthorizeFlow") =>
        {
            AuthFailure::PermissionDenied
        }
        _ => AuthFailure::RouterUnavailable,
    }
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

    #[test]
    fn router_statuses_fail_closed() {
        let error = |status| MicroserviceClientError {
            operation: "observability::AuthorizeFlow".to_string(),
            kind: MicroserviceClientErrorKind::Upstream {
                status,
                body: String::new(),
            },
        };
        assert_eq!(
            map_router_error(&error(401)),
            AuthFailure::InvalidCredential
        );
        assert_eq!(map_router_error(&error(403)), AuthFailure::PermissionDenied);
        assert_eq!(
            map_router_error(&error(500)),
            AuthFailure::RouterUnavailable
        );
    }
}
