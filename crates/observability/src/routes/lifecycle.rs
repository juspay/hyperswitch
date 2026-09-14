use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    auth, core, services,
    state::AppState,
    types::lifecycle::{AnnouncementRequest, Channel, LifecycleStateWriteRequest},
};

pub async fn read_state(
    state: web::Data<AppState>,
    request: HttpRequest,
    channel: web::Path<Channel>,
) -> HttpResponse {
    let channel = channel.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::lifecycle::read_state(state, channel).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn write_state(
    state: web::Data<AppState>,
    request: HttpRequest,
    channel: web::Path<Channel>,
    payload: web::Json<LifecycleStateWriteRequest>,
) -> HttpResponse {
    let channel = channel.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move { core::lifecycle::write_state(state, channel, payload).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn record_announcement(
    state: web::Data<AppState>,
    request: HttpRequest,
    channel: web::Path<Channel>,
    payload: web::Json<AnnouncementRequest>,
) -> HttpResponse {
    let channel = channel.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::lifecycle::record_announcement(state, channel, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
