//! Gateway-facing Grafana authorization. No browser-supplied identity or permission is accepted.

use actix_web::{http::header, web, HttpResponse};
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use hyperswitch_masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    core::router_client::{RouterCallState, RouterClient},
    state::AppState,
};

const MAX_TOKEN_BYTES: usize = 8192;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthRequest {
    token: Secret<String>,
}

#[derive(Serialize)]
struct AuthResponse {
    grafana_login: String,
}

fn deny(status: actix_web::http::StatusCode) -> HttpResponse {
    HttpResponse::build(status)
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .finish()
}

fn valid_token_shape(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= MAX_TOKEN_BYTES
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn grafana_login(email: &str) -> String {
    format!("cc_{:x}", Sha256::digest(email.as_bytes()))
}

fn map_router_error(error: &MicroserviceClientError) -> actix_web::http::StatusCode {
    use actix_web::http::StatusCode;
    match &error.kind {
        MicroserviceClientErrorKind::Upstream { status: 401, .. } => StatusCode::UNAUTHORIZED,
        MicroserviceClientErrorKind::Upstream { status: 403, .. }
            if error.operation.contains("AuthorizeFlow") =>
        {
            StatusCode::FORBIDDEN
        }
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

pub async fn authenticate(
    state: web::Data<AppState>,
    transport: web::Data<RouterCallState>,
    request: web::Json<AuthRequest>,
) -> HttpResponse {
    let token = &request.token;
    let value = token.peek();
    // A JWT must fit in HTTP headers and contain no control characters, whitespace or cookie
    // delimiters. This is a shape check only; Router authenticates and authorizes it.
    if !valid_token_shape(value) {
        return deny(actix_web::http::StatusCode::UNAUTHORIZED);
    }
    let Some(config) = &state.conf.router else {
        return deny(actix_web::http::StatusCode::SERVICE_UNAVAILABLE);
    };
    let client = match RouterClient::new(&config.base_url, token) {
        Ok(client) => client,
        Err(_) => return deny(actix_web::http::StatusCode::SERVICE_UNAVAILABLE),
    };
    match client
        .authorize_and_get_email(&transport, token.clone())
        .await
    {
        Ok(email) => {
            let grafana_login = grafana_login(&email);
            HttpResponse::Ok()
                .insert_header((header::CACHE_CONTROL, "no-store"))
                .json(AuthResponse { grafana_login })
        }
        Err(error) => deny(map_router_error(&error)),
    }
}

pub fn json_config() -> web::JsonConfig {
    web::JsonConfig::default()
        .limit(MAX_TOKEN_BYTES + 128)
        .error_handler(|_, _| {
            actix_web::error::InternalError::from_response(
                "Invalid credential request",
                deny(actix_web::http::StatusCode::UNAUTHORIZED),
            )
            .into()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_missing_oversized_or_header_injecting_tokens() {
        assert!(!valid_token_shape(""));
        assert!(!valid_token_shape(&"a".repeat(MAX_TOKEN_BYTES + 1)));
        assert!(!valid_token_shape("token\r\nAuthorization: evil"));
        assert!(!valid_token_shape("cookie; extra=value"));
        assert!(valid_token_shape("a.b_c-123.sig"));
    }

    #[test]
    fn login_is_namespaced_and_derived_from_exact_router_email() {
        assert_eq!(
            grafana_login("user@example.com"),
            grafana_login("user@example.com")
        );
        assert_ne!(
            grafana_login("User@example.com"),
            grafana_login("user@example.com")
        );
        assert_eq!(grafana_login("user@example.com").len(), 67);
        assert!(!grafana_login("user@example.com").contains('@'));
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
            actix_web::http::StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            map_router_error(&error(403)),
            actix_web::http::StatusCode::FORBIDDEN
        );
        assert_eq!(
            map_router_error(&error(500)),
            actix_web::http::StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
