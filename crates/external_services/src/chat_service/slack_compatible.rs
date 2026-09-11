use common_utils::request::{Method, RequestBuilder, RequestContent};
use error_stack::ResultExt;
use hyperswitch_interfaces::types::Proxy;
use hyperswitch_masking::Maskable;
use router_env::logger;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{
    ChatError, ChatErrorReason, ChatFile, ChatMessage, ChatResult, ChatSeverity, FileId, MessageId,
};
use crate::http_client;

const CHAT_POST_MESSAGE: &str = "chat.postMessage";
const FILES_GET_UPLOAD_URL: &str = "files.getUploadURLExternal";
const FILES_COMPLETE_UPLOAD: &str = "files.completeUploadExternal";

pub(super) const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

const TRUNCATION_MARKER: &str = "\n…(truncated)";

const BODY_SNIPPET_CHARS: usize = 512;

const UNSPECIFIED_ERROR_CODE: &str = "unspecified";

const HEADER_MAX_CHARS: usize = 150;

const SECTION_MAX_CHARS: usize = 3_000;

#[derive(Clone, Debug)]
pub(super) struct EndpointHeaders {
    api: Vec<(String, Maskable<String>)>,
    upload: Vec<(String, Maskable<String>)>,
}

impl EndpointHeaders {
    pub(super) fn new(
        api: Vec<(String, Maskable<String>)>,
        upload: Vec<(String, Maskable<String>)>,
    ) -> Self {
        Self { api, upload }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Endpoint {
    base_url: Url,

    method_prefix: &'static str,

    headers: EndpointHeaders,

    channel: String,

    timeout_seconds: u64,
    max_message_chars: usize,
    proxy: Proxy,
}

impl Endpoint {
    pub(super) fn new(
        base_url: Url,
        method_prefix: &'static str,
        headers: EndpointHeaders,
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

        if max_message_chars == 0 {
            Err(ChatError::InvalidConfiguration(
                "max message length must be greater than zero",
            ))?
        }

        Ok(Self {
            base_url,
            method_prefix,
            headers,
            channel,
            timeout_seconds,
            max_message_chars,
            proxy,
        })
    }

    fn method_url(&self, method: &str) -> String {
        format!(
            "{}{}{}",
            self.base_url.as_str().trim_end_matches('/'),
            self.method_prefix,
            method
        )
    }

    pub(super) async fn post_message(&self, message: ChatMessage) -> ChatResult<MessageId> {
        let payload = self.build_payload(&message)?;
        let url = self.method_url(CHAT_POST_MESSAGE);

        logger::info!(
            tag = "chat_post_message",
            url = %url,
            channel = %payload.channel,
            threaded = payload.thread_ts.is_some(),
            bannered = payload.attachments.is_some(),
            chars = payload.body_chars(),
        );

        let body = self
            .send(
                &url,
                RequestContent::Json(Box::new(payload)),
                &mime::APPLICATION_JSON,
                &self.headers.api,
            )
            .await?;

        serde_json::from_str::<PostMessageResponse>(&body)
            .change_context(ChatError::UnreadableResponse)
            .attach_printable_lazy(|| {
                format!(
                    "chat provider returned an unrecognised body: {}",
                    snippet(&body, BODY_SNIPPET_CHARS)
                )
            })?
            .try_into()
    }

    pub(super) async fn upload_file(&self, file: ChatFile) -> ChatResult<FileId> {
        if file.bytes().is_empty() {
            Err(ChatError::InvalidConfiguration("file must not be empty"))?
        }
        if file.filename().trim().is_empty() {
            Err(ChatError::InvalidConfiguration(
                "filename must not be empty",
            ))?
        }

        let thread_ts = file
            .reply_target()
            .map(|message_id| {
                message_id
                    .as_ts()
                    .map(str::to_owned)
                    .ok_or(ChatError::IncompatibleReplyTarget)
            })
            .transpose()?;

        let prepare_url = self.method_url(FILES_GET_UPLOAD_URL);
        let prepare_body = self
            .send(
                &prepare_url,
                RequestContent::Json(Box::new(GetUploadUrlPayload {
                    filename: file.filename().to_owned(),
                    length: file.bytes().len(),
                })),
                &mime::APPLICATION_JSON,
                &self.headers.api,
            )
            .await?;
        let prepared: GetUploadUrlResponse = serde_json::from_str(&prepare_body)
            .change_context(ChatError::UnreadableResponse)
            .attach_printable("could not read files.getUploadURLExternal response")?;
        reject_if_needed(prepared.ok, prepared.error)?;
        let upload_url = prepared
            .upload_url
            .ok_or(ChatError::UnreadableResponse)
            .attach_printable("files.getUploadURLExternal returned no upload_url")?;
        let pending_file_id = prepared
            .file_id
            .filter(|id| !id.is_empty())
            .ok_or(ChatError::UnreadableResponse)
            .attach_printable("files.getUploadURLExternal returned no file_id")?;

        let upload_body = self
            .send(
                upload_url.as_str(),
                RequestContent::RawBytes(file.bytes().to_vec()),
                &mime::APPLICATION_OCTET_STREAM,
                &self.headers.upload,
            )
            .await?;
        if let Ok(envelope) = serde_json::from_str::<UploadLegResponse>(&upload_body) {
            reject_if_needed(envelope.ok, envelope.error)?;
        }

        let complete_url = self.method_url(FILES_COMPLETE_UPLOAD);
        let complete_body = self
            .send(
                &complete_url,
                RequestContent::Json(Box::new(CompleteUploadPayload {
                    files: vec![CompleteUploadFile {
                        id: pending_file_id,
                        title: file.title().map(str::to_owned),
                    }],
                    channel_id: self.channel.clone(),
                    initial_comment: file.comment().map(str::to_owned),
                    thread_ts,
                })),
                &mime::APPLICATION_JSON,
                &self.headers.api,
            )
            .await?;
        let completed: CompleteUploadResponse = serde_json::from_str(&complete_body)
            .change_context(ChatError::UnreadableResponse)
            .attach_printable("could not read files.completeUploadExternal response")?;
        reject_if_needed(completed.ok, completed.error)?;

        completed
            .files
            .unwrap_or_default()
            .into_iter()
            .find_map(|file| file.id.filter(|id| !id.is_empty()))
            .map(FileId::identifier)
            .ok_or(ChatError::MissingFileId)
            .attach_printable("the upload completed but the response named no resulting file")
    }

    async fn send(
        &self,
        url: &str,
        body: RequestContent,
        content_type: &mime::Mime,
        backend_headers: &[(String, Maskable<String>)],
    ) -> ChatResult<String> {
        let mut headers = backend_headers.to_vec();
        headers.push((
            http::header::CONTENT_TYPE.to_string(),
            content_type.essence_str().to_owned().into(),
        ));

        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(url)
            .attach_default_headers()
            .headers(headers)
            .set_body(body)
            .build();
        let response = http_client::send_request(&self.proxy, request, Some(self.timeout_seconds))
            .await
            .change_context(ChatError::RequestFailed)
            .attach_printable_lazy(|| format!("chat request to {url} was not sent"))?;
        let status = response.status();
        let retry_after_seconds = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let body = response
            .text()
            .await
            .change_context(ChatError::UnreadableResponse)?;

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
        Ok(body)
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

        let (text, attachments) = match message.banner() {
            None => (truncate(message.text(), self.max_message_chars), None),
            Some(banner) => (
                String::new(),
                Some(vec![Attachment {
                    color: attachment_color(banner.severity()),
                    blocks: vec![
                        Block::header(truncate(banner.heading(), HEADER_MAX_CHARS)),
                        Block::section(truncate(
                            message.text(),
                            self.max_message_chars.min(SECTION_MAX_CHARS),
                        )),
                    ],
                }]),
            ),
        };

        Ok(PostMessagePayload {
            channel: self.channel.clone(),
            text,
            thread_ts,
            mrkdwn: attachments.is_none().then_some(true),
            attachments,
        })
    }
}

fn attachment_color(severity: ChatSeverity) -> &'static str {
    match severity {
        ChatSeverity::Critical => "danger",
        ChatSeverity::Warning => "warning",
        ChatSeverity::Resolved => "good",
    }
}

fn reject_if_needed(ok: bool, error: Option<SlackErrorCode>) -> ChatResult<()> {
    match ok {
        true => Ok(()),
        false => {
            let reason = error.map_or_else(
                || ChatErrorReason::Other(UNSPECIFIED_ERROR_CODE.to_owned()),
                ChatErrorReason::from,
            );
            Err(ChatError::Rejected { reason }.into())
        }
    }
}

#[derive(Debug, Serialize)]
struct GetUploadUrlPayload {
    filename: String,
    length: usize,
}

#[derive(Debug, Deserialize)]
struct GetUploadUrlResponse {
    ok: bool,
    error: Option<SlackErrorCode>,
    upload_url: Option<Url>,
    file_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadLegResponse {
    ok: bool,
    error: Option<SlackErrorCode>,
}

#[derive(Debug, Serialize)]
struct CompleteUploadPayload {
    files: Vec<CompleteUploadFile>,
    channel_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<String>,
}

#[derive(Debug, Serialize)]
struct CompleteUploadFile {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompleteUploadResponse {
    ok: bool,
    error: Option<SlackErrorCode>,
    files: Option<Vec<CompletedFile>>,
}

#[derive(Debug, Deserialize)]
struct CompletedFile {
    id: Option<String>,
}

#[derive(Debug, Serialize)]
struct PostMessagePayload {
    channel: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    mrkdwn: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<Attachment>>,
}

impl PostMessagePayload {
    fn body_chars(&self) -> usize {
        self.text.chars().count()
            + self
                .attachments
                .iter()
                .flatten()
                .flat_map(|attachment| attachment.blocks.iter())
                .map(|block| block.text.text.chars().count())
                .sum::<usize>()
    }
}

#[derive(Debug, Serialize)]
struct Attachment {
    color: &'static str,
    blocks: Vec<Block>,
}

#[derive(Debug, Serialize)]
struct Block {
    #[serde(rename = "type")]
    kind: &'static str,
    text: BlockText,
}

impl Block {
    fn header(text: String) -> Self {
        Self {
            kind: "header",
            text: BlockText {
                kind: "plain_text",
                text,
                emoji: Some(true),
            },
        }
    }

    fn section(text: String) -> Self {
        Self {
            kind: "section",
            text: BlockText {
                kind: "mrkdwn",
                text,
                emoji: None,
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct BlockText {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    emoji: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PostMessageResponse {
    ok: bool,
    error: Option<SlackErrorCode>,
    ts: Option<String>,
    message: Option<NestedMessage>,
}

#[derive(Debug, Deserialize)]
struct NestedMessage {
    ts: Option<String>,
}

impl TryFrom<PostMessageResponse> for MessageId {
    type Error = error_stack::Report<ChatError>;

    fn try_from(response: PostMessageResponse) -> Result<Self, Self::Error> {
        if !response.ok {
            let reason = response.error.map_or_else(
                || ChatErrorReason::Other(UNSPECIFIED_ERROR_CODE.to_owned()),
                ChatErrorReason::from,
            );
            return Err(ChatError::Rejected { reason }.into());
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SlackErrorCode {
    ChannelNotFound,
    NotInChannel,
    IsArchived,
    InvalidAuth,
    NotAuthed,
    TokenRevoked,
    AccountInactive,
    MsgTooLong,
    RateLimited,
    #[serde(rename = "ratelimited")]
    RateLimitedCompact,
    InvalidArguments,
    ThreadNotFound,
    InternalError,
    #[serde(untagged)]
    Unrecognised(String),
}

impl From<SlackErrorCode> for ChatErrorReason {
    fn from(code: SlackErrorCode) -> Self {
        match code {
            SlackErrorCode::ChannelNotFound => Self::ChannelNotFound,
            SlackErrorCode::NotInChannel | SlackErrorCode::IsArchived => Self::NotInChannel,
            SlackErrorCode::InvalidAuth | SlackErrorCode::NotAuthed => Self::InvalidAuth,
            SlackErrorCode::TokenRevoked | SlackErrorCode::AccountInactive => Self::TokenRevoked,
            SlackErrorCode::MsgTooLong => Self::MessageTooLong,
            SlackErrorCode::RateLimited | SlackErrorCode::RateLimitedCompact => Self::RateLimited {
                retry_after_seconds: None,
            },
            SlackErrorCode::InvalidArguments => Self::Other("invalid_arguments".to_owned()),
            SlackErrorCode::ThreadNotFound => Self::Other("thread_not_found".to_owned()),
            SlackErrorCode::InternalError => Self::Other("internal_error".to_owned()),
            SlackErrorCode::Unrecognised(code) => Self::Other(code),
        }
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

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
    use hyperswitch_masking::Mask as _;
    use serde_json::json;

    use super::*;
    use crate::chat_service::ChatBanner;

    const TEST_MAX_MESSAGE_CHARS: usize = 40_000;

    fn read(value: serde_json::Value) -> Result<ChatResult<MessageId>, serde_json::Error> {
        serde_json::from_value::<PostMessageResponse>(value).map(TryInto::try_into)
    }

    fn reason(code: &str) -> ChatErrorReason {
        serde_json::from_value::<SlackErrorCode>(json!(code))
            .unwrap()
            .into()
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
    fn a_refusal_naming_no_code_is_still_a_refusal() {
        let error = read(json!({"ok": false})).unwrap().unwrap_err();
        match error.current_context() {
            ChatError::Rejected { reason } => assert_eq!(
                reason,
                &ChatErrorReason::Other(UNSPECIFIED_ERROR_CODE.to_owned())
            ),
            other => panic!("expected a rejection, got {other:?}"),
        }
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
            reason("channel_not_found"),
            ChatErrorReason::ChannelNotFound
        );
        assert_eq!(reason("not_in_channel"), ChatErrorReason::NotInChannel);
        assert_eq!(reason("is_archived"), ChatErrorReason::NotInChannel);
        assert_eq!(reason("invalid_auth"), ChatErrorReason::InvalidAuth);
        assert_eq!(reason("not_authed"), ChatErrorReason::InvalidAuth);
        assert_eq!(reason("token_revoked"), ChatErrorReason::TokenRevoked);
        assert_eq!(reason("account_inactive"), ChatErrorReason::TokenRevoked);
        assert_eq!(reason("msg_too_long"), ChatErrorReason::MessageTooLong);

        for spelling in ["rate_limited", "ratelimited"] {
            assert_eq!(
                reason(spelling),
                ChatErrorReason::RateLimited {
                    retry_after_seconds: None
                }
            );
        }

        for code in ["invalid_arguments", "thread_not_found", "internal_error"] {
            assert_eq!(reason(code), ChatErrorReason::Other(code.to_owned()));
        }

        assert_eq!(
            reason("something_new"),
            ChatErrorReason::Other("something_new".to_owned())
        );
    }

    fn endpoint(base_url: &str, method_prefix: &'static str, token: &str) -> ChatResult<Endpoint> {
        Endpoint::new(
            Url::parse(base_url).unwrap(),
            method_prefix,
            EndpointHeaders::new(
                vec![(
                    http::header::AUTHORIZATION.to_string(),
                    format!("Bearer {token}").into_masked(),
                )],
                Vec::new(),
            ),
            "C1".to_owned(),
            DEFAULT_TIMEOUT_SECONDS,
            TEST_MAX_MESSAGE_CHARS,
            Proxy::default(),
        )
    }

    #[test]
    fn an_unusable_destination_is_rejected_on_the_way_in() {
        assert!(endpoint("https://example.com", "/", "token").is_ok());

        let blank_channel = Endpoint::new(
            Url::parse("https://example.com").unwrap(),
            "/",
            EndpointHeaders::new(Vec::new(), Vec::new()),
            "  ".to_owned(),
            DEFAULT_TIMEOUT_SECONDS,
            TEST_MAX_MESSAGE_CHARS,
            Proxy::default(),
        );
        assert!(blank_channel.is_err());

        let zero_cap = Endpoint::new(
            Url::parse("https://example.com").unwrap(),
            "/",
            EndpointHeaders::new(Vec::new(), Vec::new()),
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
            .build_payload(&ChatMessage::reply("hi", MessageId::ts("1.2")))
            .unwrap();

        assert_eq!(payload.thread_ts.as_deref(), Some("1.2"));
        assert_eq!(payload.channel, "C1");
    }

    #[test]
    fn an_unbannered_message_sends_no_attachments_key_at_all() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();

        let payload = endpoint.build_payload(&ChatMessage::new("hi")).unwrap();
        let wire = serde_json::to_value(&payload).unwrap();

        assert_eq!(wire["text"], "hi");
        assert!(
            wire.get("attachments").is_none(),
            "an empty attachments array renders as a stray divider on some backends"
        );
    }

    #[test]
    fn a_banner_moves_the_body_into_a_coloured_attachment() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();

        let payload = endpoint
            .build_payload(
                &ChatMessage::new("*Merchant:* `flowbird`")
                    .with_banner(ChatBanner::new("🔴 SEV1 · Zero SR", ChatSeverity::Critical)),
            )
            .unwrap();

        assert_eq!(
            serde_json::to_value(&payload).unwrap(),
            json!({
                "channel": "C1",
                "text": "",
                "attachments": [{
                    "color": "danger",
                    "blocks": [
                        {
                            "type": "header",
                            "text": {
                                "type": "plain_text",
                                "text": "🔴 SEV1 · Zero SR",
                                "emoji": true,
                            },
                        },
                        {
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": "*Merchant:* `flowbird`",
                            },
                        },
                    ],
                }],
            })
        );
    }

    #[test]
    fn a_bannered_message_does_not_set_mrkdwn() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();

        let payload = endpoint
            .build_payload(
                &ChatMessage::new("*bold*")
                    .with_banner(ChatBanner::new("head", ChatSeverity::Critical)),
            )
            .unwrap();

        assert!(payload.mrkdwn.is_none());
        assert!(serde_json::to_value(&payload)
            .unwrap()
            .get("mrkdwn")
            .is_none());
    }

    #[test]
    fn a_plain_message_still_sets_mrkdwn() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();

        let payload = endpoint.build_payload(&ChatMessage::new("*bold*")).unwrap();

        assert_eq!(payload.mrkdwn, Some(true));
        assert_eq!(serde_json::to_value(&payload).unwrap()["mrkdwn"], true);
    }

    #[test]
    fn an_oversized_heading_is_cut_to_the_header_limit() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();

        let payload = endpoint
            .build_payload(
                &ChatMessage::new("body")
                    .with_banner(ChatBanner::new("x".repeat(400), ChatSeverity::Critical)),
            )
            .unwrap();

        let heading = &payload.attachments.as_ref().unwrap()[0].blocks[0].text.text;
        assert!(heading.chars().count() <= HEADER_MAX_CHARS);
        assert!(heading.ends_with(TRUNCATION_MARKER));
    }

    #[test]
    fn a_long_body_is_cut_to_the_section_limit_when_bannered() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();
        let long = "x".repeat(TEST_MAX_MESSAGE_CHARS - 1);

        let bannered = endpoint
            .build_payload(
                &ChatMessage::new(&long).with_banner(ChatBanner::new("h", ChatSeverity::Critical)),
            )
            .unwrap();
        let section = &bannered.attachments.as_ref().unwrap()[0].blocks[1]
            .text
            .text;
        assert!(section.chars().count() <= SECTION_MAX_CHARS);
        assert!(section.ends_with(TRUNCATION_MARKER));

        let plain = endpoint.build_payload(&ChatMessage::new(&long)).unwrap();
        assert_eq!(plain.text.chars().count(), long.chars().count());
    }

    #[test]
    fn severity_picks_the_rail_colour() {
        assert_eq!(attachment_color(ChatSeverity::Critical), "danger");
        assert_eq!(attachment_color(ChatSeverity::Warning), "warning");
        assert_eq!(attachment_color(ChatSeverity::Resolved), "good");
    }

    #[test]
    fn a_banner_can_still_be_threaded() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();

        let payload = endpoint
            .build_payload(
                &ChatMessage::reply("resolved", MessageId::ts("1.2"))
                    .with_banner(ChatBanner::new("🟢 RESOLVED", ChatSeverity::Resolved)),
            )
            .unwrap();

        assert_eq!(payload.thread_ts.as_deref(), Some("1.2"));
        assert_eq!(payload.attachments.as_ref().unwrap()[0].color, "good");
    }

    #[test]
    fn the_logged_length_counts_the_body_wherever_it_sits() {
        let endpoint = endpoint("https://example.com", "/", "token").unwrap();

        let payload = endpoint
            .build_payload(
                &ChatMessage::new("body")
                    .with_banner(ChatBanner::new("head", ChatSeverity::Warning)),
            )
            .unwrap();

        assert_eq!(payload.body_chars(), "body".len() + "head".len());
    }
}
