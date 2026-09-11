//! Handlers for the alert configuration routes.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{
        core,
        types::config::{
            AlertDefinitionCreateRequest, AlertDefinitionUpdateRequest,
            AlertEnablementUpsertRequest,
        },
    },
    auth, services,
    state::AppState,
};

/// `GET /alerts/config/definitions`.
pub async fn list_definitions(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::config::list_definitions(state).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/config/definitions`.
pub async fn create_definition(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<AlertDefinitionCreateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move { core::config::create_definition(state, payload).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/config/definitions/{id}`.
pub async fn read_definition(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<uuid::Uuid>,
) -> HttpResponse {
    let id = id.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::config::read_definition(state, id).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/config/definitions/{id}`.
pub async fn update_definition(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<uuid::Uuid>,
    payload: web::Json<AlertDefinitionUpdateRequest>,
) -> HttpResponse {
    let id = id.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move { core::config::update_definition(state, id, payload).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/config/enablement`.
pub async fn list_enablements(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::config::list_enablements(state).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/config/enablement/{name}/{product}`.
pub async fn read_enablement(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (name, product) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::config::read_enablement(state, name, product).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/config/enablement/{name}/{product}`.
pub async fn upsert_enablement(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
    payload: web::Json<AlertEnablementUpsertRequest>,
) -> HttpResponse {
    let (name, product) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::config::upsert_enablement(state, name, product, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
