use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    auth, core, services,
    state::AppState,
    types::{
        instances::{DimensionWriteRequest, InstanceWriteRequest},
        lifecycle::Channel,
    },
};

pub async fn instances_retrieve(
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
            core::instances::retrieve_instances(state, channel, announcement).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn instances_save(
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
            core::instances::save_instances(state, channel, announcement, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn dimensions_retrieve(
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
            core::instances::retrieve_dimensions(state, channel, announcement).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn dimensions_save(
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
            core::instances::save_dimensions(state, channel, announcement, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
