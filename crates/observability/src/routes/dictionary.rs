//! Authenticated alert dictionary handlers.

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::dictionary::DictionaryUpsertRequest;

use crate::{auth, core, services, state::AppState};

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::dictionary::list,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<DictionaryUpsertRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::dictionary::upsert,
        &auth::InternalApiKeyAuth,
    )
    .await
}
