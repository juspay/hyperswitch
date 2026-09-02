//! Delivering an alert to a chat destination.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use error_stack::ResultExt;
use external_services::chat_service::{
    ChatClient, ChatError, ChatErrorReason, ChatMessage, MessageId,
};

use crate::{
    errors::{AlertsApiResult, AlertsError},
    logger,
};

/// The provider code that blames the provider rather than the request.
///
/// Every other code the Slack-compatible backends emit says the request was wrong. This one says
/// the provider failed on its own account, and it is the only one worth trying again. It rides in
/// [`ChatErrorReason::Other`] because `external_services` has no neutral variant for it yet; the
/// distinction is made here rather than by widening a shared enum for one caller.
const PROVIDER_INTERNAL_ERROR: &str = "internal_error";

/// A message to post to one chat destination.
#[derive(Debug, Clone)]
pub struct ChatNotification {
    /// The message, in the markup the destination reads. Delivered unchanged.
    pub text: String,

    /// Post as a reply under this message, if given.
    pub reply_to: Option<String>,
}

/// What a destination hands back once it has accepted a message.
#[derive(Debug, Clone)]
pub struct ChatReceipt {
    /// The provider's id for the message, which threads a later reply under it.
    pub message_id: String,
}

/// Posts an alert to one chat destination.
///
/// One implementation is bound to one destination, so there is no channel argument and no way to
/// address a channel that was not configured.
#[async_trait::async_trait]
pub trait ChatNotifier: Send + Sync + std::fmt::Debug {
    /// Deliver `notification`, returning the id of the message that was created.
    async fn notify(&self, notification: ChatNotification) -> AlertsApiResult<ChatReceipt>;
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
    /// provider refused" is not an actionable sentence and "`sr_alerts` refused" is.
    pub fn new(destination: String, client: Arc<dyn ChatClient>) -> Self {
        Self {
            destination,
            client,
        }
    }
}

#[async_trait::async_trait]
impl ChatNotifier for ChatClientNotifier {
    async fn notify(&self, notification: ChatNotification) -> AlertsApiResult<ChatReceipt> {
        let mut message = ChatMessage::new(notification.text);
        if let Some(reply_to) = notification.reply_to {
            message = message.reply_to(MessageId::ts(reply_to));
        }

        let message_id = self.client.post_message(message).await.map_err(|report| {
            let error = classify(&self.destination, report.current_context());
            // `change_context` rather than a fresh error: every `attach_printable` the client left
            // on the way up — the URL, the response snippet — stays on the report that reaches the
            // log, while the client sees only what `ErrorSwitch` renders.
            report.change_context(error)
        })?;

        Ok(ChatReceipt {
            message_id: message_id
                .as_ts()
                .ok_or(AlertsError::InternalServerError)
                .attach_printable("the chat backend returned a message id of an unexpected kind")?
                .to_owned(),
        })
    }
}

/// The stable, matchable code for a refusal, in the provider's own snake_case vocabulary.
///
/// **Not `Display`.** [`ChatErrorReason`]'s `Display` is prose written for a log line ("channel not
/// found"), and this value goes into the `reason` field of an HTTP error body where a caller is
/// expected to match on it. Deriving one from the other would make the wire contract move whenever
/// somebody rewords an error message.
///
/// **Not always the exact bytes the provider sent**, either, and it cannot be: `external_services`
/// folds synonyms on the way in, so `is_archived` arrives as `NotInChannel` and `account_inactive`
/// as `TokenRevoked`. What is guaranteed is that the same condition always produces the same code.
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

/// Decide who a chat failure belongs to.
///
/// Split by blame rather than by layer. The provider answers a bad channel id and an oversized
/// message through the same field of the same response, and they are not the same problem: one is
/// our configuration, one is the caller's message. See [`AlertsError`] for why that matters to the
/// status code.
fn classify(destination: &str, error: &ChatError) -> AlertsError {
    let destination = destination.to_owned();

    match error {
        ChatError::Rejected { reason } => match reason {
            // The caller's message, and the caller's to fix. Note the provider measures its cap
            // after rendering markup, so this can arrive even though the client already truncated
            // to `max_message_chars` on the way out.
            ChatErrorReason::MessageTooLong => AlertsError::MessageRejected {
                destination,
                reason: reason_code(reason),
            },
            ChatErrorReason::RateLimited {
                retry_after_seconds,
            } => AlertsError::RateLimited {
                destination,
                retry_after_seconds: *retry_after_seconds,
            },
            // Our configuration: the channel, or the credential we hold for it.
            ChatErrorReason::ChannelNotFound
            | ChatErrorReason::NotInChannel
            | ChatErrorReason::InvalidAuth
            | ChatErrorReason::TokenRevoked => AlertsError::DestinationUnusable {
                destination,
                reason: reason_code(reason),
            },
            ChatErrorReason::Other(code) if code == PROVIDER_INTERNAL_ERROR => {
                AlertsError::ProviderUnavailable { destination }
            }
            // Everything else the backends put in `Other` blames the request:
            // `invalid_arguments`, `thread_not_found`, `user_not_found`.
            ChatErrorReason::Other(_) => AlertsError::MessageRejected {
                destination,
                reason: reason_code(reason),
            },
        },

        // `reply_to` came off the request, so an id this backend cannot thread against is the
        // caller having handed back an id from somewhere else.
        ChatError::IncompatibleReplyTarget => AlertsError::MessageRejected {
            destination,
            reason: "incompatible_reply_target".to_owned(),
        },

        // Nothing is known about delivery in any of these. `MissingMessageId` in particular means
        // the message *was* delivered and retrying would post it twice, which is why it does not
        // land anywhere a caller would read as retryable.
        ChatError::RequestFailed
        | ChatError::HttpStatus { .. }
        | ChatError::UnreadableResponse
        | ChatError::MissingMessageId => AlertsError::ProviderUnavailable { destination },

        // Rejected at boot by `Endpoint::new`, so reaching here means a destination was built
        // some other way. Not the caller's problem either way.
        ChatError::InvalidConfiguration(_) => AlertsError::DestinationUnusable {
            destination,
            reason: "invalid_configuration".to_owned(),
        },
    }
}

/// A [`ChatNotifier`] that delivers nothing and says so.
///
/// Configured as `type = "log"`. It exists so a deployment can exercise the whole path — guard,
/// route, registry, response shape — before real credentials exist, which is where sandbox sits
/// until hyperswitch-cloud#23117 closes. Being a destination *type* rather than a flag on a real
/// destination keeps the delivery path branchless: nothing downstream asks "but is this one
/// pretending?".
///
/// **It does not log the message.** A log destination writes to the same stream as everything
/// else, and a chat alert body carries merchant ids and payment volumes. It proves the pipe works,
/// not what the message says; hyperswitch-cloud#23128 checks content against a real channel.
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
    async fn notify(&self, notification: ChatNotification) -> AlertsApiResult<ChatReceipt> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);

        logger::info!(
            tag = "chat_notify_skipped",
            destination = %self.destination,
            chars = notification.text.chars().count(),
            threaded = notification.reply_to.is_some(),
            "not delivered: this destination is configured as `log`"
        );

        Ok(ChatReceipt {
            message_id: format!("log.{sequence:06}"),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    const DESTINATION: &str = "sr_alerts";

    fn rejected(reason: ChatErrorReason) -> AlertsError {
        classify(DESTINATION, &ChatError::Rejected { reason })
    }

    #[test]
    fn an_oversized_message_is_the_callers_problem() {
        assert!(matches!(
            rejected(ChatErrorReason::MessageTooLong),
            AlertsError::MessageRejected { reason, .. } if reason == "msg_too_long"
        ));
    }

    #[test]
    fn a_bad_channel_or_credential_is_ours() {
        for reason in [
            ChatErrorReason::ChannelNotFound,
            ChatErrorReason::NotInChannel,
            ChatErrorReason::InvalidAuth,
            ChatErrorReason::TokenRevoked,
        ] {
            assert!(
                matches!(
                    rejected(reason.clone()),
                    AlertsError::DestinationUnusable { .. }
                ),
                "{reason:?} should blame our configuration"
            );
        }
    }

    /// The one code in `Other` that blames the provider, and the reason it is worth singling out:
    /// it is the only refusal where trying again can help.
    #[test]
    fn the_providers_own_failure_is_separated_from_the_rest_of_other() {
        assert!(matches!(
            rejected(ChatErrorReason::Other(PROVIDER_INTERNAL_ERROR.to_owned())),
            AlertsError::ProviderUnavailable { .. }
        ));
        assert!(matches!(
            rejected(ChatErrorReason::Other("invalid_arguments".to_owned())),
            AlertsError::MessageRejected { reason, .. } if reason == "invalid_arguments"
        ));
    }

    #[test]
    fn rate_limiting_carries_the_wait_through() {
        assert!(matches!(
            rejected(ChatErrorReason::RateLimited {
                retry_after_seconds: Some(30)
            }),
            AlertsError::RateLimited {
                retry_after_seconds: Some(30),
                ..
            }
        ));
    }

    /// An accepted message with no id is not a transport failure in the usual sense: it was
    /// delivered. It must never look retryable, because retrying posts it twice.
    #[test]
    fn transport_failures_all_report_the_provider_as_unavailable() {
        for error in [
            ChatError::RequestFailed,
            ChatError::HttpStatus { status: 503 },
            ChatError::UnreadableResponse,
            ChatError::MissingMessageId,
        ] {
            assert!(
                matches!(
                    classify(DESTINATION, &error),
                    AlertsError::ProviderUnavailable { .. }
                ),
                "{error:?} should report the provider as unavailable"
            );
        }
    }

    /// `reason` is advertised as matchable, so every value it can take has to be a code rather
    /// than a sentence. This caught a real bug: four variants were rendered through `Display` and
    /// reached the wire as prose like `channel not found`.
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

    #[test]
    fn every_failure_names_the_destination_that_produced_it() {
        let error = rejected(ChatErrorReason::ChannelNotFound);
        assert!(error.to_string().contains(DESTINATION));
    }

    #[tokio::test]
    async fn a_log_destination_accepts_a_message_and_mints_distinct_ids() {
        let notifier = LogChatNotifier::new(DESTINATION.to_owned());

        let first = notifier
            .notify(ChatNotification {
                text: "first".to_owned(),
                reply_to: None,
            })
            .await
            .unwrap();

        let second = notifier
            .notify(ChatNotification {
                text: "second".to_owned(),
                reply_to: Some(first.message_id.clone()),
            })
            .await
            .unwrap();

        assert_ne!(first.message_id, second.message_id);
    }
}
