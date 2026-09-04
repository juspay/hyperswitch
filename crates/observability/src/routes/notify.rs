//! Handlers for the notify routes. The route tree that mounts them is in [`crate::routes::app`].
//!
//! Each handler deserializes, calls [`crate::core::notifier`], and converts the outcome into a
//! response. A provider refusing the message is a `200` describing that, not an error — see
//! [`crate::types`] for why.

use actix_multipart::{Field, Multipart};
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use common_utils::errors::ErrorSwitch;
use error_stack::{report, ResultExt};
use futures_util::StreamExt as _;

use crate::{
    auth::{self, Authenticate as _},
    core,
    domain::notifier::chat::ChatAttachment,
    errors::{types::ApiErrorResponse, ObservabilityApiResult, ObservabilityError},
    logger, services,
    state::AppState,
    types::{
        ChatNotifyRequest, ChatNotifyResponse, ChatUploadResponse, EmailNotifyRequest,
        EmailNotifyResponse,
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
///
/// `multipart/form-data` rather than JSON, because the body carries a file. Base64 in the existing
/// JSON body was the alternative and costs a third more on the wire plus pushing the whole file
/// through `serde_json` as a `String`; multipart is also what the R alerts service already speaks.
///
/// **Authentication happens twice, on purpose.** The multipart body is not read until the key has
/// been checked, so an unauthenticated caller cannot make this process buffer megabytes. The
/// re-check inside [`services::server_wrap`] costs a string comparison and keeps the property that
/// every route names its authentication as a required argument rather than relying on a guard
/// somewhere up the file.
pub async fn chat_upload(
    state: web::Data<AppState>,
    request: HttpRequest,
    destination: web::Path<String>,
    payload: Multipart,
) -> HttpResponse {
    let destination = destination.into_inner();
    let state = state.get_ref().clone();

    if let Err(error) = auth::InternalApiKeyAuth.authenticate(request.headers(), &state) {
        logger::warn!(
            path = %request.path(),
            peer_address = ?request.peer_addr(),
            error = ?error,
            "Upload rejected: authentication failed"
        );
        return ErrorSwitch::<ApiErrorResponse>::switch(error.current_context()).error_response();
    }

    let attachment =
        match read_attachment(payload, state.conf.chat.get_inner().max_upload_bytes).await {
            Ok(attachment) => attachment,
            Err(error) => {
                // The report carries the detail — which field, how many bytes — and the client gets
                // only the fixed reason, because a filename is named after the run that produced it.
                logger::warn!(
                    path = %request.path(),
                    error = ?error,
                    "Upload rejected: the body could not be read"
                );
                return ErrorSwitch::<ApiErrorResponse>::switch(error.current_context())
                    .error_response();
            }
        };

    services::server_wrap(
        state,
        &request,
        attachment,
        |state, attachment| async move {
            core::notifier::upload_chat_file(state, &destination, attachment)
                .await
                .map(ChatUploadResponse::from)
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// The multipart field carrying the file itself. Everything else is metadata about it.
const FILE_FIELD: &str = "file";

/// A ceiling on the metadata fields, so a caller cannot avoid the file cap by sending a gigabyte
/// of `title`. Generous next to a comment, which the chat backends truncate at 10,000 characters
/// anyway.
const MAX_TEXT_FIELD_BYTES: usize = 64 * 1024;

/// Read a `multipart/form-data` body into an attachment, refusing anything oversized as it goes.
///
/// The cap is enforced **while streaming**, not after: buffering the whole body and then measuring
/// it would let a caller spend the memory the limit exists to protect.
async fn read_attachment(
    mut payload: Multipart,
    max_upload_bytes: usize,
) -> ObservabilityApiResult<ChatAttachment> {
    let mut bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut declared_filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut title: Option<String> = None;
    let mut comment: Option<String> = None;
    let mut reply_to: Option<String> = None;

    while let Some(field) = payload.next().await {
        // `map_err` rather than `change_context`: `actix_multipart::MultipartError` is not a
        // `Send + Sync` error, so error-stack will not take it as a context. Its text is kept as
        // an attachment so nothing diagnostic is lost.
        let mut field = field.map_err(|error| {
            report!(ObservabilityError::InvalidUpload {
                reason: "The multipart body could not be read",
            })
            .attach_printable(error.to_string())
        })?;

        let name = field.name().to_owned();

        if name == FILE_FIELD {
            // One call uploads one file. A second part would otherwise replace the first without
            // saying so, and the caller would see a success for a report that never went up.
            if bytes.is_some() {
                Err(ObservabilityError::InvalidUpload {
                    reason: "The multipart body carried more than one `file` part",
                })?
            }

            declared_filename = field
                .content_disposition()
                .get_filename()
                .map(str::to_owned)
                .filter(|name| !name.trim().is_empty());
            content_type = field.content_type().map(ToString::to_string);

            let mut contents = Vec::new();
            while let Some(chunk) = field.next().await {
                let chunk = chunk.map_err(|error| {
                    report!(ObservabilityError::InvalidUpload {
                        reason: "The file contents could not be read",
                    })
                    .attach_printable(error.to_string())
                })?;

                if contents.len() + chunk.len() > max_upload_bytes {
                    Err(ObservabilityError::InvalidUpload {
                        reason: "The file is larger than this service accepts",
                    })
                    .attach_printable_lazy(|| format!("the cap is {max_upload_bytes} bytes"))?
                }

                contents.extend_from_slice(&chunk);
            }

            bytes = Some(contents);
            continue;
        }

        let value = read_text_field(&mut field, &name).await?;

        match name.as_str() {
            "filename" => filename = Some(value),
            "title" => title = Some(value),
            "comment" => comment = Some(value),
            "reply_to" => reply_to = Some(value),
            // Unknown fields are refused rather than ignored, matching `deny_unknown_fields` on
            // the JSON routes: a misspelled `reply_to` must not silently post outside the thread.
            _ => Err(ObservabilityError::InvalidUpload {
                reason: "The multipart body carried a field this route does not accept",
            })
            .attach_printable_lazy(|| format!("unexpected field `{name}`"))?,
        }
    }

    let bytes = bytes.ok_or(ObservabilityError::InvalidUpload {
        reason: "The multipart body carried no `file` part",
    })?;

    if bytes.is_empty() {
        Err(ObservabilityError::InvalidUpload {
            reason: "The file is empty",
        })?
    }

    // An explicit `filename` field wins over the one on the part, so a caller streaming from a
    // temporary path can still name the file what it should be called in the channel.
    let filename = filename
        .or(declared_filename)
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .ok_or(ObservabilityError::InvalidUpload {
            reason: "The file part carried no filename",
        })?;

    Ok(ChatAttachment {
        filename,
        content_type,
        bytes: bytes.into(),
        title,
        comment: comment.map(Into::into),
        reply_to,
    })
}

/// Read one non-file field as UTF-8, capped.
async fn read_text_field(field: &mut Field, name: &str) -> ObservabilityApiResult<String> {
    let mut value = Vec::new();

    while let Some(chunk) = field.next().await {
        let chunk = chunk.map_err(|error| {
            report!(ObservabilityError::InvalidUpload {
                reason: "A text field could not be read",
            })
            .attach_printable(error.to_string())
        })?;

        if value.len() + chunk.len() > MAX_TEXT_FIELD_BYTES {
            Err(ObservabilityError::InvalidUpload {
                reason: "A text field is larger than this service accepts",
            })
            .attach_printable_lazy(|| format!("field `{name}` exceeded {MAX_TEXT_FIELD_BYTES}"))?
        }

        value.extend_from_slice(&chunk);
    }

    String::from_utf8(value).change_context(ObservabilityError::InvalidUpload {
        reason: "A text field was not valid UTF-8",
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use actix_web::{
        error::PayloadError,
        http::header::{HeaderMap, HeaderValue, CONTENT_TYPE},
        web::Bytes,
    };
    use futures_util::stream;
    use hyperswitch_masking::PeekInterface;

    use super::*;

    const BOUNDARY: &str = "boundarytestvalue";
    const GENEROUS_CAP: usize = 1024 * 1024;

    /// Build a `Multipart` over a body, the way actix would hand one to a handler.
    fn multipart(body: String) -> Multipart {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(&format!("multipart/form-data; boundary={BOUNDARY}")).unwrap(),
        );

        Multipart::new(
            &headers,
            stream::once(async move { Ok::<_, PayloadError>(Bytes::from(body)) }),
        )
    }

    fn text_part(name: &str, value: &str) -> String {
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        )
    }

    fn file_part(filename: Option<&str>, content_type: &str, contents: &str) -> String {
        let disposition = filename.map_or_else(
            || "form-data; name=\"file\"".to_owned(),
            |name| format!("form-data; name=\"file\"; filename=\"{name}\""),
        );

        format!(
            "--{BOUNDARY}\r\nContent-Disposition: {disposition}\r\nContent-Type: \
             {content_type}\r\n\r\n{contents}\r\n"
        )
    }

    fn close() -> String {
        format!("--{BOUNDARY}--\r\n")
    }

    #[tokio::test]
    async fn a_full_body_becomes_an_attachment() {
        let body = format!(
            "{}{}{}{}{}",
            file_part(Some("sr-report.pdf"), "application/pdf", "%PDF-1.4"),
            text_part("title", "SR drop report"),
            text_part("comment", "detail attached"),
            text_part("reply_to", "cmtmsn8md0nsn5rqa4nn5np39"),
            close()
        );

        let attachment = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap();

        assert_eq!(attachment.filename, "sr-report.pdf");
        assert_eq!(attachment.content_type.as_deref(), Some("application/pdf"));
        assert_eq!(attachment.bytes.peek(), b"%PDF-1.4");
        assert_eq!(attachment.title.as_deref(), Some("SR drop report"));
        assert_eq!(
            attachment.comment.as_ref().map(|c| c.peek().as_str()),
            Some("detail attached")
        );
        assert_eq!(
            attachment.reply_to.as_deref(),
            Some("cmtmsn8md0nsn5rqa4nn5np39")
        );
    }

    #[tokio::test]
    async fn a_bare_file_needs_nothing_else() {
        let body = format!(
            "{}{}",
            file_part(Some("chart.png"), "image/png", "PNG"),
            close()
        );

        let attachment = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap();

        assert_eq!(attachment.filename, "chart.png");
        assert!(attachment.title.is_none());
        assert!(attachment.comment.is_none());
        assert!(attachment.reply_to.is_none());
    }

    /// The cap is what stops a caller spending this process's memory, so it has to bite. A body
    /// under it must still get through, or the check is just a smaller outage.
    #[tokio::test]
    async fn the_cap_refuses_an_oversized_file_and_passes_a_small_one() {
        let body = format!(
            "{}{}",
            file_part(Some("big.pdf"), "application/pdf", &"x".repeat(64)),
            close()
        );

        let error = read_attachment(multipart(body.clone()), 32)
            .await
            .unwrap_err();
        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidUpload {
                reason: "The file is larger than this service accepts"
            }
        ));

        assert!(read_attachment(multipart(body), 64).await.is_ok());
    }

    /// A caller cannot get round the file cap by sending an enormous `title` instead.
    #[tokio::test]
    async fn a_text_field_is_capped_too() {
        let body = format!(
            "{}{}{}",
            file_part(Some("r.pdf"), "application/pdf", "x"),
            text_part("title", &"t".repeat(MAX_TEXT_FIELD_BYTES + 1)),
            close()
        );

        let error = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidUpload {
                reason: "A text field is larger than this service accepts"
            }
        ));
    }

    /// The same call `deny_unknown_fields` makes on the JSON routes: a misspelled `reply_to` must
    /// fail rather than quietly post the report outside the thread it belongs in.
    #[tokio::test]
    async fn an_unknown_field_is_refused_rather_than_ignored() {
        let body = format!(
            "{}{}{}",
            file_part(Some("r.pdf"), "application/pdf", "x"),
            text_part("thread_ts", "1.2"),
            close()
        );

        let error = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidUpload {
                reason: "The multipart body carried a field this route does not accept"
            }
        ));
    }

    /// One call uploads one file. Letting a second part win silently would report success for a
    /// report that never went anywhere.
    #[tokio::test]
    async fn a_second_file_part_is_refused_rather_than_overwriting_the_first() {
        let body = format!(
            "{}{}{}",
            file_part(Some("first.pdf"), "application/pdf", "one"),
            file_part(Some("second.pdf"), "application/pdf", "two"),
            close()
        );

        let error = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidUpload {
                reason: "The multipart body carried more than one `file` part"
            }
        ));
    }

    #[tokio::test]
    async fn a_body_with_no_file_part_is_refused() {
        let body = format!("{}{}", text_part("title", "nothing to attach"), close());

        let error = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidUpload {
                reason: "The multipart body carried no `file` part"
            }
        ));
    }

    /// Both backends refuse an empty upload, so refusing it here saves three round trips and
    /// reports the same thing.
    #[tokio::test]
    async fn an_empty_file_is_refused_before_any_round_trip() {
        let body = format!(
            "{}{}",
            file_part(Some("empty.pdf"), "application/pdf", ""),
            close()
        );

        let error = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidUpload {
                reason: "The file is empty"
            }
        ));
    }

    /// An explicit `filename` field wins, so a caller streaming from a temporary path can still
    /// name the file what it should be called in the channel.
    #[tokio::test]
    async fn an_explicit_filename_overrides_the_part() {
        let body = format!(
            "{}{}{}",
            file_part(Some("tmp8f2a.bin"), "application/pdf", "%PDF"),
            text_part("filename", "sr-report-2026-09-04.pdf"),
            close()
        );

        let attachment = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap();
        assert_eq!(attachment.filename, "sr-report-2026-09-04.pdf");
    }

    /// Without a name there is nothing to store the file under, and a backend that invents one
    /// puts an unreadable filename in the channel.
    #[tokio::test]
    async fn a_file_with_no_name_anywhere_is_refused() {
        let body = format!("{}{}", file_part(None, "application/pdf", "%PDF"), close());

        let error = read_attachment(multipart(body), GENEROUS_CAP)
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidUpload {
                reason: "The file part carried no filename"
            }
        ));
    }
}
