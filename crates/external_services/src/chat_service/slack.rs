//! The Slack chat API.
//!
//! Slack and [`super::xyne`] speak the same protocol — Xyne is a facade over this one — so the
//! request, the envelope and the error vocabulary are shared through
//! [`super::slack_compatible`]. Two things are Slack's own: methods sit directly under the API
//! root rather than in a namespace, and the credential is a bot token (`xoxb-…`) rather than an
//! app JWT.
//!
//! This exists because it is nearly free, not because it is on the critical path: Xyne is the
//! destination in use today.

use hyperswitch_interfaces::types::Proxy;
use hyperswitch_masking::Secret;
use serde::Deserialize;

use super::{
    slack_compatible::{Endpoint, DEFAULT_MAX_MESSAGE_CHARS, DEFAULT_TIMEOUT_SECONDS},
    ChatClient, ChatMessage, ChatResult, MessageId,
};

/// Slack serves methods directly off its API root.
const METHOD_PREFIX: &str = "/";

/// Slack's public API root.
const DEFAULT_BASE_URL: &str = "https://slack.com/api";

fn default_base_url() -> String {
    DEFAULT_BASE_URL.to_owned()
}

fn default_timeout_seconds() -> u64 {
    DEFAULT_TIMEOUT_SECONDS
}

fn default_max_message_chars() -> usize {
    DEFAULT_MAX_MESSAGE_CHARS
}

/// One Slack destination: a workspace and a channel within it.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackConfig {
    /// Root of the Slack API. Defaults to the public one.
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// The bot token (`xoxb-…`), presented as `Authorization: Bearer <token>`.
    pub bot_token: Secret<String>,

    /// Where messages go. A channel id is preferred; a channel name is accepted.
    pub channel: String,

    /// How long to wait for a response.
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,

    /// Longest message body Slack will accept, in characters.
    #[serde(default = "default_max_message_chars")]
    pub max_message_chars: usize,
}

/// Posts messages to one Slack channel.
#[derive(Debug, Clone)]
pub struct SlackClient {
    endpoint: Endpoint,
}

impl SlackClient {
    /// Build a client for one destination, rejecting a destination that cannot work.
    pub fn new(config: SlackConfig, proxy: Proxy) -> ChatResult<Self> {
        Ok(Self {
            endpoint: Endpoint::new(
                config.base_url,
                METHOD_PREFIX,
                config.bot_token,
                config.channel,
                config.timeout_seconds,
                config.max_message_chars,
                proxy,
            )?,
        })
    }
}

#[async_trait::async_trait]
impl ChatClient for SlackClient {
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

    /// The one thing that differs from Xyne: no `/slack/` namespace in front of the method.
    #[tokio::test]
    async fn methods_sit_directly_under_the_api_root() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat.postMessage"))
            .and(header("authorization", "Bearer xoxb-test"))
            .and(body_json(json!({"channel": "C1", "text": "hello"})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"ok": true, "ts": "1.1"})),
            )
            .mount(&server)
            .await;

        let client = SlackClient::new(
            SlackConfig {
                base_url: server.uri(),
                bot_token: Secret::new("xoxb-test".to_owned()),
                channel: "C1".to_owned(),
                timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
                max_message_chars: DEFAULT_MAX_MESSAGE_CHARS,
            },
            Proxy::default(),
        )
        .unwrap();

        assert_eq!(
            client
                .post_message(ChatMessage::new("hello"))
                .await
                .unwrap(),
            MessageId::ts("1.1")
        );
    }

    #[test]
    fn config_defaults_to_the_public_slack_api() {
        let config: SlackConfig =
            serde_json::from_value(json!({"bot_token": "xoxb-x", "channel": "C1"})).unwrap();

        assert_eq!(config.base_url, DEFAULT_BASE_URL);
    }
}
