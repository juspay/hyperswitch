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
use url::Url;

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
///
/// `Debug` is derived: [`Secret`] redacts itself, so the token cannot reach a log line through it.
#[derive(Clone, Debug)]
pub(super) struct Endpoint {
    /// Root of the API, e.g. `https://slack.com/api`.
    base_url: Url,

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

impl Endpoint {
    /// Validate a destination and bind it to a proxy.
    ///
    /// Validation happens here rather than at send time so a destination read from configuration
    /// or from a database row fails once, on the way in, rather than once per message.
    pub(super) fn new(
        base_url: Url,
        method_prefix: &'static str,
        token: Secret<String>,
        channel: String,
        timeout_seconds: u64,
        max_message_chars: usize,
        proxy: Proxy,
    ) -> ChatResult<Self> {
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

    /// `Url::join` is deliberately not used: it resolves relatively, so a base of `/api/apps`
    /// without a trailing slash would produce `/api/chat.postMessage`, silently dropping a path
    /// segment.
    fn method_url(&self, method: &str) -> String {
        format!(
            "{}{}{}",
            self.base_url.as_str().trim_end_matches('/'),
            self.method_prefix,
            method
        )
    }

    /// Post a message and return the id the provider assigned it.
    ///
    /// **Not idempotent, and the transport may retry.**
    /// [`http_client::send_request`] clones a JSON request and resends it once when the connection
    /// closes before the response completes, and `chat.postMessage` offers no idempotency key. A
    /// message can therefore be delivered twice. That trade is deliberate for this caller: a
    /// duplicate alert costs far less than a dropped one, and every connector call in this
    /// workspace already carries the same retry.
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

        // The body, not the status code, decides whether this succeeded.
        serde_json::from_str::<PostMessageResponse>(&body)
            .change_context(ChatError::UnreadableResponse)
            .attach_printable_lazy(|| {
                format!(
                    "chat provider returned HTTP {} with an unrecognised body: {}",
                    status.as_u16(),
                    snippet(&body, BODY_SNIPPET_CHARS)
                )
            })?
            .try_into()
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

/// The `chat.postMessage` response, exactly as it arrives.
///
/// Faithful to the wire and nothing else. `ok` is a required field, so a body that carries no
/// success marker fails to deserialize rather than being read as a success — which is the trap
/// this API sets, since it reports failure at HTTP 200.
///
/// Private, and the only way out of this module is [`MessageId`] via the conversion below, so no
/// caller can reach a `ts` without the `ok` check having run.
#[derive(Debug, Deserialize)]
struct PostMessageResponse {
    ok: bool,
    error: Option<String>,
    ts: Option<String>,
    message: Option<NestedMessage>,
}

/// Some responses carry the id on the echoed message rather than at the top level.
#[derive(Debug, Deserialize)]
struct NestedMessage {
    ts: Option<String>,
}

impl TryFrom<PostMessageResponse> for MessageId {
    type Error = error_stack::Report<ChatError>;

    fn try_from(response: PostMessageResponse) -> Result<Self, Self::Error> {
        if !response.ok {
            let code = response.error.unwrap_or_default();
            return Err(ChatError::Rejected {
                reason: reason_from_code(&code),
            })
            .attach_printable_lazy(|| format!("chat provider error code: {code}"));
        }

        response
            .ts
            .or_else(|| response.message.and_then(|message| message.ts))
            .filter(|ts| !ts.is_empty())
            .map(Self::ts)
            .ok_or(ChatError::MissingMessageId)
            .attach_printable(
                "the message was accepted; a reply cannot be threaded under it, and retrying \
                 would post a duplicate",
            )
    }
}

/// Map a provider error code onto the backend-neutral vocabulary.
///
/// Slack spells its rate-limit code both ways depending on the endpoint, and treats a deactivated
/// account the same way an operator has to treat a revoked token — re-issue it.
///
/// No `Retry-After` here: the provider sends that header with a `429`, which is handled before a
/// body is ever parsed. A rate-limit code arriving at HTTP 200 comes without one.
fn reason_from_code(code: &str) -> ChatErrorReason {
    match code {
        "channel_not_found" => ChatErrorReason::ChannelNotFound,
        "not_in_channel" | "is_archived" => ChatErrorReason::NotInChannel,
        "invalid_auth" | "not_authed" => ChatErrorReason::InvalidAuth,
        "token_revoked" | "account_inactive" => ChatErrorReason::TokenRevoked,
        "msg_too_long" => ChatErrorReason::MessageTooLong,
        "rate_limited" | "ratelimited" => ChatErrorReason::RateLimited {
            retry_after_seconds: None,
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

    // The marker is itself subject to the cap. Appending it whole to an empty remainder would
    // return something *longer* than the limit this function exists to enforce, which the provider
    // would then reject.
    let marker: String = TRUNCATION_MARKER.chars().take(max_chars).collect();
    let keep = max_chars.saturating_sub(marker.chars().count());

    let mut truncated: String = text.chars().take(keep).collect();
    truncated.push_str(&marker);
    truncated
}

fn snippet(body: &str, max_chars: usize) -> String {
    body.chars().take(max_chars).collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::unwrap_in_result,
    clippy::expect_used,
    clippy::indexing_slicing
)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Parse a body and run it through the conversion, exactly as `post_message` does.
    fn read(value: serde_json::Value) -> Result<ChatResult<MessageId>, serde_json::Error> {
        serde_json::from_value::<PostMessageResponse>(value).map(TryInto::try_into)
    }

    #[test]
    fn ok_false_is_not_a_success() {
        let error = read(json!({"ok": false, "error": "channel_not_found"}))
            .unwrap()
            .unwrap_err();
        assert!(matches!(
            error.current_context(),
            ChatError::Rejected {
                reason: ChatErrorReason::ChannelNotFound
            }
        ));
    }

    #[test]
    fn ok_false_with_a_ts_present_is_still_a_failure() {
        // The shape that catches a status-code-only client out: everything a success has, plus
        // `ok: false`.
        let error = read(json!({"ok": false, "error": "msg_too_long", "ts": "1.2"}))
            .unwrap()
            .unwrap_err();
        assert!(matches!(
            error.current_context(),
            ChatError::Rejected {
                reason: ChatErrorReason::MessageTooLong
            }
        ));
    }

    #[test]
    fn a_body_without_ok_does_not_deserialize() {
        // `ok` is required, so a body carrying a `ts` and nothing else cannot be mistaken for a
        // success.
        assert!(read(json!({"ts": "1503435956.000247"})).is_err());
    }

    #[test]
    fn ok_true_is_a_success() {
        assert_eq!(
            read(json!({"ok": true, "ts": "1503435956.000247"}))
                .unwrap()
                .unwrap(),
            MessageId::ts("1503435956.000247")
        );
    }

    #[test]
    fn ok_true_with_no_id_anywhere_is_reported_rather_than_faked() {
        let error = read(json!({"ok": true})).unwrap().unwrap_err();
        assert!(matches!(
            error.current_context(),
            ChatError::MissingMessageId
        ));
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
    fn truncate_never_exceeds_a_cap_shorter_than_its_own_marker() {
        for max_chars in 1..=TRUNCATION_MARKER.chars().count() {
            let truncated = truncate(&"a".repeat(100), max_chars);
            assert_eq!(
                truncated.chars().count(),
                max_chars,
                "cap of {max_chars} was exceeded"
            );
        }
    }

    #[test]
    fn error_codes_map_onto_neutral_reasons() {
        assert_eq!(
            reason_from_code("channel_not_found"),
            ChatErrorReason::ChannelNotFound
        );
        assert_eq!(
            reason_from_code("not_in_channel"),
            ChatErrorReason::NotInChannel
        );
        assert_eq!(
            reason_from_code("invalid_auth"),
            ChatErrorReason::InvalidAuth
        );
        assert_eq!(
            reason_from_code("token_revoked"),
            ChatErrorReason::TokenRevoked
        );
        assert_eq!(
            reason_from_code("msg_too_long"),
            ChatErrorReason::MessageTooLong
        );
        assert_eq!(
            reason_from_code("ratelimited"),
            ChatErrorReason::RateLimited {
                retry_after_seconds: None
            }
        );
        assert_eq!(
            reason_from_code("something_new"),
            ChatErrorReason::Other("something_new".to_owned())
        );
    }

    fn endpoint(base_url: &str, method_prefix: &'static str, token: &str) -> ChatResult<Endpoint> {
        Endpoint::new(
            Url::parse(base_url).unwrap(),
            method_prefix,
            Secret::new(token.to_owned()),
            "C1".to_owned(),
            DEFAULT_TIMEOUT_SECONDS,
            DEFAULT_MAX_MESSAGE_CHARS,
            Proxy::default(),
        )
    }

    #[test]
    fn an_unusable_destination_is_rejected_on_the_way_in() {
        // A malformed base URL cannot reach here at all: it is a `Url`, so it fails at
        // deserialization. What is left for `Endpoint::new` is the rest of the destination.
        assert!(endpoint("https://example.com", "/", "").is_err());
        assert!(endpoint("https://example.com", "/", "token").is_ok());

        let blank_channel = Endpoint::new(
            Url::parse("https://example.com").unwrap(),
            "/",
            Secret::new("token".to_owned()),
            "  ".to_owned(),
            DEFAULT_TIMEOUT_SECONDS,
            DEFAULT_MAX_MESSAGE_CHARS,
            Proxy::default(),
        );
        assert!(blank_channel.is_err());

        let zero_cap = Endpoint::new(
            Url::parse("https://example.com").unwrap(),
            "/",
            Secret::new("token".to_owned()),
            "C1".to_owned(),
            DEFAULT_TIMEOUT_SECONDS,
            0,
            Proxy::default(),
        );
        assert!(zero_cap.is_err());
    }

    #[test]
    fn method_url_namespaces_the_method_and_drops_a_trailing_slash() {
        assert_eq!(
            endpoint("https://spaces.xyne.juspay.net/api/apps/", "/slack/", "jwt")
                .unwrap()
                .method_url(CHAT_POST_MESSAGE),
            "https://spaces.xyne.juspay.net/api/apps/slack/chat.postMessage"
        );
    }

    #[test]
    fn the_derived_debug_does_not_print_the_token() {
        let endpoint = endpoint("https://example.com", "/", "xoxb-super-secret").unwrap();
        assert!(!format!("{endpoint:?}").contains("super-secret"));
    }

    #[test]
    fn a_reply_target_is_carried_as_thread_ts() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();

        let payload = endpoint
            .build_payload(&ChatMessage::new("hi").reply_to(MessageId::ts("1.2")))
            .unwrap();

        assert_eq!(payload.thread_ts.as_deref(), Some("1.2"));
        assert_eq!(payload.channel, "C1");
    }
}
