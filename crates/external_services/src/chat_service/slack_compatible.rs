//! The Slack-compatible wire protocol.
//!
//! Xyne exposes a facade over Slack's own API, so both backends post the same JSON body to the
//! same method name and read the same response envelope. Only the base URL, the path prefix in
//! front of the method and the credential differ, which is why [`Endpoint`] holds those three and
//! the public clients are thin wrappers over it.
//!
//! Everything here is private to [`crate::chat_service`]. The single most important reason is the
//! envelope: this API reports failures **twice over**, once as a non-2xx status and once as HTTP
//! 200 carrying `{"ok": false, "error": "..."}`. A caller that never sees the raw response cannot
//! forget the second one.

use common_utils::request::{Method, RequestBuilder, RequestContent};
use error_stack::ResultExt;
use hyperswitch_interfaces::types::Proxy;
use hyperswitch_masking::{Mask as _, PeekInterface, Secret};
use router_env::logger;
use serde::{Deserialize, Serialize};

use super::{ChatError, ChatErrorReason, ChatMessage, ChatResult, MessageId};
use crate::http_client;

/// The method that posts a message. The only one this crate calls; `files.upload` is out of v1.
const CHAT_POST_MESSAGE: &str = "chat.postMessage";

/// Slack's documented limit on the `text` field of `chat.postMessage`.
pub(super) const DEFAULT_MAX_MESSAGE_CHARS: usize = 40_000;

/// Matches the R alerts service, which has run this in production against Xyne.
pub(super) const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

/// Appended to a message that had to be cut down to fit.
const TRUNCATION_MARKER: &str = "\n…(truncated)";

/// How much of an unexpected response body is worth carrying into the logs.
const BODY_SNIPPET_CHARS: usize = 512;

/// One destination on a Slack-compatible API: where to post, as whom, and to which channel.
#[derive(Clone)]
pub(super) struct Endpoint {
    /// Root of the API, with no trailing slash, e.g. `https://slack.com/api`.
    base_url: String,

    /// Sits between the base URL and the method name. Slack serves methods directly off its base
    /// (`/api/chat.postMessage`); Xyne namespaces them (`/api/apps/slack/chat.postMessage`).
    method_prefix: &'static str,

    /// Presented as `Authorization: Bearer <token>`.
    token: Secret<String>,

    /// Channel id is preferred over channel name; both are accepted, and the provider decides.
    channel: String,

    timeout_seconds: u64,
    max_message_chars: usize,
    proxy: Proxy,
}

/// Hand-written so no future `#[derive(Debug)]` on a field can print the credential.
impl std::fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("base_url", &self.base_url)
            .field("method_prefix", &self.method_prefix)
            .field("channel", &self.channel)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("max_message_chars", &self.max_message_chars)
            .finish_non_exhaustive()
    }
}

impl Endpoint {
    /// Validate a destination and bind it to a proxy.
    ///
    /// Validation happens here rather than at send time so a destination read from configuration
    /// or from a database row fails once, on the way in, rather than once per message.
    pub(super) fn new(
        base_url: String,
        method_prefix: &'static str,
        token: Secret<String>,
        channel: String,
        timeout_seconds: u64,
        max_message_chars: usize,
        proxy: Proxy,
    ) -> ChatResult<Self> {
        let base_url = base_url.trim().trim_end_matches('/').to_owned();
        if base_url.is_empty() {
            Err(ChatError::InvalidConfiguration(
                "base url must not be empty",
            ))?
        }
        url::Url::parse(&base_url).change_context(ChatError::InvalidConfiguration(
            "base url must be a valid URL",
        ))?;

        let channel = channel.trim().to_owned();
        if channel.is_empty() {
            Err(ChatError::InvalidConfiguration(
                "a channel id or channel name is required",
            ))?
        }

        if token.peek().trim().is_empty() {
            Err(ChatError::InvalidConfiguration("token must not be empty"))?
        }

        if max_message_chars == 0 {
            Err(ChatError::InvalidConfiguration(
                "max message length must be greater than zero",
            ))?
        }

        Ok(Self {
            base_url,
            method_prefix,
            token,
            channel,
            timeout_seconds,
            max_message_chars,
            proxy,
        })
    }

    fn method_url(&self, method: &str) -> String {
        format!("{}{}{}", self.base_url, self.method_prefix, method)
    }

    /// Post a message and return the id the provider assigned it.
    pub(super) async fn post_message(&self, message: ChatMessage) -> ChatResult<MessageId> {
        let payload = self.build_payload(&message)?;
        let url = self.method_url(CHAT_POST_MESSAGE);

        // Without this, the only question that matters when no message arrives — did we try, where
        // to, and what came back — has no answer in the logs.
        logger::info!(
            tag = "chat_post_message",
            url = %url,
            channel = %payload.channel,
            threaded = payload.thread_ts.is_some(),
            chars = payload.text.chars().count(),
        );

        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&url)
            .attach_default_headers()
            .headers(vec![
                (
                    http::header::AUTHORIZATION.to_string(),
                    format!("Bearer {}", self.token.peek()).into_masked(),
                ),
                (
                    http::header::CONTENT_TYPE.to_string(),
                    "application/json".to_owned().into(),
                ),
            ])
            .set_body(RequestContent::Json(Box::new(payload)))
            .build();

        let response = http_client::send_request(&self.proxy, request, Some(self.timeout_seconds))
            .await
            .change_context(ChatError::RequestFailed)
            .attach_printable_lazy(|| format!("chat request to {url} was not sent"))?;

        let status = response.status();

        // Read before the body, which consumes the response.
        let retry_after_seconds = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());

        let body = response
            .text()
            .await
            .change_context(ChatError::UnreadableResponse)
            .attach_printable("could not read the chat provider's response body")?;

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(ChatError::Rejected {
                reason: ChatErrorReason::RateLimited {
                    retry_after_seconds,
                },
            })?
        }

        if !status.is_success() {
            Err(ChatError::HttpStatus {
                status: status.as_u16(),
            })
            .attach_printable_lazy(|| {
                format!("chat provider body: {}", snippet(&body, BODY_SNIPPET_CHARS))
            })?
        }

        // The envelope, not the status code, decides whether this succeeded.
        let envelope = serde_json::from_str::<PostMessageEnvelope>(&body)
            .change_context(ChatError::UnreadableResponse)
            .attach_printable_lazy(|| {
                format!(
                    "chat provider returned HTTP {} with an unrecognised body: {}",
                    status.as_u16(),
                    snippet(&body, BODY_SNIPPET_CHARS)
                )
            })?;

        match envelope {
            PostMessageEnvelope::Failure { error, .. } => {
                let code = error.unwrap_or_default();
                let reason = reason_from_code(&code, retry_after_seconds);
                Err(ChatError::Rejected { reason })
                    .attach_printable_lazy(|| format!("chat provider error code: {code}"))?
            }
            PostMessageEnvelope::Success { ts, message, .. } => ts
                .or_else(|| message.and_then(|message| message.ts))
                .filter(|ts| !ts.is_empty())
                .map(MessageId::ts)
                .ok_or(ChatError::MissingMessageId)
                .attach_printable(
                    "the message was accepted; a reply cannot be threaded under it, and retrying \
                     would post a duplicate",
                ),
        }
    }

    fn build_payload(&self, message: &ChatMessage) -> ChatResult<PostMessagePayload> {
        let thread_ts = message
            .reply_target()
            .map(|message_id| {
                message_id
                    .as_ts()
                    .map(str::to_owned)
                    .ok_or(ChatError::IncompatibleReplyTarget)
            })
            .transpose()?;

        Ok(PostMessagePayload {
            channel: self.channel.clone(),
            text: truncate(message.text(), self.max_message_chars),
            thread_ts,
        })
    }
}

/// The `chat.postMessage` request body.
#[derive(Debug, Serialize)]
struct PostMessagePayload {
    channel: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<String>,
}

/// The response envelope.
///
/// `untagged` plus the [`OkTrue`] / [`OkFalse`] flags is what makes an `ok: false` body
/// *unrepresentable* as a success: the success arm cannot deserialize unless `ok` is literally
/// `true`, so `{"ok": false, ...}` falls through to the failure arm, and a body carrying no `ok`
/// at all matches neither arm and surfaces as [`ChatError::UnreadableResponse`].
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PostMessageEnvelope {
    Success {
        /// Read by serde to discriminate; never read by us.
        #[allow(dead_code)]
        ok: OkTrue,
        ts: Option<String>,
        message: Option<NestedMessage>,
    },
    Failure {
        #[allow(dead_code)]
        ok: OkFalse,
        error: Option<String>,
    },
}

/// Some responses carry the id on the echoed message rather than at the top level.
#[derive(Debug, Deserialize)]
struct NestedMessage {
    ts: Option<String>,
}

/// Deserializes only from `true`.
#[derive(Debug, Clone, Copy)]
struct OkTrue;

/// Deserializes only from `false`.
#[derive(Debug, Clone, Copy)]
struct OkFalse;

impl<'de> Deserialize<'de> for OkTrue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        expect_ok(deserializer, true).map(|()| Self)
    }
}

impl<'de> Deserialize<'de> for OkFalse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        expect_ok(deserializer, false).map(|()| Self)
    }
}

fn expect_ok<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
    expected: bool,
) -> Result<(), D::Error> {
    let found = bool::deserialize(deserializer)?;
    if found == expected {
        Ok(())
    } else {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Bool(found),
            &if expected { "true" } else { "false" },
        ))
    }
}

/// Map a provider error code onto the backend-neutral vocabulary.
///
/// Slack spells its rate-limit code both ways depending on the endpoint, and treats a deactivated
/// account the same way an operator has to treat a revoked token — re-issue it.
fn reason_from_code(code: &str, retry_after_seconds: Option<u64>) -> ChatErrorReason {
    match code {
        "channel_not_found" => ChatErrorReason::ChannelNotFound,
        "not_in_channel" | "is_archived" => ChatErrorReason::NotInChannel,
        "invalid_auth" | "not_authed" => ChatErrorReason::InvalidAuth,
        "token_revoked" | "account_inactive" => ChatErrorReason::TokenRevoked,
        "msg_too_long" => ChatErrorReason::MessageTooLong,
        "rate_limited" | "ratelimited" => ChatErrorReason::RateLimited {
            retry_after_seconds,
        },
        other => ChatErrorReason::Other(other.to_owned()),
    }
}

/// Cut `text` to `max_chars`, marking that it happened.
///
/// The API does not paginate, so oversized messages are rejected outright rather than split. This
/// is a wire-level limit and belongs here; capping the *number of items* rendered into a message
/// is the formatter's business, since this module never sees items.
fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

    let keep = max_chars.saturating_sub(TRUNCATION_MARKER.chars().count());
    let mut truncated: String = text.chars().take(keep).collect();
    truncated.push_str(TRUNCATION_MARKER);
    truncated
}

fn snippet(body: &str, max_chars: usize) -> String {
    body.chars().take(max_chars).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use serde_json::json;

    use super::*;

    fn envelope(value: serde_json::Value) -> Result<PostMessageEnvelope, serde_json::Error> {
        serde_json::from_value(value)
    }

    #[test]
    fn ok_false_cannot_deserialize_as_success() {
        let parsed = envelope(json!({"ok": false, "error": "channel_not_found"})).unwrap();
        assert!(matches!(parsed, PostMessageEnvelope::Failure { .. }));
    }

    #[test]
    fn ok_false_with_a_ts_present_is_still_a_failure() {
        // The shape that catches a status-code-only client out: everything a success has, plus
        // `ok: false`.
        let parsed = envelope(json!({"ok": false, "error": "msg_too_long", "ts": "1.2"})).unwrap();
        assert!(matches!(parsed, PostMessageEnvelope::Failure { .. }));
    }

    #[test]
    fn a_body_without_ok_is_neither_success_nor_failure() {
        assert!(envelope(json!({"ts": "1503435956.000247"})).is_err());
    }

    #[test]
    fn ok_true_is_a_success() {
        let parsed = envelope(json!({"ok": true, "ts": "1503435956.000247"})).unwrap();
        match parsed {
            PostMessageEnvelope::Success { ts, .. } => {
                assert_eq!(ts.as_deref(), Some("1503435956.000247"))
            }
            PostMessageEnvelope::Failure { .. } => panic!("expected a success"),
        }
    }

    #[test]
    fn truncate_leaves_short_text_alone() {
        assert_eq!(truncate("hello", 40), "hello");
    }

    #[test]
    fn truncate_marks_what_it_cut_and_respects_the_limit() {
        let truncated = truncate(&"a".repeat(100), 40);
        assert_eq!(truncated.chars().count(), 40);
        assert!(truncated.ends_with(TRUNCATION_MARKER));
    }

    #[test]
    fn truncate_counts_characters_not_bytes() {
        // Byte slicing here would panic mid-codepoint.
        let truncated = truncate(&"🚨".repeat(100), 20);
        assert_eq!(truncated.chars().count(), 20);
    }

    #[test]
    fn error_codes_map_onto_neutral_reasons() {
        assert_eq!(
            reason_from_code("channel_not_found", None),
            ChatErrorReason::ChannelNotFound
        );
        assert_eq!(
            reason_from_code("not_in_channel", None),
            ChatErrorReason::NotInChannel
        );
        assert_eq!(
            reason_from_code("invalid_auth", None),
            ChatErrorReason::InvalidAuth
        );
        assert_eq!(
            reason_from_code("token_revoked", None),
            ChatErrorReason::TokenRevoked
        );
        assert_eq!(
            reason_from_code("msg_too_long", None),
            ChatErrorReason::MessageTooLong
        );
        assert_eq!(
            reason_from_code("ratelimited", Some(30)),
            ChatErrorReason::RateLimited {
                retry_after_seconds: Some(30)
            }
        );
        assert_eq!(
            reason_from_code("something_new", None),
            ChatErrorReason::Other("something_new".to_owned())
        );
    }

    #[test]
    fn a_blank_destination_is_rejected_on_the_way_in() {
        let build = |base_url: &str, token: &str, channel: &str| {
            Endpoint::new(
                base_url.to_owned(),
                "/slack/",
                Secret::new(token.to_owned()),
                channel.to_owned(),
                DEFAULT_TIMEOUT_SECONDS,
                DEFAULT_MAX_MESSAGE_CHARS,
                Proxy::default(),
            )
        };

        assert!(build("", "token", "C1").is_err());
        assert!(build("not a url", "token", "C1").is_err());
        assert!(build("https://example.com", "", "C1").is_err());
        assert!(build("https://example.com", "token", "  ").is_err());
        assert!(build("https://example.com/", "token", "C1").is_ok());
    }

    #[test]
    fn method_url_namespaces_the_method_and_drops_a_trailing_slash() {
        let endpoint = Endpoint::new(
            "https://spaces.xyne.juspay.net/api/apps/".to_owned(),
            "/slack/",
            Secret::new("jwt".to_owned()),
            "C1".to_owned(),
            DEFAULT_TIMEOUT_SECONDS,
            DEFAULT_MAX_MESSAGE_CHARS,
            Proxy::default(),
        )
        .unwrap();

        assert_eq!(
            endpoint.method_url(CHAT_POST_MESSAGE),
            "https://spaces.xyne.juspay.net/api/apps/slack/chat.postMessage"
        );
    }

    #[test]
    fn the_debug_impl_does_not_print_the_token() {
        let endpoint = Endpoint::new(
            "https://example.com".to_owned(),
            "/",
            Secret::new("xoxb-super-secret".to_owned()),
            "C1".to_owned(),
            DEFAULT_TIMEOUT_SECONDS,
            DEFAULT_MAX_MESSAGE_CHARS,
            Proxy::default(),
        )
        .unwrap();

        assert!(!format!("{endpoint:?}").contains("super-secret"));
    }

    #[test]
    fn a_reply_target_from_another_backend_is_refused_before_the_wire() {
        // Today every `MessageId` is a `Ts`, so this asserts the shape rather than a live case:
        // a payload built from a `Ts` reply target carries it as `thread_ts`.
        let endpoint = Endpoint::new(
            "https://example.com".to_owned(),
            "/",
            Secret::new("token".to_owned()),
            "C1".to_owned(),
            DEFAULT_TIMEOUT_SECONDS,
            DEFAULT_MAX_MESSAGE_CHARS,
            Proxy::default(),
        )
        .unwrap();

        let payload = endpoint
            .build_payload(&ChatMessage::new("hi").reply_to(MessageId::ts("1.2")))
            .unwrap();

        assert_eq!(payload.thread_ts.as_deref(), Some("1.2"));
        assert_eq!(payload.channel, "C1");
    }
}
