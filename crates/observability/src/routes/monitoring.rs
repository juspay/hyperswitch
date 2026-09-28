//! HTTP transport for the Grafana gateway authorization decision.

use actix_web::{
    http::{header, StatusCode},
    web, HttpResponse,
};
use api_models::observability::monitoring::{GrafanaAuthRequest, GrafanaAuthResponse};

use crate::{
    core::{
        monitoring::{self, AuthFailure},
        router_client::RouterCallState,
    },
    state::AppState,
};

fn deny(status: StatusCode) -> HttpResponse {
    HttpResponse::build(status)
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .finish()
}

pub async fn authenticate(
    state: web::Data<AppState>,
    transport: web::Data<RouterCallState>,
    request: web::Json<GrafanaAuthRequest>,
) -> HttpResponse {
    let Some(router) = &state.conf.router else {
        return deny(StatusCode::SERVICE_UNAVAILABLE);
    };
    match monitoring::authorize(&router.base_url, &transport, request.into_inner().token).await {
        Ok(login) => HttpResponse::Ok()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .json(GrafanaAuthResponse {
                grafana_login: login.into_string(),
            }),
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
