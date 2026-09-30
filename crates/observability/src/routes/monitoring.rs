//! Monitoring HTTP adapters using the shared request/auth/error pipeline.

#[cfg(test)]
mod tests;

use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use api_models::observability::monitoring::GrafanaAuthRequest;

use common_utils::errors::ErrorSwitch;

use crate::{auth, core::monitoring, errors::types::ApiErrorResponse, services, state::AppState};

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

pub async fn session(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    let token = match auth::get_jwt_from_authorization_header(request.headers()) {
        Ok(token) => token,
        Err(error) => {
            return ErrorSwitch::<ApiErrorResponse>::switch(error.current_context())
                .error_response();
        }
    };
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (id.into_inner(), token),
        monitoring::session,
        &auth::NoAuth,
    )
    .await
}
