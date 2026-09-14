use actix_web::{web, HttpRequest, HttpResponse};

use crate::{auth, core, services, state::AppState};

pub async fn notification_watermark_retrieve(
    state: web::Data<AppState>,
    request: HttpRequest,
) -> HttpResponse {
    let user_name = auth::get_required_user_name(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move {
            core::notifications::retrieve_notification_watermark(state, user_name?).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn notification_watermark_upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
) -> HttpResponse {
    let user_name = auth::get_required_user_name(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move {
            core::notifications::upsert_notification_watermark(state, user_name?).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
