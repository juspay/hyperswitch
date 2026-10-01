//! Per-request notification logic: resolve a destination, hand the message over, report what
//! happened.
//!
//! The whole of it is "look up the id, call the notifier". That is deliberate — the crate is a
//! pipe, and anything more here would be a decision the caller should have made. What the layer
//! buys is a seam a handler can be tested against without HTTP, and one place where "unknown
//! destination" is turned into an error rather than repeated per route.

use error_stack::report;
use hyperswitch_masking::PeekInterface;

use crate::{
    domain::notifier::{
        chat::{ChatFileOutcome, ChatFileUpload, ChatNotification, ChatOutcome, ChatUpdate},
        email::{EmailNotification, EmailOutcome},
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
    types::{ChatNotifyRequest, ChatUpdateRequest, ChatUploadRequest, EmailNotifyRequest},
};

/// Deliver a chat message to the named destination.
pub async fn notify_chat(
    state: AppState,
    destination: &str,
    request: ChatNotifyRequest,
) -> ObservabilityApiResult<ChatOutcome> {
    // An alert's body sits in a block the provider refuses when empty.
    if request.alert.is_some() && request.text.peek().trim().is_empty() {
        Err(report!(ObservabilityError::InvalidRequest))?
    }

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
            alert: request.alert,
        })
        .await
}

/// Replace the content of an earlier message at the named destination.
pub async fn update_chat(
    state: AppState,
    destination: &str,
    request: ChatUpdateRequest,
) -> ObservabilityApiResult<ChatOutcome> {
    if request.message_id.trim().is_empty()
        // `resolved` recolours an alert's rail; a plain message has none.
        || (request.resolved && request.alert.is_none())
        || (request.alert.is_some() && request.text.peek().trim().is_empty())
    {
        Err(report!(ObservabilityError::InvalidRequest))?
    }

    state
        .chat
        .get(destination)
        .ok_or_else(|| {
            report!(ObservabilityError::UnknownDestination {
                destination: destination.to_owned(),
            })
        })?
        .update(ChatUpdate {
            message_id: request.message_id,
            text: request.text,
            alert: request.alert,
            resolved: request.resolved,
        })
        .await
}

/// Upload a file to the named chat destination.
pub async fn upload_chat_file(
    state: AppState,
    destination: &str,
    request: ChatUploadRequest,
) -> ObservabilityApiResult<ChatFileOutcome> {
    let filename = request
        .filename
        .filter(|filename| !filename.peek().trim().is_empty())
        .ok_or_else(|| report!(ObservabilityError::InvalidRequest))?;
    if request.bytes.peek().is_empty() {
        Err(report!(ObservabilityError::InvalidRequest))?
    }

    state
        .chat
        .get(destination)
        .ok_or_else(|| {
            report!(ObservabilityError::UnknownDestination {
                destination: destination.to_owned(),
            })
        })?
        .upload_file(ChatFileUpload {
            bytes: request.bytes,
            filename,
            title: request.title,
            comment: request.comment,
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
