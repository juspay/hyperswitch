//! Handlers for the notify routes. The route tree that mounts them is in [`crate::routes::app`].
//!
//! Each handler deserializes, calls [`crate::core::notifier`], and converts the outcome into a
//! response. A provider refusing the message is a `200` describing that, not an error — see
//! [`crate::types`] for why.

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use common_utils::errors::ErrorSwitch;
use futures::StreamExt;
use hyperswitch_masking::Secret;

use crate::{
    auth::{self, Authenticate},
    core,
    errors::types::{ApiError, ApiErrorResponse},
    logger, services,
    state::AppState,
    types::{
        ChatNotifyRequest, ChatNotifyResponse, ChatUploadRequest, ChatUploadResponse,
        EmailNotifyRequest, EmailNotifyResponse,
    },
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

/// `POST /alerts/chat/upload/{destination}`.
pub async fn chat_upload(
    state: web::Data<AppState>,
    request: HttpRequest,
    destination: web::Path<String>,
    payload: Multipart,
) -> HttpResponse {
    // Multipart is a stream. Authenticate before consuming it so an unauthenticated caller cannot
    // make this process buffer a large file. `server_wrap` checks again after parsing to preserve
    // the required-auth argument on every core invocation.
    if let Err(error) = auth::InternalApiKeyAuth.authenticate(request.headers(), state.get_ref()) {
        return ErrorSwitch::<ApiErrorResponse>::switch(error.current_context()).error_response();
    }

    let max_bytes = state.conf.chat.get_inner().max_upload_bytes;
    let payload = match parse_upload(payload, max_bytes).await {
        Ok(payload) => payload,
        Err(reason) => {
            logger::warn!(path = %request.path(), reason, "Upload request rejected");
            return ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                4,
                "The request body could not be parsed",
            ))
            .error_response();
        }
    };
    let destination = destination.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload,
        |state, payload| async move {
            core::notifier::upload_chat_file(state, &destination, payload)
                .await
                .map(ChatUploadResponse::from)
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

async fn parse_upload(
    mut multipart: Multipart,
    max_bytes: usize,
) -> Result<ChatUploadRequest, &'static str> {
    let mut total = 0usize;
    let mut file: Option<(Vec<u8>, Option<String>)> = None;
    let mut filename: Option<String> = None;
    let mut title: Option<Secret<String>> = None;
    let mut comment: Option<Secret<String>> = None;
    let mut reply_to: Option<String> = None;

    while let Some(field) = multipart.next().await {
        let mut field = field.map_err(|_| "invalid multipart field")?;
        let disposition = field.content_disposition();
        let name = disposition
            .get_name()
            .ok_or("multipart field has no name")?
            .to_owned();
        if !matches!(
            name.as_str(),
            "file" | "filename" | "title" | "comment" | "reply_to"
        ) {
            return Err("unknown multipart field");
        }

        let part_filename = disposition.get_filename().map(str::to_owned);
        let mut bytes = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_err(|_| "multipart field could not be read")?;
            total = total
                .checked_add(chunk.len())
                .ok_or("upload is too large")?;
            if total > max_bytes {
                return Err("upload is too large");
            }
            bytes.extend_from_slice(&chunk);
        }

        if name == "file" {
            if file.replace((bytes, part_filename)).is_some() {
                return Err("file field was supplied more than once");
            }
            continue;
        }

        let value = String::from_utf8(bytes).map_err(|_| "text field is not UTF-8")?;
        let slot = match name.as_str() {
            "filename" => &mut filename,
            "reply_to" => &mut reply_to,
            "title" => {
                if title.replace(value.into()).is_some() {
                    return Err("title field was supplied more than once");
                }
                continue;
            }
            "comment" => {
                if comment.replace(value.into()).is_some() {
                    return Err("comment field was supplied more than once");
                }
                continue;
            }
            _ => return Err("unknown multipart field"),
        };
        if slot.replace(value).is_some() {
            return Err("multipart field was supplied more than once");
        }
    }

    let (bytes, part_filename) = file.ok_or("file field is required")?;
    if bytes.is_empty() {
        return Err("file must not be empty");
    }
    let filename = filename
        .or(part_filename)
        .filter(|value| !value.trim().is_empty())
        .ok_or("filename is required")?;

    Ok(ChatUploadRequest {
        bytes,
        filename,
        title,
        comment,
        reply_to,
    })
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
