//! HTTP transport for the Grafana gateway authorization decision.

use actix_web::{
    http::{header, StatusCode},
    web, HttpResponse,
};
use api_models::observability::monitoring::GrafanaAuthRequest;

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
