//! Monitoring HTTP adapters using the shared request/auth/error pipeline.

#[cfg(test)]
mod tests;

use actix_web::{
    cookie::{Cookie, SameSite},
    http::header,
    web, HttpRequest, HttpResponse, ResponseError,
};
use api_models::observability::monitoring::GrafanaAuthRequest;
use common_utils::errors::ErrorSwitch;
use hyperswitch_masking::{PeekInterface, Secret};

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
    no_store(
        services::server_wrap(
            state.get_ref().clone(),
            &request,
            payload.into_inner(),
            monitoring::authorize,
            // Authorization happens asynchronously in core via Router, not via an internal API key.
            &auth::NoAuth,
        )
        .await,
    )
}

pub async fn session(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    no_store(
        services::server_wrap_with_response(
            state.get_ref().clone(),
            &request,
            auth::get_jwt_from_authorization_header(request.headers()),
            monitoring::session,
            &auth::NoAuth,
            session_response,
        )
        .await,
    )
}

fn session_response(token: Secret<String>) -> HttpResponse {
    HttpResponse::NoContent()
        .cookie(
            Cookie::build("grafana_token", token.peek().clone())
                .path("/api/observability-plane/grafana")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Strict)
                .finish(),
        )
        .finish()
}

fn no_store(mut response: HttpResponse) -> HttpResponse {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}

pub fn json_config() -> web::JsonConfig {
    web::JsonConfig::default()
        .limit(8192 + 128)
        .error_handler(|_, _| {
            actix_web::error::InternalError::from_response(
                "Invalid credential request",
                no_store(
                    ErrorSwitch::<ApiErrorResponse>::switch(&ObservabilityError::InvalidSession)
                        .error_response(),
                ),
            )
            .into()
        })
}
