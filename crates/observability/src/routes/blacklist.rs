//! Authenticated alert blacklist handlers.

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::blacklist::{
    BlacklistDeleteRequest, BlacklistUpsertRequest,
};

use crate::{auth, core, services, state::AppState};

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::blacklist::list,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<BlacklistUpsertRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::blacklist::upsert,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn delete(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<BlacklistDeleteRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::blacklist::delete,
        &auth::InternalApiKeyAuth,
    )
    .await
}
