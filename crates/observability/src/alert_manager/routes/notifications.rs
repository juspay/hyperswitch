//! Handlers for the notification bell's read watermark.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{core, types::UserName},
    auth, services,
    state::AppState,
};

/// `GET /alerts/config/notifications/read`.
pub async fn read(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    let user = UserName::from_headers(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::notifications::read(state, user?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/config/notifications/read`.
pub async fn mark_read(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    let user = UserName::from_headers(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::notifications::mark_read(state, user?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}
