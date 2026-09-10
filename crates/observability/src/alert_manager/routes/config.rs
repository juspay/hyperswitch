//! Handlers for the alert configuration routes. The route tree that mounts them is in
//! [`crate::routes::app`].
//!
//! Each handler deserializes, calls [`crate::alert_manager::core::config`], and converts the
//! outcome into a response — the same shape as [`crate::routes::notify`], down to the required
//! authentication argument on [`crate::services::server_wrap`].
//!
//! Reads pass `()` as the payload. `server_wrap` takes one so that authentication cannot be
//! forgotten, and a route with no body still has to go through it; the unit is the honest spelling
//! of "there is nothing here to log".

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
///
/// `POST` rather than `PUT`, because the body is a partial change and `PUT` means replacement.
/// Sending a whole row would be the thing that makes two portal screens overwrite each other.
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
///
/// A real upsert: one statement with the table's composite primary key as its conflict target, so
/// a repeated call updates the row it wrote the first time instead of adding a second one that
/// disagrees with it.
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
