use actix_web::{web, HttpRequest, HttpResponse};

use crate::{auth, core, services, state::AppState};

pub async fn read_watermark(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    let user_name = auth::get_required_user_name(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::notifications::read_watermark(state, user_name?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn mark_read(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    let user_name = auth::get_required_user_name(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::notifications::mark_read(state, user_name?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}
