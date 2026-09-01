//! Xyne, an internally hosted chat service.
//!
//! Xyne fronts a Slack-compatible API — `{base}/slack/chat.postMessage`, the `{ok, error, ts}`
//! envelope, `thread_ts` threading — so the wire work is shared with [`super::slack`] and lives in
//! [`super::slack_compatible`]. What is Xyne's own is the path namespace and the credential: a JWT
//! issued to the app, presented as a bearer token. It is **not** a Slack `xoxb-` token, and the
//! two are not interchangeable despite the shared protocol.

use hyperswitch_interfaces::types::Proxy;
use hyperswitch_masking::Secret;
use serde::Deserialize;

use super::{
    slack_compatible::{Endpoint, DEFAULT_MAX_MESSAGE_CHARS, DEFAULT_TIMEOUT_SECONDS},
    ChatClient, ChatMessage, ChatResult, MessageId,
};

/// Xyne namespaces the Slack-compatible methods it proxies.
const METHOD_PREFIX: &str = "/slack/";

/// Where Xyne is served in production.
const DEFAULT_BASE_URL: &str = "https://spaces.xyne.juspay.net/api/apps";

fn default_base_url() -> String {
    DEFAULT_BASE_URL.to_owned()
}

fn default_timeout_seconds() -> u64 {
    DEFAULT_TIMEOUT_SECONDS
}

fn default_max_message_chars() -> usize {
    DEFAULT_MAX_MESSAGE_CHARS
}

/// One Xyne destination: a workspace and a channel within it.
///
/// `Deserialize`, so it can be read from a settings file or built from a stored record without a
/// second representation in between.
#[derive(Debug, Clone, Deserialize)]
pub struct XyneConfig {
    /// Root of the Xyne app API. Defaults to production.
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// The app JWT, presented as `Authorization: Bearer <jwt>`.
    pub app_jwt: Secret<String>,

    /// Where messages go. A channel id is preferred; a channel name is accepted.
    pub channel: String,

    /// How long to wait for a response.
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,

    /// Longest message body Xyne will accept, in characters.
    #[serde(default = "default_max_message_chars")]
    pub max_message_chars: usize,
}

/// Posts messages to one Xyne channel.
#[derive(Debug, Clone)]
pub struct XyneClient {
    endpoint: Endpoint,
}

impl XyneClient {
    /// Build a client for one destination, rejecting a destination that cannot work.
    ///
    /// `proxy` is a deployment fact rather than a property of the destination — Xyne is not always
    /// directly reachable from the pod — which is why it is passed alongside the config rather
    /// than carried inside it.
    pub fn new(config: XyneConfig, proxy: Proxy) -> ChatResult<Self> {
        Ok(Self {
            endpoint: Endpoint::new(
                config.base_url,
                METHOD_PREFIX,
                config.app_jwt,
                config.channel,
                config.timeout_seconds,
                config.max_message_chars,
                proxy,
            )?,
        })
    }
}

#[async_trait::async_trait]
impl ChatClient for XyneClient {
    async fn post_message(&self, message: ChatMessage) -> ChatResult<MessageId> {
        self.endpoint.post_message(message).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use serde_json::json;
    use wiremock::{
        matchers::{body_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;
    use crate::chat_service::{ChatError, ChatErrorReason};

    const TOKEN: &str = "test-jwt";
    const CHANNEL: &str = "C0123456789";
    const METHOD_PATH: &str = "/api/apps/slack/chat.postMessage";

    fn client_for(server: &MockServer) -> XyneClient {
        XyneClient::new(
            XyneConfig {
                base_url: format!("{}/api/apps", server.uri()),
                app_jwt: Secret::new(TOKEN.to_owned()),
                channel: CHANNEL.to_owned(),
                timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
                max_message_chars: DEFAULT_MAX_MESSAGE_CHARS,
            },
            Proxy::default(),
        )
        .unwrap()
    }

    async fn mount(server: &MockServer, status: u16, body: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path(METHOD_PATH))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn posts_the_documented_body_with_a_bearer_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(METHOD_PATH))
            .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
            .and(header("content-type", "application/json"))
            .and(body_json(json!({"channel": CHANNEL, "text": "hello"})))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"ok": true, "ts": "1503435956.000247"})),
            )
            .mount(&server)
            .await;

        let message_id = client_for(&server)
            .post_message(ChatMessage::new("hello"))
            .await
            .unwrap();

        assert_eq!(message_id, MessageId::ts("1503435956.000247"));
    }

    #[tokio::test]
    async fn a_reply_carries_thread_ts_and_nothing_else_changes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(METHOD_PATH))
            .and(body_json(
                json!({"channel": CHANNEL, "text": "recovered", "thread_ts": "1503435956.000247"}),
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"ok": true, "ts": "1503435999.000111"})),
            )
            .mount(&server)
            .await;

        let message_id = client_for(&server)
            .post_message(
                ChatMessage::new("recovered").reply_to(MessageId::ts("1503435956.000247")),
            )
            .await
            .unwrap();

        assert_eq!(message_id, MessageId::ts("1503435999.000111"));
    }

    #[tokio::test]
    async fn reads_the_id_off_the_nested_message_when_the_top_level_omits_it() {
        let server = MockServer::start().await;
        mount(
            &server,
            200,
            json!({"ok": true, "message": {"ts": "1503435956.000247"}}),
        )
        .await;

        assert_eq!(
            client_for(&server)
                .post_message(ChatMessage::new("hello"))
                .await
                .unwrap(),
            MessageId::ts("1503435956.000247")
        );
    }

    /// Failure mode one: the status code says no.
    #[tokio::test]
    async fn a_non_2xx_status_is_a_failure() {
        let server = MockServer::start().await;
        mount(&server, 502, json!({"detail": "upstream is down"})).await;

        let error = client_for(&server)
            .post_message(ChatMessage::new("hello"))
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ChatError::HttpStatus { status: 502 }
        ));
    }

    /// Failure mode two, and the one a status-code-only client reports as a success.
    #[tokio::test]
    async fn ok_false_at_http_200_is_a_failure() {
        let server = MockServer::start().await;
        mount(
            &server,
            200,
            json!({"ok": false, "error": "channel_not_found"}),
        )
        .await;

        let error = client_for(&server)
            .post_message(ChatMessage::new("hello"))
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ChatError::Rejected {
                reason: ChatErrorReason::ChannelNotFound
            }
        ));
    }

    #[tokio::test]
    async fn each_distinguished_error_code_arrives_as_its_own_reason() {
        for (code, expected) in [
            ("not_in_channel", ChatErrorReason::NotInChannel),
            ("invalid_auth", ChatErrorReason::InvalidAuth),
            ("token_revoked", ChatErrorReason::TokenRevoked),
            ("msg_too_long", ChatErrorReason::MessageTooLong),
            (
                "rate_limited",
                ChatErrorReason::RateLimited {
                    retry_after_seconds: None,
                },
            ),
        ] {
            let server = MockServer::start().await;
            mount(&server, 200, json!({"ok": false, "error": code})).await;

            let error = client_for(&server)
                .post_message(ChatMessage::new("hello"))
                .await
                .unwrap_err();

            match error.current_context() {
                ChatError::Rejected { reason } => assert_eq!(reason, &expected, "for code {code}"),
                other => panic!("expected a rejection for {code}, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn a_429_carries_the_retry_after_the_provider_asked_for() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(METHOD_PATH))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "30")
                    .set_body_json(json!({"ok": false, "error": "ratelimited"})),
            )
            .mount(&server)
            .await;

        let error = client_for(&server)
            .post_message(ChatMessage::new("hello"))
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ChatError::Rejected {
                reason: ChatErrorReason::RateLimited {
                    retry_after_seconds: Some(30)
                }
            }
        ));
    }

    #[tokio::test]
    async fn a_body_with_no_ok_marker_is_not_read_as_a_success() {
        let server = MockServer::start().await;
        mount(&server, 200, json!({"ts": "1503435956.000247"})).await;

        let error = client_for(&server)
            .post_message(ChatMessage::new("hello"))
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ChatError::UnreadableResponse
        ));
    }

    #[tokio::test]
    async fn an_accepted_message_with_no_id_is_reported_as_such() {
        let server = MockServer::start().await;
        mount(&server, 200, json!({"ok": true})).await;

        let error = client_for(&server)
            .post_message(ChatMessage::new("hello"))
            .await
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ChatError::MissingMessageId
        ));
    }

    #[tokio::test]
    async fn an_oversized_message_is_cut_down_rather_than_sent_whole() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(METHOD_PATH))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"ok": true, "ts": "1.1"})),
            )
            .mount(&server)
            .await;

        let client = XyneClient::new(
            XyneConfig {
                base_url: format!("{}/api/apps", server.uri()),
                app_jwt: Secret::new(TOKEN.to_owned()),
                channel: CHANNEL.to_owned(),
                timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
                max_message_chars: 50,
            },
            Proxy::default(),
        )
        .unwrap();

        client
            .post_message(ChatMessage::new("x".repeat(500)))
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        let text = body["text"].as_str().unwrap();
        assert_eq!(text.chars().count(), 50);
        assert!(text.ends_with("(truncated)"));
    }

    #[test]
    fn config_defaults_to_production_xyne() {
        let config: XyneConfig =
            serde_json::from_value(json!({"app_jwt": "jwt", "channel": "C1"})).unwrap();

        assert_eq!(config.base_url, DEFAULT_BASE_URL);
        assert_eq!(config.timeout_seconds, DEFAULT_TIMEOUT_SECONDS);
        assert_eq!(config.max_message_chars, DEFAULT_MAX_MESSAGE_CHARS);
    }
}
