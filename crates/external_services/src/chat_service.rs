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

/// The Slack chat API.
pub mod slack;

/// Xyne, which exposes a Slack-compatible messaging API.
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
///
/// The fields are private and reached through constructors and accessors on purpose: this is the
/// type most likely to grow, and private fields mean it can grow without breaking callers.
///
/// Two directions it is expected to grow, and how:
///
/// - **A backend that is not Slack-compatible.** `text` is markup, and the markup is not portable
///   — Slack reads `*bold*` where Discord reads `**bold**`. Today every backend is
///   Slack-compatible so a rendered string is honest. The first backend that is not forces a
///   choice between rendering at the call site and carrying structured content here; keeping
///   `text` private means that choice stays open.
/// - **Files.** They do not belong on this type. Uploading is a different endpoint with a
///   different result — a file id, not a message id — and on current Slack it is three calls
///   rather than one. It earns a sibling method on [`ChatClient`], not a field here.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    text: String,
    reply_to: Option<MessageId>,
}

impl ChatMessage {
    /// A new top-level message.
    ///
    /// `text` is delivered as-is, in whatever markup the target backend reads. Escape anything
    /// interpolated into it: on Slack-compatible backends an unescaped `<` in a merchant id or an
    /// error reason opens markup and mangles the message.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            reply_to: None,
        }
    }

    /// A message threaded as a reply under `message_id`.
    ///
    /// Serialised as `thread_ts` by Slack-compatible backends.
    ///
    /// A second constructor rather than a `mut self` builder on top of [`ChatMessage::new`]:
    /// whether a message is threaded is known at the call site, so it is an argument rather than a
    /// state a value passes through.
    pub fn reply(text: impl Into<String>, message_id: MessageId) -> Self {
        Self {
            text: text.into(),
            reply_to: Some(message_id),
        }
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

    /// [`ChatMessage::reply`] carried an id this backend cannot thread against — typically an
    /// id minted by a different backend.
    #[error("The message id supplied cannot thread a reply on this chat provider")]
    IncompatibleReplyTarget,
}

/// Why a provider refused a message, in vocabulary no single backend owns.
///
/// Backends map their own codes into this; the code they actually sent is preserved either in
/// [`ChatErrorReason::Other`] or on the error report's attachments, so nothing is lost for logs.
/// `Display` is what [`ChatError::Rejected`] interpolates as `{reason}`, so each message lives on
/// the variant it describes rather than in a separate impl that can drift from it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChatErrorReason {
    /// No such channel, or the credential cannot see it.
    #[error("channel not found")]
    ChannelNotFound,

    /// The channel exists but the bot is not a member of it.
    #[error("not a member of the channel")]
    NotInChannel,

    /// The credential was not accepted.
    #[error("credential rejected")]
    InvalidAuth,

    /// The credential was valid and has since been revoked or deactivated; it needs re-issuing.
    #[error("credential revoked")]
    TokenRevoked,

    /// The message exceeded the provider's size limit.
    #[error("message too long")]
    MessageTooLong,

    /// The caller is posting too fast.
    #[error("rate limited{}", retry_after_seconds.map_or_else(String::new, |seconds| format!(", retry after {seconds}s")))]
    RateLimited {
        /// How long the provider asked us to wait, when it said.
        retry_after_seconds: Option<u64>,
    },

    /// Anything else, carrying the provider's own code verbatim.
    #[error("{0}")]
    Other(String),
}
