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

    /// Upload a file, returning the id the backend stored it under.
    ///
    /// A sibling of [`ChatClient::post_message`] rather than a field on [`ChatMessage`], because
    /// it is a different endpoint with a different result. It returns a [`FileId`] and **not** a
    /// [`MessageId`]: the upload creates a message, but the backends do not agree that you may
    /// know its id, so nothing can be threaded under an upload.
    async fn upload_file(&self, file: ChatFile) -> ChatResult<FileId>;
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

/// Identifies a file a backend has stored.
///
/// Deliberately *not* a [`MessageId`]. Uploading produces a file, and the message carrying it is a
/// side effect the backend does not always name — Xyne's `files.completeUploadExternal` returns no
/// `ts` at all. Conflating the two would promise threading that the upload path cannot deliver.
///
/// An enum for the same reason [`MessageId`] is one, though the case is weaker and worth stating
/// honestly: nothing hands a file id *back* to a backend today, so there is no round trip for a
/// mismatched id to break. What it buys is that the first backend whose file id is not a bare
/// string — or is not interchangeable with a Slack-compatible one — adds a variant instead of
/// forcing this type open. Non-exhaustive, so match with a wildcard arm.
///
/// The value is also not the id the upload was started with: a backend is free to remap it between
/// reserving an upload and completing it, and Xyne does.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileId {
    /// An id minted by a Slack-compatible backend: opaque, and only meaningful to the backend that
    /// issued it. Slack spells these `F...`; Xyne returns its own store id.
    SlackCompatible(String),
}

impl FileId {
    /// Build an id from a Slack-compatible backend.
    pub fn slack_compatible(value: impl Into<String>) -> Self {
        Self::SlackCompatible(value.into())
    }

    /// The id as the backend spelled it, if it came from a Slack-compatible backend.
    pub fn as_slack_compatible(&self) -> Option<&str> {
        match self {
            Self::SlackCompatible(value) => Some(value.as_str()),
        }
    }
}

/// A file to upload, and how it should appear when it lands.
///
/// Fields are private and reached through a constructor and chained setters, matching
/// [`ChatMessage`]. Unlike a message, most of what can be said about a file is optional, so this
/// one takes the setters: four optional fields would otherwise be sixteen constructors.
#[derive(Clone)]
pub struct ChatFile {
    filename: String,
    content_type: Option<String>,
    bytes: Vec<u8>,
    title: Option<String>,
    comment: Option<String>,
    reply_to: Option<MessageId>,
}

impl ChatFile {
    /// A file to upload, named and with its contents.
    pub fn new(filename: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            filename: filename.into(),
            content_type: None,
            bytes,
            title: None,
            comment: None,
            reply_to: None,
        }
    }

    /// Declare the file's media type. Without one the backend infers from the filename.
    #[must_use]
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// A display title. Xyne echoes it and stores the filename regardless; Slack honours it.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Text posted alongside the file, in the destination's markup.
    #[must_use]
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Land the file inside the thread of an existing message.
    #[must_use]
    pub fn reply_under(mut self, message_id: MessageId) -> Self {
        self.reply_to = Some(message_id);
        self
    }

    /// The filename the backend should store.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// The declared media type, if any.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// The file's contents.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// How many bytes there are. Backends want this before they will accept the upload.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether there is nothing to upload. Backends refuse an empty file.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The display title, if one was set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The accompanying text, if any.
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    /// The message this file lands under, if any.
    pub fn reply_target(&self) -> Option<&MessageId> {
        self.reply_to.as_ref()
    }
}

/// Hand-written so the contents cannot reach a log line.
///
/// The one place in this module where `Debug` is not derived. An alert report is a rendered
/// picture of merchant ids and payment volumes, and `error-stack` prints `Debug` for every value it
/// attaches, so a derive here would put the whole file into the log stream the first time an upload
/// failed. Sizes are what diagnosis needs and they are all that appears.
impl std::fmt::Debug for ChatFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatFile")
            .field("filename", &self.filename)
            .field("content_type", &self.content_type)
            .field("bytes", &format_args!("{} bytes", self.bytes.len()))
            .field("title", &self.title)
            .field("comment", &self.comment.as_ref().map(|_| "<set>"))
            .field("reply_to", &self.reply_to)
            .finish()
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

    /// The upload completed but the provider named no id for the stored file.
    ///
    /// The file is up. Retrying would store it twice, so this is reported apart from the failures.
    #[error("Chat provider stored the file without returning a file id")]
    MissingFileId,

    /// The provider's upload URL pointed somewhere other than the provider.
    ///
    /// The multi-step upload takes a URL out of a response body and then sends the credential to
    /// it. A URL on another origin is refused rather than followed: a compromised or spoofed
    /// response would otherwise be handed a working bot token.
    #[error("Chat provider returned an upload URL on an unexpected origin")]
    UntrustedUploadUrl,

    /// The provider rejected the bytes at the upload URL, or never took them.
    #[error("Chat provider did not accept the file contents")]
    UploadRejected,
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// The one type here whose `Debug` is hand-written, and the reason it is: `error-stack` prints
    /// `Debug` for everything attached to a report, so a derive would put an entire alert report
    /// into the log stream the first time an upload failed.
    #[test]
    fn debug_never_prints_a_file_or_its_comment() {
        let file = ChatFile::new("sr-report.pdf", b"4201 of 5000 payments lost".to_vec())
            .with_comment("merchant_1234 is not converting");

        let rendered = format!("{file:?}");

        assert!(!rendered.contains("payments lost"));
        assert!(!rendered.contains("merchant_1234"));
        // What diagnosis actually needs survives.
        assert!(rendered.contains("26 bytes"));
        assert!(rendered.contains("sr-report.pdf"));
    }

    /// A file id is not a message id, and the type system is what keeps a caller from threading
    /// under an upload — which the current protocol cannot express.
    #[test]
    fn a_file_id_is_opaque_and_round_trips() {
        assert_eq!(
            FileId::slack_compatible("cmtmsn9c1").as_slack_compatible(),
            Some("cmtmsn9c1")
        );
    }

    #[test]
    fn a_file_carries_only_what_was_set() {
        let bare = ChatFile::new("report.png", vec![1, 2, 3]);
        assert_eq!(bare.filename(), "report.png");
        assert_eq!(bare.len(), 3);
        assert!(!bare.is_empty());
        assert!(bare.content_type().is_none());
        assert!(bare.title().is_none());
        assert!(bare.comment().is_none());
        assert!(bare.reply_target().is_none());

        let dressed = ChatFile::new("report.png", vec![1])
            .with_content_type("image/png")
            .with_title("SR drop chart")
            .with_comment("detail attached")
            .reply_under(MessageId::ts("1.2"));

        assert_eq!(dressed.content_type(), Some("image/png"));
        assert_eq!(dressed.title(), Some("SR drop chart"));
        assert_eq!(dressed.comment(), Some("detail attached"));
        assert_eq!(dressed.reply_target(), Some(&MessageId::ts("1.2")));
    }

    #[test]
    fn an_empty_file_reports_itself_as_empty() {
        assert!(ChatFile::new("empty.pdf", Vec::new()).is_empty());
    }
}
