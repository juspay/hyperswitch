//! Handlers for the notify routes. The route tree that mounts them is in [`crate::routes::app`].
//!
//! Both handlers are the same four steps: resolve the destination, hand the message over
//! unchanged, and render whatever comes back. Nothing here formats, truncates or escapes. The
//! caller decides what its message looks like, because it is the side that knows what it is
//! saying and in which markup — the `hyperswitch-alerts` R service already renders `mrkdwn` for
//! chat and something else entirely for email, and re-deciding that here would mean owning a
//! renderer for a domain this crate deliberately knows nothing about.

use actix_web::{web, HttpRequest, HttpResponse};
use error_stack::report;

use crate::{
    auth,
    core::notifier::{chat::ChatNotification, email::EmailNotification},
    errors::AlertsError,
    services,
    state::AppState,
    types::{ChatNotifyRequest, ChatNotifyResponse, EmailNotifyRequest, EmailNotifyResponse},
};

/// `POST /alerts/chat/notify`.
pub async fn chat(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<ChatNotifyRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            let notifier = state.chat.get(&payload.destination).ok_or_else(|| {
                report!(AlertsError::UnknownDestination {
                    destination: payload.destination.clone(),
                })
            })?;

            let receipt = notifier
                .notify(ChatNotification {
                    text: payload.text,
                    reply_to: payload.reply_to,
                })
                .await?;

            Ok(ChatNotifyResponse {
                message_id: receipt.message_id,
            })
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `POST /alerts/email/notify`.
pub async fn email(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<EmailNotifyRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            let notifier = state.email.get(&payload.destination).ok_or_else(|| {
                report!(AlertsError::UnknownDestination {
                    destination: payload.destination.clone(),
                })
            })?;

            notifier
                .notify(EmailNotification {
                    subject: payload.subject,
                    body: payload.body,
                })
                .await?;

            Ok(EmailNotifyResponse {})
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
