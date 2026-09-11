//! Delivery of chat messages to a destination channel.

/// The Slack chat API.
pub mod slack;

/// Xyne, which exposes a Slack-compatible messaging API.
pub mod xyne;

/// The wire protocol Xyne and Slack share.
mod slack_compatible;

use common_utils::errors::CustomResult;
use hyperswitch_masking::{PeekInterface, Secret};

/// Result type for chat operations.
pub type ChatResult<T> = CustomResult<T, ChatError>;

/// Posts messages to one chat destination.
#[async_trait::async_trait]
pub trait ChatClient: Send + Sync + std::fmt::Debug {
    /// Post a message, returning the id of the message that was created.
    async fn post_message(&self, message: ChatMessage) -> ChatResult<MessageId>;

    /// Upload a file and optionally share it under an existing message.
    async fn upload_file(&self, file: ChatFile) -> ChatResult<FileId>;
}

/// Identifies a file that a backend has accepted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileId {
    /// An opaque identifier returned by a file upload API.
    Identifier(String),
}

impl FileId {
    /// Build an opaque file identifier.
    pub fn identifier(value: impl Into<String>) -> Self {
        Self::Identifier(value.into())
    }

    /// The provider's opaque file identifier.
    pub fn as_identifier(&self) -> Option<&str> {
        match self {
            Self::Identifier(value) => Some(value.as_str()),
        }
    }
}

/// A file to upload to a chat destination.
#[derive(Debug, Clone)]
pub struct ChatFile {
    bytes: Secret<Vec<u8>>,
    filename: Secret<String>,
    title: Option<Secret<String>>,
    comment: Option<Secret<String>>,
    reply_to: Option<MessageId>,
}

impl ChatFile {
    /// Build one upload.
    pub fn new(
        bytes: Vec<u8>,
        filename: impl Into<String>,
        title: Option<String>,
        comment: Option<String>,
        reply_to: Option<MessageId>,
    ) -> Self {
        Self {
            bytes: Secret::new(bytes),
            filename: Secret::new(filename.into()),
            title: title.map(Secret::new),
            comment: comment.map(Secret::new),
            reply_to,
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        self.bytes.peek()
    }

    pub(crate) fn filename(&self) -> &str {
        self.filename.peek()
    }

    pub(crate) fn title(&self) -> Option<&str> {
        self.title
            .as_ref()
            .map(PeekInterface::peek)
            .map(String::as_str)
    }

    pub(crate) fn comment(&self) -> Option<&str> {
        self.comment
            .as_ref()
            .map(PeekInterface::peek)
            .map(String::as_str)
    }

    pub(crate) fn reply_target(&self) -> Option<&MessageId> {
        self.reply_to.as_ref()
    }
}

/// Identifies a message that a backend has accepted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MessageId {
    /// A Slack-compatible `ts`:
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
    banner: Option<ChatBanner>,
}

/// How urgent a message is, in the vocabulary of the reader rather than of any one backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatSeverity {
    /// Something is broken now.
    Critical,
    /// Still broken, said again.
    Warning,
    /// Over.
    Resolved,
}

/// A titled, colour-coded frame around a message.
#[derive(Debug, Clone)]
pub struct ChatBanner {
    heading: String,
    severity: ChatSeverity,
}

impl ChatBanner {
    /// A banner reading `heading`, weighted by `severity`.
    pub fn new(heading: impl Into<String>, severity: ChatSeverity) -> Self {
        Self {
            heading: heading.into(),
            severity,
        }
    }

    /// The heading, rendered as plain text:
    pub fn heading(&self) -> &str {
        &self.heading
    }

    /// How urgent the message is.
    pub fn severity(&self) -> ChatSeverity {
        self.severity
    }
}

impl ChatMessage {
    /// A new top-level message.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            reply_to: None,
            banner: None,
        }
    }

    /// A message threaded as a reply under `message_id`.
    pub fn reply(text: impl Into<String>, message_id: MessageId) -> Self {
        Self {
            text: text.into(),
            reply_to: Some(message_id),
            banner: None,
        }
    }

    /// Frame this message in a titled, colour-coded banner.
    pub fn with_banner(mut self, banner: ChatBanner) -> Self {
        self.banner = Some(banner);
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

    /// The banner to frame this message in, if any.
    pub fn banner(&self) -> Option<&ChatBanner> {
        self.banner.as_ref()
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
    #[error("Could not interpret the chat provider's response")]
    UnreadableResponse,

    /// The provider accepted the request and refused the message.
    #[error("Chat provider rejected the message: {reason}")]
    Rejected {
        /// Why it was refused.
        reason: ChatErrorReason,
    },

    /// The message was delivered but the provider named no id for it, so replies cannot be threaded under it.
    #[error("Chat provider accepted the message without returning a message id")]
    MissingMessageId,

    /// [`ChatMessage::reply`] carried an id this backend cannot thread against — typically an id minted by a different backend.
    #[error("The message id supplied cannot thread a reply on this chat provider")]
    IncompatibleReplyTarget,

    /// The provider accepted an upload but did not identify the resulting file.
    #[error("Chat provider accepted the upload without returning a file id")]
    MissingFileId,
}

/// Why a provider refused a message, in vocabulary no single backend owns.
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
