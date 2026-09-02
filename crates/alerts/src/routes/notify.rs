//! Handlers for the notify routes. The route tree that mounts them is in [`crate::routes::app`].
//!
//! Each handler deserializes, calls [`crate::core::notifier`], and converts the outcome into a
//! response. A provider refusing the message is a `200` describing that, not an error — see
//! [`crate::types`] for why.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    auth, core, services,
    state::AppState,
    types::{ChatNotifyRequest, ChatNotifyResponse, EmailNotifyRequest, EmailNotifyResponse},
};

/// `POST /alerts/chat/notify/{destination}`.
pub async fn chat(
    state: web::Data<AppState>,
    request: HttpRequest,
    destination: web::Path<String>,
    payload: web::Json<ChatNotifyRequest>,
) -> HttpResponse {
    let destination = destination.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::notifier::notify_chat(state, &destination, payload)
                .await
                .map(ChatNotifyResponse::from)
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/email/notify/{destination}`.
pub async fn email(
    state: web::Data<AppState>,
    request: HttpRequest,
    destination: web::Path<String>,
    payload: web::Json<EmailNotifyRequest>,
) -> HttpResponse {
    let destination = destination.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::notifier::notify_email(state, &destination, payload)
                .await
                .map(EmailNotifyResponse::from)
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
