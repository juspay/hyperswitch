use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{
        core,
        types::lifecycle::{AnnouncementRequest, Channel, LifecycleStateWriteRequest},
    },
    auth, services,
    state::AppState,
};

pub async fn read_state(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let channel = Channel::from_path(&path.into_inner());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::lifecycle::read_state(state, channel?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn write_state(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<LifecycleStateWriteRequest>,
) -> HttpResponse {
    let channel = Channel::from_path(&path.into_inner());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::lifecycle::write_state(state, channel?, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn record_announcement(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<String>,
    payload: web::Json<AnnouncementRequest>,
) -> HttpResponse {
    let channel = Channel::from_path(&path.into_inner());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::lifecycle::record_announcement(state, channel?, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
