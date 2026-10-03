pub mod slack;

pub mod xyne;

mod slack_compatible;

use common_utils::errors::CustomResult;
use hyperswitch_masking::{PeekInterface, Secret};

pub type ChatResult<T> = CustomResult<T, ChatError>;

#[async_trait::async_trait]
pub trait ChatClient: Send + Sync + std::fmt::Debug {
    async fn post_message(&self, message: ChatMessage) -> ChatResult<MessageId>;

    async fn upload_file(&self, file: ChatFile) -> ChatResult<FileId>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileId {
    Identifier(String),
}

impl FileId {
    pub fn identifier(value: impl Into<String>) -> Self {
        Self::Identifier(value.into())
    }

    pub fn as_identifier(&self) -> Option<&str> {
        match self {
            Self::Identifier(value) => Some(value.as_str()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatFile {
    bytes: Secret<Vec<u8>>,
    filename: Secret<String>,
    title: Option<Secret<String>>,
    comment: Option<Secret<String>>,
    reply_to: Option<MessageId>,
}

impl ChatFile {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MessageId {
    Ts(String),
}

impl MessageId {
    pub fn ts(value: impl Into<String>) -> Self {
        Self::Ts(value.into())
    }

    pub fn as_ts(&self) -> Option<&str> {
        match self {
            Self::Ts(value) => Some(value.as_str()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    text: String,
    reply_to: Option<MessageId>,
    banner: Option<ChatBanner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatSeverity {
    Critical,
    Warning,
    Resolved,
}

#[derive(Debug, Clone)]
pub struct ChatBanner {
    heading: String,
    severity: ChatSeverity,
}

impl ChatBanner {
    pub fn new(heading: impl Into<String>, severity: ChatSeverity) -> Self {
        Self {
            heading: heading.into(),
            severity,
        }
    }

    pub fn heading(&self) -> &str {
        &self.heading
    }

    pub fn severity(&self) -> ChatSeverity {
        self.severity
    }
}

impl ChatMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            reply_to: None,
            banner: None,
        }
    }

    pub fn reply(text: impl Into<String>, message_id: MessageId) -> Self {
        Self {
            text: text.into(),
            reply_to: Some(message_id),
            banner: None,
        }
    }

    pub fn with_banner(mut self, banner: ChatBanner) -> Self {
        self.banner = Some(banner);
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn reply_target(&self) -> Option<&MessageId> {
        self.reply_to.as_ref()
    }

    pub fn banner(&self) -> Option<&ChatBanner> {
        self.banner.as_ref()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("Invalid chat client configuration: {0}")]
    InvalidConfiguration(&'static str),

    #[error("Failed to send the request to the chat provider")]
    RequestFailed,

    #[error("Chat provider responded with HTTP status {status}")]
    HttpStatus { status: u16 },

    #[error("Could not interpret the chat provider's response")]
    UnreadableResponse,

    #[error("Chat provider rejected the message: {reason}")]
    Rejected { reason: ChatErrorReason },

    #[error("Chat provider accepted the message without returning a message id")]
    MissingMessageId,

    #[error("The message id supplied cannot thread a reply on this chat provider")]
    IncompatibleReplyTarget,

    #[error("Chat provider accepted the upload without returning a file id")]
    MissingFileId,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChatErrorReason {
    #[error("channel not found")]
    ChannelNotFound,

    #[error("not a member of the channel")]
    NotInChannel,

    #[error("credential rejected")]
    InvalidAuth,

    #[error("credential revoked")]
    TokenRevoked,

    #[error("message too long")]
    MessageTooLong,

    #[error("rate limited{}", retry_after_seconds.map_or_else(String::new, |seconds| format!(", retry after {seconds}s")))]
    RateLimited { retry_after_seconds: Option<u64> },

    #[error("{0}")]
    Other(String),
}
