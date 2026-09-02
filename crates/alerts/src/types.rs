//! The wire contract: what a caller sends and what it gets back.
//!
//! Lives here rather than in an API-models crate for the same reason
//! [`crate::errors::types`] does — `alerts` has none — and moves wholesale if one ever appears.
//!
//! **One route per channel, and the path names the channel.** `/chat/notify` and `/email/notify`
//! carry different bodies because chat and email genuinely differ: chat threads and returns a
//! message id, email has a subject and does neither. The alternative considered was a single
//! `/notify/{id}` whose body is a tagged union. It was rejected because the destination id already
//! resolves the channel through configuration, so a tag in the body is a second authority on the
//! same fact, and the two can disagree. Under this shape they cannot.
//!
//! **Every request body is `deny_unknown_fields`.** That is what makes `reply_to` on
//! [`EmailNotifyRequest`] a 400 rather than a silently dropped field. A caller threading a
//! recovery notice against a mailing list has a bug, and finding out weeks later because nothing
//! ever linked back is the failure mode this prevents.
//!
//! **Nothing here renders.** `text`, `subject` and `body` are delivered exactly as they arrive.
//! The caller decides what its message looks like, in whatever markup its destination reads —
//! Slack `mrkdwn` for chat, and HTML for email, because both email backends in
//! `external_services` hardcode an HTML body (`email/ses.rs` builds it with `.html(...)`,
//! `email/smtp.rs` sets `ContentType::TEXT_HTML`) and there is no plain-text path to reach.
//! hyperswitch-cloud#23160 tracks growing one.

use std::fmt;

use serde::{Deserialize, Serialize};

/// `POST /alerts/chat/notify`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatNotifyRequest {
    /// Names a destination under `chat.destinations` in configuration. Unknown ids are rejected;
    /// the request never carries a channel id or a credential of its own.
    pub destination: String,

    /// The message, in the markup the destination reads. Delivered unchanged.
    pub text: String,

    /// Post this as a reply in the thread of an earlier message, identified by the `message_id`
    /// that message's [`ChatNotifyResponse`] returned.
    ///
    /// The R alerts service uses this to put a recovery notice under the alert it clears, so a
    /// reader sees the two together rather than as unrelated messages.
    #[serde(default)]
    pub reply_to: Option<String>,
}

/// `POST /alerts/email/notify`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailNotifyRequest {
    /// Names a destination under `email.destinations` in configuration.
    pub destination: String,

    /// The subject line, delivered unchanged.
    pub subject: String,

    /// The body, as HTML. See the module docs: the transport offers nothing else today.
    pub body: String,
}

/// What `/alerts/chat/notify` returns once the provider has accepted the message.
#[derive(Debug, Serialize)]
pub struct ChatNotifyResponse {
    /// The provider's id for the message just posted. Hand it back as
    /// [`ChatNotifyRequest::reply_to`] to thread under it.
    pub message_id: String,
}

/// What `/alerts/email/notify` returns.
///
/// Deliberately empty rather than `{"ok": true}`. The status code already says whether it worked,
/// and a success field in the body invites a caller to check the field instead of the status,
/// which is exactly the mistake the chat provider's own `{"ok": false}` at HTTP 200 causes.
#[derive(Debug, Serialize)]
pub struct EmailNotifyResponse {}

// `Debug` is written out rather than derived on both requests, because `services::server_wrap`
// takes `T: Debug` and one added log line would otherwise put a subject full of merchant ids and a
// body full of payment volumes into the log stream. The R service made the same call for the same
// reason: `email_send_message` logs `chars = nchar(message)` and never the message. Sizes are what
// you actually want when an alert did not arrive.

impl fmt::Debug for ChatNotifyRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChatNotifyRequest")
            .field("destination", &self.destination)
            .field("text_chars", &self.text.chars().count())
            .field("threaded", &self.reply_to.is_some())
            .finish()
    }
}

impl fmt::Debug for EmailNotifyRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmailNotifyRequest")
            .field("destination", &self.destination)
            .field("subject_chars", &self.subject.chars().count())
            .field("body_chars", &self.body.chars().count())
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_reads_a_threaded_message() {
        let request: ChatNotifyRequest = serde_json::from_value(serde_json::json!({
            "destination": "sr_alerts",
            "text": "*3 merchants not converting*",
            "reply_to": "1503435956.000247",
        }))
        .unwrap();

        assert_eq!(request.destination, "sr_alerts");
        assert_eq!(request.reply_to.as_deref(), Some("1503435956.000247"));
    }

    #[test]
    fn chat_request_defaults_reply_to() {
        let request: ChatNotifyRequest = serde_json::from_value(serde_json::json!({
            "destination": "sr_alerts",
            "text": "hello",
        }))
        .unwrap();

        assert!(request.reply_to.is_none());
    }

    /// The whole point of `deny_unknown_fields`: threading against email is a rejection, not a
    /// field that quietly goes nowhere.
    #[test]
    fn email_request_rejects_reply_to() {
        let error = serde_json::from_value::<EmailNotifyRequest>(serde_json::json!({
            "destination": "oncall",
            "subject": "[Hyperswitch] 3 merchants not converting",
            "body": "<pre>...</pre>",
            "reply_to": "1503435956.000247",
        }))
        .unwrap_err();

        assert!(error.to_string().contains("reply_to"));
    }

    #[test]
    fn debug_never_prints_the_message() {
        let chat = ChatNotifyRequest {
            destination: "sr_alerts".to_owned(),
            text: "acquirer_declined for merchant_1234".to_owned(),
            reply_to: None,
        };
        let rendered = format!("{chat:?}");
        assert!(!rendered.contains("merchant_1234"));
        assert!(rendered.contains("text_chars"));

        let email = EmailNotifyRequest {
            destination: "oncall".to_owned(),
            subject: "merchant_1234 not converting".to_owned(),
            body: "<pre>4,201 of 5,000 payments lost</pre>".to_owned(),
        };
        let rendered = format!("{email:?}");
        assert!(!rendered.contains("merchant_1234"));
        assert!(!rendered.contains("4,201"));
    }
}
