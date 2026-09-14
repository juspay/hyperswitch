use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    auth, core, services,
    state::AppState,
    types::lifecycle::{
        AnnouncementListRequest, AnnouncementRequest, AnnouncementUpdateRequest, Channel,
        LifecycleStateWriteRequest,
    },
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

pub async fn list_announcements(
    state: web::Data<AppState>,
    request: HttpRequest,
    channel: web::Path<Channel>,
    query: web::Query<AnnouncementListRequest>,
) -> HttpResponse {
    let channel = channel.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        query.into_inner(),
        |state, query| async move { core::lifecycle::list_announcements(state, channel, query).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn update_announcement(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(Channel, uuid::Uuid)>,
    payload: web::Json<AnnouncementUpdateRequest>,
) -> HttpResponse {
    let (channel, id) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::lifecycle::update_announcement(state, channel, id, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
