//! Delivering an alert to a chat destination.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use external_services::chat_service::{
    ChatClient, ChatError, ChatErrorReason, ChatMessage, MessageId,
};
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};

use super::{Outcome, Refusal};
use crate::{
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
};

/// The provider code that blames the provider rather than the message.
///
/// Every other code the Slack-compatible backends emit describes something about the request. This
/// one says the provider failed on its own account, so nothing is known about delivery and it
/// belongs with the transport failures rather than with the refusals. It rides in
/// [`ChatErrorReason::Other`] because `external_services` has no neutral variant for it; the
/// distinction is drawn here rather than by widening a shared enum for one caller.
const PROVIDER_INTERNAL_ERROR: &str = "internal_error";

/// A message to post to one chat destination.
#[derive(Debug, Clone)]
pub struct ChatNotification {
    /// The message, in the markup the destination reads. Delivered unchanged.
    pub text: Secret<String>,

    /// Post as a reply under this message, if given.
    pub reply_to: Option<String>,
}

/// What a chat destination hands back when it accepts a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatReceipt {
    /// The provider's id for the message, which threads a later reply under it.
    ///
    /// `None` when the provider accepted the message but named no id. The alert *was* delivered —
    /// retrying would post it twice — so this is a success that has lost only the ability to
    /// thread under it.
    pub message_id: Option<String>,
}

/// The result of one chat delivery attempt.
pub type ChatOutcome = Outcome<ChatReceipt>;

/// Posts an alert to one chat destination.
///
/// One implementation is bound to one destination, so there is no channel argument and no way to
/// address a channel that was not configured.
#[async_trait::async_trait]
pub trait ChatNotifier: Send + Sync + std::fmt::Debug {
    /// Attempt delivery.
    ///
    /// A provider that refuses returns `Ok(Outcome::Refused)`, not an error: it was reached and it
    /// answered. `Err` means the attempt itself failed, so whether the message arrived is unknown.
    async fn notify(&self, notification: ChatNotification) -> ObservabilityApiResult<ChatOutcome>;
}

/// A [`ChatNotifier`] backed by a real chat transport.
#[derive(Debug)]
pub struct ChatClientNotifier {
    destination: String,
    client: Arc<dyn ChatClient>,
}

impl ChatClientNotifier {
    /// Bind a client to the destination id it was configured under.
    ///
    /// The id is carried so failures can name it. With several destinations configured, "the chat
    /// provider is unreachable" is not an actionable sentence and "`sr_alerts` is" is.
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

        match self.client.post_message(message).await {
            Ok(message_id) => Ok(Outcome::Delivered(ChatReceipt {
                message_id: message_id.as_ts().map(str::to_owned),
            })),

            Err(report) => match classify(report.current_context()) {
                Verdict::Refused(refusal) => Ok(Outcome::Refused(refusal)),

                // `Delivered` from an `Err` is not a contradiction: the provider accepted the
                // message and named no id for it. Reporting that as a failure would invite a retry
                // that posts the alert twice.
                Verdict::DeliveredWithoutId => {
                    Ok(Outcome::Delivered(ChatReceipt { message_id: None }))
                }

                Verdict::Failed(error) => {
                    // `change_context` rather than a fresh error, so every `attach_printable` the
                    // client left on the way up — the URL, the response snippet — reaches the log
                    // while the caller sees only what `ErrorSwitch` renders.
                    Err(report.change_context(error(self.destination.clone())))
                }
            },
        }
    }
}

/// What a chat failure means for the caller.
enum Verdict {
    /// The provider answered and said no.
    Refused(Refusal),
    /// The provider accepted the message but named no id for it.
    DeliveredWithoutId,
    /// Nothing is known about delivery.
    Failed(fn(String) -> ObservabilityError),
}

/// Decide whether a chat failure is a refusal, a delivery, or an unknown.
///
/// The dividing line is **what the provider told us**, not whose fault it is. A refusal in its
/// documented envelope — for any reason, including a channel we cannot see or a credential it will
/// not accept — means the attempt completed and the answer was no. Only silence, or an answer we
/// cannot interpret, leaves delivery unknown.
///
/// Fault deliberately does not appear here. Whether a bad channel id is our mistake or a merchant's
/// depends on who owns the destination, and that moves from a config file to a database row without
/// the wire contract being allowed to move with it.
fn classify(error: &ChatError) -> Verdict {
    match error {
        ChatError::Rejected { reason } => match reason {
            // The provider blaming itself is not an answer about the message, so nothing is known.
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

        // `reply_to` came off the request and named an id this backend cannot thread against.
        // Rejected before anything was sent, so the message did not go anywhere.
        ChatError::IncompatibleReplyTarget => {
            Verdict::Refused(Refusal::new("incompatible_reply_target"))
        }

        // No answer, or one outside the documented envelope. Delivery is genuinely unknown.
        ChatError::RequestFailed | ChatError::HttpStatus { .. } | ChatError::UnreadableResponse => {
            Verdict::Failed(|destination| ObservabilityError::ProviderUnavailable { destination })
        }

        // Rejected at boot by `Endpoint::new`, so reaching here means a destination was built some
        // other way.
        ChatError::InvalidConfiguration(_) => {
            Verdict::Failed(|_destination| ObservabilityError::InternalServerError)
        }
    }
}

/// The stable, matchable code for a refusal, in the provider's own snake_case vocabulary.
///
/// **Not `Display`.** [`ChatErrorReason`]'s `Display` is prose written for a log line ("channel not
/// found"), and this value goes onto the wire where a caller matches on it. Deriving one from the
/// other would let a reworded error message silently change the API.
///
/// **Not always the exact bytes the provider sent**, and it cannot be: `external_services` folds
/// synonyms on the way in, so `is_archived` arrives as `NotInChannel` and `account_inactive` as
/// `TokenRevoked`. The guarantee is that one condition always yields one code.
fn reason_code(reason: &ChatErrorReason) -> String {
    match reason {
        ChatErrorReason::ChannelNotFound => "channel_not_found".to_owned(),
        ChatErrorReason::NotInChannel => "not_in_channel".to_owned(),
        ChatErrorReason::InvalidAuth => "invalid_auth".to_owned(),
        ChatErrorReason::TokenRevoked => "token_revoked".to_owned(),
        ChatErrorReason::MessageTooLong => "msg_too_long".to_owned(),
        ChatErrorReason::RateLimited { .. } => "rate_limited".to_owned(),
        // Already a wire code, carried through untouched.
        ChatErrorReason::Other(code) => code.clone(),
    }
}

/// A [`ChatNotifier`] that delivers nothing and says so.
///
/// Configured as `type = "log"`, so a deployment can exercise the whole path — guard, route,
/// registry, response shape — before real credentials exist. A destination *type* rather than a
/// flag on a real destination, which keeps the delivery path branchless: nothing downstream asks
/// "but is this one pretending?".
///
/// **It does not log the message.** This writes to the same stream as everything else, and an
/// alert body carries merchant ids and payment volumes. It proves the pipe works, not what the
/// message says.
#[derive(Debug)]
pub struct LogChatNotifier {
    destination: String,
    /// Makes each synthetic id distinct so a threading round trip can be exercised end to end.
    sequence: AtomicU64,
}

impl LogChatNotifier {
    /// Build a log destination under the id it was configured with.
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
            "not delivered: this destination is configured as `log`"
        );

        Ok(Outcome::Delivered(ChatReceipt {
            message_id: Some(format!("log.{sequence:06}")),
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

    /// The whole point of the redesign: every reason the provider names is an answer, so it comes
    /// back as an outcome. Fault does not enter into it — `channel_not_found` is as much an answer
    /// as `msg_too_long`, and which of them is "our fault" changes when destinations move to a
    /// database.
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

    /// The provider blaming itself is not an answer about the message, so delivery is unknown.
    #[test]
    fn the_providers_own_failure_is_not_a_refusal() {
        assert!(matches!(
            classify(&ChatError::Rejected {
                reason: ChatErrorReason::Other(PROVIDER_INTERNAL_ERROR.to_owned()),
            }),
            Verdict::Failed(_)
        ));
    }

    /// The message went out. Anything that reads as retryable here posts the alert twice.
    #[test]
    fn an_accepted_message_with_no_id_is_a_delivery() {
        assert!(matches!(
            classify(&ChatError::MissingMessageId),
            Verdict::DeliveredWithoutId
        ));
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

    /// `code` is advertised as matchable, so every value it can take has to be a code rather than a
    /// sentence. This caught a real bug: four variants were rendered through `Display` and reached
    /// the wire as prose like `channel not found`.
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

    /// The provider's spelling, not ours. Pinned because these are a wire contract now.
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
            })
            .await
            .unwrap();
        let second = notifier
            .notify(ChatNotification {
                text: "second".to_owned().into(),
                reply_to: None,
            })
            .await
            .unwrap();

        assert_ne!(first, second);
        assert!(matches!(first, Outcome::Delivered(_)));
    }
}
