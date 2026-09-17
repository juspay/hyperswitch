//! Authenticated per-alert metadata and snooze handlers.

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::metadata::AlertMetadataPatchRequest;

use crate::{auth, core, services, state::AppState};

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::metadata::list,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn patch(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
    payload: web::Json<AlertMetadataPatchRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (id.into_inner(), payload.into_inner()),
        core::metadata::patch,
        &auth::InternalApiKeyAuth,
    )
    .await
}
