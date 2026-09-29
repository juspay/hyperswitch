//! HTTP transport for the Grafana gateway authorization decision.

#[cfg(test)]
mod tests;

use actix_web::{
    cookie::{Cookie, SameSite},
    http::{header, StatusCode},
    web, HttpRequest, HttpResponse,
};
use api_models::observability::monitoring::GrafanaAuthRequest;
use hyperswitch_masking::{PeekInterface, Secret};

use crate::{
    core::monitoring::{self, AuthFailure},
    state::AppState,
};

fn deny(status: StatusCode) -> HttpResponse {
    HttpResponse::build(status)
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .finish()
}

pub async fn authenticate(
    state: web::Data<AppState>,
    request: web::Json<GrafanaAuthRequest>,
) -> HttpResponse {
    match monitoring::authorize(&state, request.into_inner().token).await {
        Ok(response) => HttpResponse::Ok()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .json(response),
        Err(AuthFailure::InvalidCredential) => deny(StatusCode::UNAUTHORIZED),
        Err(AuthFailure::PermissionDenied) => deny(StatusCode::FORBIDDEN),
        Err(AuthFailure::RouterUnavailable) => deny(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// Establish a host-only cookie for the public Grafana proxy prefix, not the auth endpoints.
pub async fn session(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    session_with_client(state.router_transport.as_deref(), &request).await
}

async fn session_with_client(
    client: Option<&crate::core::router_client::RouterClient>,
    request: &HttpRequest,
) -> HttpResponse {
    let mut headers = request.headers().get_all(header::AUTHORIZATION);
    let token = headers
        .next()
        .and_then(|v| v.to_str().ok())
        .and_then(|value| {
            let (scheme, token) = value.split_once(' ')?;
            scheme.eq_ignore_ascii_case("Bearer").then_some(token)
        });
    let Some(token) = token.filter(|_| headers.next().is_none()) else {
        return deny(StatusCode::UNAUTHORIZED);
    };
    let token = Secret::new(token.to_owned());
    match monitoring::authorize_with_client(client, &token).await {
        Ok(_) => HttpResponse::NoContent()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .cookie(
                Cookie::build("grafana_token", token.peek().clone())
                    .path("/api/observability-plane/grafana")
                    .secure(true)
                    .http_only(true)
                    .same_site(SameSite::Strict)
                    .finish(),
            )
            .finish(),
        Err(AuthFailure::InvalidCredential) => deny(StatusCode::UNAUTHORIZED),
        Err(AuthFailure::PermissionDenied) => deny(StatusCode::FORBIDDEN),
        Err(AuthFailure::RouterUnavailable) => deny(StatusCode::SERVICE_UNAVAILABLE),
    }
}

pub fn json_config() -> web::JsonConfig {
    web::JsonConfig::default()
        .limit(8192 + 128)
        .error_handler(|_, _| {
            actix_web::error::InternalError::from_response(
                "Invalid credential request",
                deny(StatusCode::UNAUTHORIZED),
            )
            .into()
        })
}
