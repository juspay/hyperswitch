//! Authenticated alert rule handlers.

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::alert_rules::AlertRuleUpsertRequest;

use crate::{auth, core, services, state::AppState};

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::alert_rules::list,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn get(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        id.into_inner(),
        core::alert_rules::get,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
    payload: web::Json<AlertRuleUpsertRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (id.into_inner(), payload.into_inner()),
        core::alert_rules::upsert,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn delete(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        id.into_inner(),
        core::alert_rules::delete,
        &auth::InternalApiKeyAuth,
    )
    .await
}
