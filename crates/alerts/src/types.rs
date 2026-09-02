//! The wire contract: what a caller sends and what it gets back.
//!
//! Lives here rather than in an API-models crate for the same reason [`crate::errors::types`] does
//! — `alerts` has none — and moves wholesale if one ever appears.
//!
//! ## The URL says where, the body says what
//!
//! `POST /alerts/chat/notify/{destination}`. The path names the channel and the destination; the
//! body carries only content. That keeps the destination visible to access logs, metrics labels and
//! tracing spans without anyone parsing a body, so "which destination is failing" is answerable from
//! the ops view.
//!
//! A single `/notify/{destination}` over a channel-tagged body was considered and rejected: the
//! destination already resolves the channel through configuration, so a tag in the body is a second
//! authority on the same fact and the two can disagree.
//!
//! ## Status answers "did the notifier work", the body answers "did the message arrive"
//!
//! A provider that refuses is a `200` carrying [`NotifyStatus::Refused`], not an HTTP error. It was
//! reached, it answered, and the notifier did its job. Only a request we cannot act on (`4xx`), an
//! unreachable provider (`502`) or our own fault (`500`) is an error — so an alert on `5xx` fires
//! when this service is genuinely broken and at no other time.
//!
//! This is the same line payments draws between a connector declining a transaction and a connector
//! being unreachable, and it is drawn deliberately rather than by fault. Whether `channel_not_found`
//! is our mistake or a merchant's depends on who owns the destination, and that moves from a config
//! file to a database row without a status code being able to move with it.
//!
//! **`status` is required, and that is load-bearing.** A caller cannot deserialize a response
//! without confronting whether the message arrived. `external_services` uses the same trick on the
//! provider's own `ok` field, for the same reason: this shape's failure mode is a caller that reads
//! `200` and stops looking.
//!
//! ## Content is `Secret`, so redaction is the type's job
//!
//! `text`, `subject` and `body` are `Secret<String>`. A subject carries merchant ids and a body
//! carries payment volumes, and `services::server_wrap` takes `T: Debug`, so one added log line
//! would otherwise put both in the log stream. A hand-written `Debug` would do the same job until
//! somebody adds a field and forgets; the type cannot forget.
//!
//! Sizes are logged where they are useful — the chat client already emits `chars` per request — so
//! nothing diagnostic is lost by redacting here.
//!
//! ## Nothing here renders
//!
//! `text`, `subject` and `body` are delivered exactly as they arrive. The caller decides what its
//! message looks like, in whatever markup its destination reads. `body` is HTML, because both email
//! backends in `external_services` hardcode an HTML body and there is no plain-text path to reach.

use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::domain::notifier::{
    chat::{ChatOutcome, ChatReceipt},
    email::EmailOutcome,
    Outcome, Refusal,
};

/// The body of `POST /alerts/chat/notify/{destination}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatNotifyRequest {
    /// The message, in the markup the destination reads. Delivered unchanged.
    pub text: Secret<String>,

    /// Post this as a reply in the thread of an earlier message, identified by the `message_id`
    /// that message's response returned.
    #[serde(default)]
    pub reply_to: Option<String>,
}

/// The body of `POST /alerts/email/notify/{destination}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailNotifyRequest {
    /// The subject line, delivered unchanged.
    pub subject: Secret<String>,

    /// The body, as HTML. See the module docs: the transport offers nothing else today.
    pub body: Secret<String>,
}

/// Whether the message arrived.
///
/// Not a bool, so a third outcome can be added without breaking a caller's match, and so the two
/// states read the same in a log line as they do in code.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotifyStatus {
    /// The provider accepted the message.
    Delivered,
    /// The provider was reached and refused it.
    Refused,
}

/// What `/alerts/chat/notify/{destination}` returns.
#[derive(Debug, Serialize)]
pub struct ChatNotifyResponse {
    /// Whether the message arrived. Always present.
    pub status: NotifyStatus,

    /// The provider's id for the message, when it named one. Hand it back as
    /// [`ChatNotifyRequest::reply_to`] to thread under it.
    ///
    /// `null` on a refusal, and also on the rare delivery where the provider accepted the message
    /// without naming an id — the alert went out, but nothing can be threaded under it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// Why the provider refused, as a stable snake_case code — `msg_too_long`,
    /// `channel_not_found`, `rate_limited`. Absent on delivery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    /// How long the provider asked us to wait, when it said. Only set alongside a rate-limiting
    /// code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}

/// What `/alerts/email/notify/{destination}` returns.
#[derive(Debug, Serialize)]
pub struct EmailNotifyResponse {
    /// Whether the mail was sent. Always present.
    pub status: NotifyStatus,

    /// Why the provider refused, as a stable snake_case code. Absent on delivery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    /// How long the provider asked us to wait, when it said.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}

impl From<ChatOutcome> for ChatNotifyResponse {
    fn from(outcome: ChatOutcome) -> Self {
        match outcome {
            Outcome::Delivered(ChatReceipt { message_id }) => Self {
                status: NotifyStatus::Delivered,
                message_id,
                error_code: None,
                retry_after_seconds: None,
            },
            Outcome::Refused(Refusal {
                code,
                retry_after_seconds,
            }) => Self {
                status: NotifyStatus::Refused,
                message_id: None,
                error_code: Some(code),
                retry_after_seconds,
            },
        }
    }
}

impl From<EmailOutcome> for EmailNotifyResponse {
    fn from(outcome: EmailOutcome) -> Self {
        match outcome {
            Outcome::Delivered(()) => Self {
                status: NotifyStatus::Delivered,
                error_code: None,
                retry_after_seconds: None,
            },
            Outcome::Refused(Refusal {
                code,
                retry_after_seconds,
            }) => Self {
                status: NotifyStatus::Refused,
                error_code: Some(code),
                retry_after_seconds,
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn body_of<T: Serialize>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn a_delivery_carries_the_message_id_and_no_error() {
        let body = body_of(&ChatNotifyResponse::from(Outcome::Delivered(ChatReceipt {
            message_id: Some("cmtk931s1".to_owned()),
        })));

        assert_eq!(body["status"], "delivered");
        assert_eq!(body["message_id"], "cmtk931s1");
        assert!(body.get("error_code").is_none());
    }

    /// The alert went out; only the ability to thread under it was lost. It must not look like a
    /// failure, because a retry would post the alert twice.
    #[test]
    fn a_delivery_without_an_id_is_still_a_delivery() {
        let body = body_of(&ChatNotifyResponse::from(Outcome::Delivered(ChatReceipt {
            message_id: None,
        })));

        assert_eq!(body["status"], "delivered");
        assert!(body.get("message_id").is_none());
    }

    #[test]
    fn a_refusal_carries_the_code_and_no_message_id() {
        let body = body_of(&ChatNotifyResponse::from(Outcome::Refused(
            Refusal::retry_after("rate_limited", Some(30)),
        )));

        assert_eq!(body["status"], "refused");
        assert_eq!(body["error_code"], "rate_limited");
        assert_eq!(body["retry_after_seconds"], 30);
        assert!(body.get("message_id").is_none());
    }

    /// `status` is what stops a caller reading `200` and assuming delivery, so it is never skipped.
    #[test]
    fn status_is_always_present() {
        for outcome in [
            Outcome::Delivered(()),
            Outcome::Refused(Refusal::new("channel_not_found")),
        ] {
            let body = body_of(&EmailNotifyResponse::from(outcome));
            assert!(body.get("status").is_some());
        }
    }

    #[test]
    fn chat_request_reads_a_threaded_message() {
        let request: ChatNotifyRequest =
            serde_json::from_value(serde_json::json!({ "text": "hi", "reply_to": "cmtk931s1" }))
                .unwrap();

        assert_eq!(request.reply_to.as_deref(), Some("cmtk931s1"));
    }

    /// Threading against a mailing list is a caller bug. `deny_unknown_fields` makes it a rejection
    /// rather than a field that quietly goes nowhere.
    #[test]
    fn email_request_rejects_reply_to() {
        let error = serde_json::from_value::<EmailNotifyRequest>(serde_json::json!({
            "subject": "s",
            "body": "<pre>b</pre>",
            "reply_to": "cmtk931s1",
        }))
        .unwrap_err();

        assert!(error.to_string().contains("reply_to"));
    }

    /// The property is now the type's, not a hand-written `Debug`'s: a field added later cannot
    /// leak by someone forgetting to update an impl.
    #[test]
    fn debug_never_prints_the_message() {
        let chat = ChatNotifyRequest {
            text: "acquirer_declined for merchant_1234".to_owned().into(),
            reply_to: None,
        };
        assert!(!format!("{chat:?}").contains("merchant_1234"));

        let email = EmailNotifyRequest {
            subject: "merchant_1234 not converting".to_owned().into(),
            body: "<pre>4,201 of 5,000 payments lost</pre>".to_owned().into(),
        };
        let rendered = format!("{email:?}");
        assert!(!rendered.contains("merchant_1234"));
        assert!(!rendered.contains("4,201"));
    }
}
