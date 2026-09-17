//! Authenticated threshold override handlers.

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::thresholds::{
    ThresholdDeleteRequest, ThresholdUpsertRequest,
};

use crate::{auth, core, services, state::AppState};

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::thresholds::list,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<ThresholdUpsertRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::thresholds::upsert,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn delete(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<ThresholdDeleteRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::thresholds::delete,
        &auth::InternalApiKeyAuth,
    )
    .await
}
