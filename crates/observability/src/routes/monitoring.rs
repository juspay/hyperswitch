//! Monitoring HTTP adapters using the shared request/auth/error pipeline.

#[cfg(test)]
mod tests;

use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use api_models::observability::monitoring::GrafanaAuthRequest;
use common_utils::errors::ErrorSwitch;

use crate::{
    auth,
    core::monitoring,
    errors::{types::ApiErrorResponse, ObservabilityError},
    services,
    state::AppState,
};

pub async fn authenticate(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<GrafanaAuthRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        monitoring::authorize,
        // Authorization happens asynchronously in core via Router, not via an internal API key.
        &auth::NoAuth,
    )
    .await
}

pub async fn session(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        auth::get_jwt_from_authorization_header(request.headers()),
        monitoring::session,
        &auth::NoAuth,
    )
    .await
}

pub fn json_config() -> web::JsonConfig {
    web::JsonConfig::default()
        .limit(8192 + 128)
        .error_handler(|_, _| {
            actix_web::error::InternalError::from_response(
                "Invalid credential request",
                ErrorSwitch::<ApiErrorResponse>::switch(&ObservabilityError::InvalidSession)
                    .error_response(),
            )
            .into()
        })
}
