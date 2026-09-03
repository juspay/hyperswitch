//! Per-request notification logic: resolve a destination, hand the message over, report what
//! happened.
//!
//! The whole of it is "look up the id, call the notifier". That is deliberate — the crate is a
//! pipe, and anything more here would be a decision the caller should have made. What the layer
//! buys is a seam a handler can be tested against without HTTP, and one place where "unknown
//! destination" is turned into an error rather than repeated per route.

use error_stack::report;

use crate::{
    domain::notifier::{
        chat::{ChatNotification, ChatOutcome},
        email::{EmailNotification, EmailOutcome},
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
    types::{ChatNotifyRequest, EmailNotifyRequest},
};

/// Deliver a chat message to the named destination.
pub async fn notify_chat(
    state: AppState,
    destination: &str,
    request: ChatNotifyRequest,
) -> ObservabilityApiResult<ChatOutcome> {
    state
        .chat
        .get(destination)
        .ok_or_else(|| {
            report!(ObservabilityError::UnknownDestination {
                destination: destination.to_owned(),
            })
        })?
        .notify(ChatNotification {
            text: request.text,
            reply_to: request.reply_to,
        })
        .await
}

/// Deliver an email to the named destination.
pub async fn notify_email(
    state: AppState,
    destination: &str,
    request: EmailNotifyRequest,
) -> ObservabilityApiResult<EmailOutcome> {
    state
        .email
        .get(destination)
        .ok_or_else(|| {
            report!(ObservabilityError::UnknownDestination {
                destination: destination.to_owned(),
            })
        })?
        .notify(EmailNotification {
            subject: request.subject,
            body: request.body,
        })
        .await
}
