use error_stack::report;
use external_services::chat_service::ChatBanner;
use hyperswitch_masking::{ExposeInterface, PeekInterface};

use crate::{
    domain::notifier::{
        chat::{ChatFileOutcome, ChatFileUpload, ChatNotification, ChatOutcome},
        email::{EmailNotification, EmailOutcome},
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
    types::{ChatNotifyRequest, ChatUploadRequest, EmailNotifyRequest},
};

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
            banner: request
                .heading
                .zip(request.severity)
                .map(|(heading, severity)| ChatBanner::new(heading.expose(), severity.into())),
        })
        .await
}

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
