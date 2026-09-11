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

pub async fn read_instances(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, uuid::Uuid)>,
) -> HttpResponse {
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
