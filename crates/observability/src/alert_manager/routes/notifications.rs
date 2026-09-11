use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{core, types::UserName},
    auth, services,
    state::AppState,
};

pub async fn read_watermark(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    let user = UserName::from_headers(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::notifications::read_watermark(state, user?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

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
