use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    auth, core, services,
    state::AppState,
    types::{
        instances::{DimensionWriteRequest, InstanceWriteRequest},
        lifecycle::Channel,
    },
};

pub async fn read_instances(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(Channel, uuid::Uuid)>,
) -> HttpResponse {
    let (channel, announcement) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::instances::read_instances(state, channel, announcement).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn write_instances(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(Channel, uuid::Uuid)>,
    payload: web::Json<InstanceWriteRequest>,
) -> HttpResponse {
    let (channel, announcement) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::instances::write_instances(state, channel, announcement, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn read_dimensions(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(Channel, uuid::Uuid)>,
) -> HttpResponse {
    let (channel, announcement) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move {
            core::instances::read_dimensions(state, channel, announcement).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn write_dimensions(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(Channel, uuid::Uuid)>,
    payload: web::Json<DimensionWriteRequest>,
) -> HttpResponse {
    let (channel, announcement) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::instances::write_dimensions(state, channel, announcement, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
