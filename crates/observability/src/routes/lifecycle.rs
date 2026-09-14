use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    auth, core, services,
    state::AppState,
    types::lifecycle::{
        AnnouncementListRequest, AnnouncementRequest, AnnouncementUpdateRequest, Channel,
        LifecycleStateWriteRequest,
    },
};

pub async fn lifecycle_state_retrieve(
    state: web::Data<AppState>,
    request: HttpRequest,
    channel: web::Path<Channel>,
) -> HttpResponse {
    let channel = channel.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::lifecycle::retrieve_lifecycle_state(state, channel).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn lifecycle_state_save(
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
        |state, payload| async move {
            core::lifecycle::save_lifecycle_state(state, channel, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn announcement_create(
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
            core::lifecycle::create_announcement(state, channel, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn announcement_list(
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

pub async fn announcement_update(
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
