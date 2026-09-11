//! Handlers for per-merchant alert instances and their per-dimension breakdown.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{
        core,
        types::{
            instances::{DimensionWriteRequest, InstanceWriteRequest},
            lifecycle::Channel,
        },
    },
    auth, services,
    state::AppState,
};

/// `GET /alerts/instances/{channel}/{announcement_id}`.
pub async fn read_instances(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, uuid::Uuid)>,
) -> HttpResponse {
    // Resolved here and carried into the closure, the way the lifecycle routes carry theirs.
    let (channel, announcement) = path.into_inner();
    let channel = Channel::from_path(&channel);

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::instances::read_instances(state, channel?, announcement).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/instances/{channel}/{announcement_id}`.
pub async fn write_instances(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, uuid::Uuid)>,
    payload: web::Json<InstanceWriteRequest>,
) -> HttpResponse {
    let (channel, announcement) = path.into_inner();
    let channel = Channel::from_path(&channel);

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::instances::write_instances(state, channel?, announcement, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/dimensions/{announcement_id}`.
pub async fn read_dimensions(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
) -> HttpResponse {
    let announcement = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::instances::read_dimensions(state, announcement).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/dimensions/{announcement_id}`.
pub async fn write_dimensions(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<uuid::Uuid>,
    payload: web::Json<DimensionWriteRequest>,
) -> HttpResponse {
    let announcement = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::instances::write_dimensions(state, announcement, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
