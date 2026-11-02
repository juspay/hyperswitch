use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use external_services::chat_service::{
    ChatBanner, ChatClient, ChatError, ChatErrorReason, ChatFile, ChatMessage, MessageId,
};
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};

use super::{Outcome, Refusal};
use crate::{
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
};

const PROVIDER_INTERNAL_ERROR: &str = "internal_error";

#[derive(Debug, Clone)]
pub struct ChatNotification {
    pub text: Secret<String>,

    pub reply_to: Option<String>,

    pub banner: Option<ChatBanner>,
}

#[derive(Debug, Clone)]
pub struct ChatFileUpload {
    pub bytes: Secret<Vec<u8>>,
    pub filename: Secret<String>,
    pub title: Option<Secret<String>>,
    pub comment: Option<Secret<String>>,
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatFileReceipt {
    pub file_id: Option<String>,
}

pub type ChatFileOutcome = Outcome<ChatFileReceipt>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatReceipt {
    pub message_id: Option<String>,
}

pub type ChatOutcome = Outcome<ChatReceipt>;

#[async_trait::async_trait]
pub trait ChatNotifier: Send + Sync + std::fmt::Debug {
    async fn notify(&self, notification: ChatNotification) -> ObservabilityApiResult<ChatOutcome>;

    async fn upload_file(&self, upload: ChatFileUpload) -> ObservabilityApiResult<ChatFileOutcome>;
}

#[derive(Debug)]
pub struct ChatClientNotifier {
    destination: String,
    client: Arc<dyn ChatClient>,
}

impl ChatClientNotifier {
    pub fn new(destination: String, client: Arc<dyn ChatClient>) -> Self {
        Self {
            destination,
            client,
        }
    }
}

#[async_trait::async_trait]
impl ChatNotifier for ChatClientNotifier {
    async fn notify(&self, notification: ChatNotification) -> ObservabilityApiResult<ChatOutcome> {
        let message = match notification.reply_to {
            Some(reply_to) => {
                ChatMessage::reply(notification.text.expose(), MessageId::ts(reply_to))
            }
            None => ChatMessage::new(notification.text.expose()),
        };
        let message = match notification.banner {
            Some(banner) => message.with_banner(banner),
            None => message,
        };

        match self.client.post_message(message).await {
            Ok(message_id) => Ok(Outcome::Delivered(ChatReceipt {
                message_id: message_id.as_ts().map(str::to_owned),
            })),

            Err(report) => match classify(report.current_context()) {
                Verdict::Refused(refusal) => Ok(Outcome::Refused(refusal)),

                Verdict::DeliveredWithoutId => {
                    Ok(Outcome::Delivered(ChatReceipt { message_id: None }))
                }

                Verdict::Failed(error) => {
                    Err(report.change_context(error(self.destination.clone())))
                }
            },
        }
    }

    async fn upload_file(&self, upload: ChatFileUpload) -> ObservabilityApiResult<ChatFileOutcome> {
        let file = ChatFile::new(
            upload.bytes.expose(),
            upload.filename.expose(),
            upload.title.map(ExposeInterface::expose),
            upload.comment.map(ExposeInterface::expose),
            upload.reply_to.map(MessageId::ts),
        );

        match self.client.upload_file(file).await {
            Ok(file_id) => Ok(Outcome::Delivered(ChatFileReceipt {
                file_id: file_id.as_identifier().map(str::to_owned),
            })),
            Err(report) => match classify(report.current_context()) {
                Verdict::Refused(refusal) => Ok(Outcome::Refused(refusal)),
                Verdict::DeliveredWithoutId => {
                    Ok(Outcome::Delivered(ChatFileReceipt { file_id: None }))
                }
                Verdict::Failed(error) => {
                    Err(report.change_context(error(self.destination.clone())))
                }
            },
        }
    }
}

enum Verdict {
    Refused(Refusal),
    DeliveredWithoutId,
    Failed(fn(String) -> ObservabilityError),
}

fn classify(error: &ChatError) -> Verdict {
    match error {
        ChatError::Rejected { reason } => match reason {
            ChatErrorReason::Other(code) if code == PROVIDER_INTERNAL_ERROR => {
                Verdict::Failed(|destination| ObservabilityError::ProviderUnavailable {
                    destination,
                })
            }
            ChatErrorReason::RateLimited {
                retry_after_seconds,
            } => Verdict::Refused(Refusal::retry_after(
                reason_code(reason),
                *retry_after_seconds,
            )),
            _ => Verdict::Refused(Refusal::new(reason_code(reason))),
        },

        ChatError::MissingMessageId => Verdict::DeliveredWithoutId,

        ChatError::IncompatibleReplyTarget => {
            Verdict::Refused(Refusal::new("incompatible_reply_target"))
        }

        ChatError::RequestFailed | ChatError::HttpStatus { .. } | ChatError::UnreadableResponse => {
            Verdict::Failed(|destination| ObservabilityError::ProviderUnavailable { destination })
        }

        ChatError::InvalidConfiguration(_) => {
            Verdict::Failed(|_destination| ObservabilityError::InternalServerError)
        }

        ChatError::MissingFileId => Verdict::DeliveredWithoutId,
    }
}

fn reason_code(reason: &ChatErrorReason) -> String {
    match reason {
        ChatErrorReason::ChannelNotFound => "channel_not_found".to_owned(),
        ChatErrorReason::NotInChannel => "not_in_channel".to_owned(),
        ChatErrorReason::InvalidAuth => "invalid_auth".to_owned(),
        ChatErrorReason::TokenRevoked => "token_revoked".to_owned(),
        ChatErrorReason::MessageTooLong => "msg_too_long".to_owned(),
        ChatErrorReason::RateLimited { .. } => "rate_limited".to_owned(),
        ChatErrorReason::Other(code) => code.clone(),
    }
}

#[derive(Debug)]
pub struct LogChatNotifier {
    destination: String,
    sequence: AtomicU64,
}

impl LogChatNotifier {
    pub fn new(destination: String) -> Self {
        Self {
            destination,
            sequence: AtomicU64::new(1),
        }
    }
}

#[async_trait::async_trait]
impl ChatNotifier for LogChatNotifier {
    async fn notify(&self, notification: ChatNotification) -> ObservabilityApiResult<ChatOutcome> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);

        logger::info!(
            tag = "chat_notify_skipped",
            destination = %self.destination,
            chars = notification.text.peek().chars().count(),
            threaded = notification.reply_to.is_some(),
            bannered = notification.banner.is_some(),
            "not delivered: this destination is configured as `log`"
        );

        Ok(Outcome::Delivered(ChatReceipt {
            message_id: Some(format!("log.{sequence:06}")),
        }))
    }

    async fn upload_file(&self, upload: ChatFileUpload) -> ObservabilityApiResult<ChatFileOutcome> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        logger::info!(
            tag = "chat_upload_skipped",
            destination = %self.destination,
            bytes = upload.bytes.peek().len(),
            threaded = upload.reply_to.is_some(),
            "not delivered: this destination is configured as `log`"
        );
        Ok(Outcome::Delivered(ChatFileReceipt {
            file_id: Some(format!("log-file.{sequence:06}")),
        }))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn refusal_for(reason: ChatErrorReason) -> Refusal {
        match classify(&ChatError::Rejected { reason }) {
            Verdict::Refused(refusal) => refusal,
            _ => panic!("a documented refusal should be an outcome, not a failure"),
        }
    }

    #[test]
    fn every_documented_refusal_is_an_outcome() {
        for reason in [
            ChatErrorReason::ChannelNotFound,
            ChatErrorReason::NotInChannel,
            ChatErrorReason::InvalidAuth,
            ChatErrorReason::TokenRevoked,
            ChatErrorReason::MessageTooLong,
            ChatErrorReason::Other("thread_not_found".to_owned()),
        ] {
            let code = refusal_for(reason.clone()).code;
            assert!(!code.is_empty(), "{reason:?} produced no code");
        }
    }

    #[test]
    fn rate_limiting_carries_the_wait_into_the_outcome() {
        assert_eq!(
            refusal_for(ChatErrorReason::RateLimited {
                retry_after_seconds: Some(30)
            }),
            Refusal {
                code: "rate_limited".to_owned(),
                retry_after_seconds: Some(30),
            }
        );
    }

    #[test]
    fn the_providers_own_failure_is_not_a_refusal() {
        assert!(matches!(
            classify(&ChatError::Rejected {
                reason: ChatErrorReason::Other(PROVIDER_INTERNAL_ERROR.to_owned()),
            }),
            Verdict::Failed(_)
        ));
    }

    #[test]
    fn accepted_content_with_no_id_is_still_a_delivery() {
        for error in [ChatError::MissingMessageId, ChatError::MissingFileId] {
            assert!(matches!(classify(&error), Verdict::DeliveredWithoutId));
        }
    }

    #[test]
    fn only_silence_or_an_unreadable_answer_leaves_delivery_unknown() {
        for error in [
            ChatError::RequestFailed,
            ChatError::HttpStatus { status: 503 },
            ChatError::UnreadableResponse,
        ] {
            assert!(
                matches!(classify(&error), Verdict::Failed(_)),
                "{error:?} should leave delivery unknown"
            );
        }
    }

    #[test]
    fn every_reason_is_a_matchable_code_not_prose() {
        let reasons = [
            ChatErrorReason::ChannelNotFound,
            ChatErrorReason::NotInChannel,
            ChatErrorReason::InvalidAuth,
            ChatErrorReason::TokenRevoked,
            ChatErrorReason::MessageTooLong,
            ChatErrorReason::RateLimited {
                retry_after_seconds: None,
            },
            ChatErrorReason::Other("invalid_arguments".to_owned()),
        ];

        for reason in reasons {
            let code = reason_code(&reason);
            assert!(
                !code.is_empty()
                    && !code.contains(' ')
                    && code
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{reason:?} produced `{code}`, which is not a matchable code"
            );
        }
    }

    #[test]
    fn reason_codes_match_the_providers_spelling() {
        assert_eq!(
            reason_code(&ChatErrorReason::ChannelNotFound),
            "channel_not_found"
        );
        assert_eq!(
            reason_code(&ChatErrorReason::MessageTooLong),
            "msg_too_long"
        );
        assert_eq!(
            reason_code(&ChatErrorReason::Other("thread_not_found".to_owned())),
            "thread_not_found"
        );
    }

    #[tokio::test]
    async fn a_log_destination_delivers_and_mints_distinct_ids() {
        let notifier = LogChatNotifier::new("smoke".to_owned());

        let first = notifier
            .notify(ChatNotification {
                text: "first".to_owned().into(),
                reply_to: None,
                banner: None,
            })
            .await
            .unwrap();
        let second = notifier
            .notify(ChatNotification {
                text: "second".to_owned().into(),
                reply_to: None,
                banner: None,
            })
            .await
            .unwrap();

        assert_ne!(first, second);
        assert!(matches!(first, Outcome::Delivered(_)));
    }
}
