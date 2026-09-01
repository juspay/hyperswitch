//! Delivery of chat messages to a destination channel.
//!
//! A [`ChatClient`] is bound to one destination: a base URL, a credential and a channel. Callers
//! that hold several destinations — one per merchant workspace, say — build one client per
//! destination and hand each a [`ChatMessage`]. Clients own no connection state (the underlying
//! `reqwest::Client` and its pool are cached globally by
//! [`crate::http_client`]), so constructing one per message is cheap.
//!
//! This module deliberately knows nothing about *alerts*. It renders no domain content and holds
//! no configuration store; deciding what to say, and looking up where to say it, belong to the
//! callers.

/// Markdown formatting for chat backends that speak Slack's `mrkdwn`.
pub mod mrkdwn;

/// The Slack chat API.
pub mod slack;

/// Xyne, an internally hosted chat service exposing a Slack-compatible API.
pub mod xyne;

/// The wire protocol Xyne and Slack share. Private: nothing outside this module should have to
/// know that `ok: false` can arrive with HTTP 200.
mod slack_compatible;

use common_utils::errors::CustomResult;

/// Result type for chat operations.
pub type ChatResult<T> = CustomResult<T, ChatError>;

/// Posts messages to one chat destination.
///
/// Object-safe on purpose: a caller resolving destinations at runtime holds these as
/// `Arc<dyn ChatClient>`. Resist adding an associated type — [`crate::email::EmailClient`] has one
/// (`RichText`), which is why it cannot be used as a trait object and why the erased
/// [`crate::email::EmailService`] had to be invented alongside it.
#[async_trait::async_trait]
pub trait ChatClient: Send + Sync + std::fmt::Debug {
    /// Post a message, returning the id of the message that was created.
    async fn post_message(&self, message: ChatMessage) -> ChatResult<MessageId>;
}

/// Identifies a message that a backend has accepted.
///
/// Backends disagree on what a message id *is*, and the disagreement is not cosmetic: threading a
/// reply means handing an id back, so an id from the wrong backend is a bug we want the type
/// system to catch rather than a malformed field on the wire.
///
/// Non-exhaustive because more backends are expected (Discord identifies messages by a numeric
/// snowflake rather than a timestamp); match with a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MessageId {
    /// A Slack-compatible `ts`: `"1503435956.000247"`, seconds and microseconds since the epoch.
    ///
    /// Opaque — it must stay a string. Parsing it as a float loses precision, and Slack documents
    /// it as an identifier rather than a time.
    Ts(String),
}

impl MessageId {
    /// Build a Slack-compatible `ts` id.
    pub fn ts(value: impl Into<String>) -> Self {
        Self::Ts(value.into())
    }

    /// The `ts` string, if this id came from a Slack-compatible backend.
    pub fn as_ts(&self) -> Option<&str> {
        match self {
            Self::Ts(value) => Some(value.as_str()),
        }
    }
}

/// A message to post.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    text: String,
    reply_to: Option<MessageId>,
}

impl ChatMessage {
    /// A new top-level message.
    ///
    /// `text` is delivered as-is; use [`mrkdwn`] to format it for backends that expect Slack's
    /// markup, and [`mrkdwn::escape`] on any value interpolated into it.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            reply_to: None,
        }
    }

    /// Post this message as a reply threaded under `message_id`.
    ///
    /// Serialised as `thread_ts` by Slack-compatible backends.
    pub fn reply_to(mut self, message_id: MessageId) -> Self {
        self.reply_to = Some(message_id);
        self
    }

    /// The message body.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The message this one replies to, if any.
    pub fn reply_target(&self) -> Option<&MessageId> {
        self.reply_to.as_ref()
    }
}

/// Errors raised when posting a chat message.
#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    /// The destination could not be turned into a usable client.
    #[error("Invalid chat client configuration: {0}")]
    InvalidConfiguration(&'static str),

    /// The request never produced a response — DNS, TLS, proxy, timeout.
    #[error("Failed to send the request to the chat provider")]
    RequestFailed,

    /// The provider answered outside the 2xx range.
    #[error("Chat provider responded with HTTP status {status}")]
    HttpStatus {
        /// The status code returned.
        status: u16,
    },

    /// The response body could not be read, or was not the envelope the provider documents.
    ///
    /// A body carrying no success marker lands here rather than being read as a success.
    #[error("Could not interpret the chat provider's response")]
    UnreadableResponse,

    /// The provider accepted the request and refused the message.
    #[error("Chat provider rejected the message: {reason}")]
    Rejected {
        /// Why it was refused.
        reason: ChatErrorReason,
    },

    /// The message was delivered but the provider named no id for it, so replies cannot be
    /// threaded under it. Retrying would post a duplicate.
    #[error("Chat provider accepted the message without returning a message id")]
    MissingMessageId,

    /// [`ChatMessage::reply_to`] carried an id this backend cannot thread against — typically an
    /// id minted by a different backend.
    #[error("The message id supplied cannot thread a reply on this chat provider")]
    IncompatibleReplyTarget,
}

/// Why a provider refused a message, in vocabulary no single backend owns.
///
/// Backends map their own codes into this; the code they actually sent is preserved either in
/// [`ChatErrorReason::Other`] or on the error report's attachments, so nothing is lost for logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatErrorReason {
    /// No such channel, or the credential cannot see it.
    ChannelNotFound,

    /// The channel exists but the bot is not a member of it.
    NotInChannel,

    /// The credential was not accepted.
    InvalidAuth,

    /// The credential was valid and has since been revoked or deactivated; it needs re-issuing.
    TokenRevoked,

    /// The message exceeded the provider's size limit.
    MessageTooLong,

    /// The caller is posting too fast.
    RateLimited {
        /// How long the provider asked us to wait, when it said.
        retry_after_seconds: Option<u64>,
    },

    /// Anything else, carrying the provider's own code verbatim.
    Other(String),
}

impl std::fmt::Display for ChatErrorReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChannelNotFound => f.write_str("channel not found"),
            Self::NotInChannel => f.write_str("not a member of the channel"),
            Self::InvalidAuth => f.write_str("credential rejected"),
            Self::TokenRevoked => f.write_str("credential revoked"),
            Self::MessageTooLong => f.write_str("message too long"),
            Self::RateLimited {
                retry_after_seconds: Some(seconds),
            } => write!(f, "rate limited, retry after {seconds}s"),
            Self::RateLimited {
                retry_after_seconds: None,
            } => f.write_str("rate limited"),
            Self::Other(code) => write!(f, "{code}"),
        }
    }
}
