//! The wire contract: what a caller sends and what it gets back.
//!
//! Lives here rather than in an API-models crate for the same reason [`crate::errors::types`] does
//! — `observability` has none — and moves wholesale if one ever appears.
//!
//! ## The URL says where, the body says what
//!
//! `POST /alerts/chat/notify/{destination}`. The path names the channel and the destination; the
//! body carries only content. That keeps the destination visible to access logs, metrics labels and
//! tracing spans without anyone parsing a body, so "which destination is failing" is answerable from
//! the ops view.
//!
//! A single `/notify/{destination}` over a channel-tagged body was considered and rejected: the
//! destination already resolves the channel through configuration, so a tag in the body is a second
//! authority on the same fact and the two can disagree.
//!
//! ## Status answers "did the notifier work", the body answers "did the message arrive"
//!
//! A provider that refuses is a `200` carrying [`NotifyStatus::Refused`], not an HTTP error. It was
//! reached, it answered, and the notifier did its job. Only a request we cannot act on (`4xx`), an
//! unreachable provider (`502`) or our own fault (`500`) is an error — so an alert on `5xx` fires
//! when this service is genuinely broken and at no other time.
//!
//! This is the same line payments draws between a connector declining a transaction and a connector
//! being unreachable, and it is drawn deliberately rather than by fault. Whether `channel_not_found`
//! is our mistake or a merchant's depends on who owns the destination, and that moves from a config
//! file to a database row without a status code being able to move with it.
//!
//! **`status` is required, and that is load-bearing.** A caller cannot deserialize a response
//! without confronting whether the message arrived. `external_services` uses the same trick on the
//! provider's own `ok` field, for the same reason: this shape's failure mode is a caller that reads
//! `200` and stops looking.
//!
//! ## Content is `Secret`, so redaction is the type's job
//!
//! `text`, `subject` and `body` are `Secret<String>`. A subject carries merchant ids and a body
//! carries payment volumes, and `services::server_wrap` takes `T: Debug`, so one added log line
//! would otherwise put both in the log stream. A hand-written `Debug` would do the same job until
//! somebody adds a field and forgets; the type cannot forget.
//!
//! Sizes are logged where they are useful — the chat client already emits `chars` per request — so
//! nothing diagnostic is lost by redacting here.
//!
//! ## Nothing here renders
//!
//! `text`, `subject` and `body` are delivered exactly as they arrive. The caller decides what its
//! message looks like, in whatever markup its destination reads. `body` is HTML, because both email
//! backends in `external_services` hardcode an HTML body and there is no plain-text path to reach.

use std::collections::BTreeMap;

use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    core::alarm as run,
    domain::{
        alarm::AlarmState,
        notifier::{
            chat::{ChatFileOutcome, ChatFileReceipt, ChatOutcome, ChatReceipt},
            email::EmailOutcome,
            Outcome, Refusal,
        },
    },
};

/// The body of `POST /alerts/chat/notify/{destination}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatNotifyRequest {
    /// The message, in the markup the destination reads. Delivered unchanged.
    pub text: Secret<String>,

    /// Post this as a reply in the thread of an earlier message, identified by the `message_id`
    /// that message's response returned.
    #[serde(default)]
    pub reply_to: Option<String>,
}

/// Multipart fields accepted by `POST /alerts/chat/upload/{destination}`.
#[derive(Debug, MultipartForm)]
#[multipart(deny_unknown_fields, duplicate_field = "deny")]
pub struct ChatUploadForm {
    pub file: Bytes,
    pub filename: Option<Text<String>>,
    pub title: Option<Text<String>>,
    pub comment: Option<Text<String>>,
    pub reply_to: Option<Text<String>>,
}

/// Parsed multipart body passed through the authenticated request wrapper.
#[derive(Debug)]
pub struct ChatUploadRequest {
    pub bytes: Secret<Vec<u8>>,
    pub filename: Option<Secret<String>>,
    pub title: Option<Secret<String>>,
    pub comment: Option<Secret<String>>,
    pub reply_to: Option<String>,
}

/// The body of `POST /alerts/email/notify/{destination}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailNotifyRequest {
    /// The subject line, delivered unchanged.
    pub subject: Secret<String>,

    /// The body, as HTML. See the module docs: the transport offers nothing else today.
    pub body: Secret<String>,
}

/// Whether the message arrived.
///
/// Not a bool, so a third outcome can be added without breaking a caller's match, and so the two
/// states read the same in a log line as they do in code.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotifyStatus {
    /// The provider accepted the message.
    Delivered,
    /// The provider was reached and refused it.
    Refused,
}

/// What `/alerts/chat/notify/{destination}` returns.
#[derive(Debug, Serialize)]
pub struct ChatNotifyResponse {
    /// Whether the message arrived. Always present.
    pub status: NotifyStatus,

    /// The provider's id for the message, when it named one. Hand it back as
    /// [`ChatNotifyRequest::reply_to`] to thread under it.
    ///
    /// `null` on a refusal, and also on the rare delivery where the provider accepted the message
    /// without naming an id — the alert went out, but nothing can be threaded under it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// Why the provider refused, as a stable snake_case code — `msg_too_long`,
    /// `channel_not_found`, `rate_limited`. Absent on delivery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    /// How long the provider asked us to wait, when it said. Only set alongside a rate-limiting
    /// code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}

/// What `/alerts/chat/upload/{destination}` returns.
#[derive(Debug, Serialize)]
pub struct ChatUploadResponse {
    pub status: NotifyStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}

/// What `/alerts/email/notify/{destination}` returns.
#[derive(Debug, Serialize)]
pub struct EmailNotifyResponse {
    /// Whether the mail was sent. Always present.
    pub status: NotifyStatus,

    /// Why the provider refused, as a stable snake_case code. Absent on delivery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    /// How long the provider asked us to wait, when it said.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}

impl From<ChatOutcome> for ChatNotifyResponse {
    fn from(outcome: ChatOutcome) -> Self {
        match outcome {
            Outcome::Delivered(ChatReceipt { message_id }) => Self {
                status: NotifyStatus::Delivered,
                message_id,
                error_code: None,
                retry_after_seconds: None,
            },
            Outcome::Refused(Refusal {
                code,
                retry_after_seconds,
            }) => Self {
                status: NotifyStatus::Refused,
                message_id: None,
                error_code: Some(code),
                retry_after_seconds,
            },
        }
    }
}

impl From<ChatFileOutcome> for ChatUploadResponse {
    fn from(outcome: ChatFileOutcome) -> Self {
        match outcome {
            Outcome::Delivered(ChatFileReceipt { file_id }) => Self {
                status: NotifyStatus::Delivered,
                file_id,
                error_code: None,
                retry_after_seconds: None,
            },
            Outcome::Refused(Refusal {
                code,
                retry_after_seconds,
            }) => Self {
                status: NotifyStatus::Refused,
                file_id: None,
                error_code: Some(code),
                retry_after_seconds,
            },
        }
    }
}

impl From<EmailOutcome> for EmailNotifyResponse {
    fn from(outcome: EmailOutcome) -> Self {
        match outcome {
            Outcome::Delivered(()) => Self {
                status: NotifyStatus::Delivered,
                error_code: None,
                retry_after_seconds: None,
            },
            Outcome::Refused(Refusal {
                code,
                retry_after_seconds,
            }) => Self {
                status: NotifyStatus::Refused,
                error_code: Some(code),
                retry_after_seconds,
            },
        }
    }
}

/// The body of `POST /alerts/cloudwatch/evaluate`.
///
/// Every field is optional, so an empty body is a normal evaluation — the shape a cron trigger or
/// a bare `curl -XPOST` sends.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EvaluateAlarmsRequest {
    /// Evaluate and render, but send nothing at all.
    ///
    /// Nothing, including the failure summary: a dry run that announced a CloudWatch outage would
    /// not be a dry run. The messages a real run would have sent come back in the response, built
    /// by the same code that would have sent them.
    pub dry_run: bool,
}

/// What `POST /alerts/cloudwatch/evaluate` returns.
///
/// Verbose on purpose. For a while the only caller is a person with `curl` asking why an alert did
/// or did not fire, and the questions they will have — what did CloudWatch say, what state did
/// that put each severity in, what changed, what was sent, what could not be read — are all
/// answered here rather than only in the logs.
///
/// The rendered messages are included in full. They carry metric names, thresholds and instance
/// identifiers, which are infrastructure facts rather than the merchant data the notify routes
/// treat as [`hyperswitch_masking::Secret`].
#[derive(Debug, Serialize)]
pub struct EvaluateAlarmsResponse {
    /// The instant this evaluation ends at: the latest completed minute, less any configured
    /// delay.
    ///
    /// Rendered by `common_utils`' ISO 8601 serializer, so it reads the same as every other
    /// timestamp this repository puts on the wire.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub evaluated_at: time::PrimitiveDateTime,

    /// The instant the evaluation it was compared against ends at. One minute earlier, whatever
    /// the metrics' periods — that is CloudWatch's own evaluation cadence.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub compared_with: time::PrimitiveDateTime,

    /// Whether every send was skipped.
    pub dry_run: bool,

    /// How many catalogue entries the run covered.
    pub definitions: usize,

    /// How many distinct CloudWatch queries those entries needed. Fewer than the severities,
    /// because a threshold is not part of a query.
    pub readings: usize,

    /// The counts worth reading before the list.
    pub summary: AlarmRunSummary,

    /// One entry per severity, in catalogue order.
    pub evaluations: Vec<AlarmEvaluation>,

    /// The operational summary, present only when something could not be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<AlarmRunFailure>,
}

/// The headline counts of one run.
#[derive(Debug, Serialize)]
pub struct AlarmRunSummary {
    /// Severities that produced a state.
    pub evaluated: usize,
    /// Severities whose metric could not be read.
    pub unevaluated: usize,
    /// Severities whose state moved.
    pub transitions: usize,
    /// Announcements the provider accepted.
    pub delivered: usize,
    /// Announcements that were refused or could not be sent. Non-zero here with a `200` overall is
    /// the case the delivery-hardening work exists for.
    pub undelivered: usize,
}

/// What one severity's evaluation found.
#[derive(Debug, Serialize)]
pub struct AlarmEvaluation {
    /// The catalogue entry's name.
    pub definition: String,
    /// The group it belongs to.
    pub classification: String,
    /// The severity id.
    pub severity: String,
    /// Namespace and metric name.
    pub metric: String,
    /// The dimensions it read. What actually identifies the CloudWatch metric, so this is the
    /// field to compare against the AWS console when an alarm disagrees with its counterpart.
    pub dimensions: BTreeMap<String, String>,

    /// The state the current window puts it in. `null` when it could not be evaluated, which is
    /// deliberately different from any of the three states.
    pub state: Option<&'static str>,
    /// The state the previous window puts it in.
    pub previous_state: Option<&'static str>,
    /// Whether the two differ. The only thing that causes an announcement.
    pub transitioned: bool,

    /// The most recent real reading in the current window. `null` when the window held none, in
    /// which case the state came from the missing-data policy alone.
    pub observed: Option<f64>,
    /// What readings were compared against.
    pub threshold: f64,
    /// How many of the window's readings breached.
    pub breaching_datapoints: usize,
    /// How many periods the missing-data policy had to stand in for.
    pub missing_datapoints: usize,
    /// N.
    pub evaluation_periods: u32,
    /// M.
    pub datapoints_to_alarm: u32,

    /// Why it could not be evaluated. Present exactly when `state` is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// The message that was sent, or that a dry run would have sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// What came of sending it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<AlarmDelivery>,
}

/// The run's own failure, when CloudWatch did not answer for some or all of it.
#[derive(Debug, Serialize)]
pub struct AlarmRunFailure {
    /// The definitions that went unevaluated.
    pub definitions: Vec<String>,
    /// The one reason behind all of them, when there was only one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The summary that was sent, or that a dry run would have sent.
    pub message: String,
    /// What came of sending it.
    pub delivery: AlarmDelivery,
}

/// What came of one attempt to send a message.
///
/// Tagged, and the tag is required, for the reason [`NotifyStatus`] is: a caller cannot read this
/// without confronting whether the message actually arrived.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AlarmDelivery {
    /// The provider accepted it.
    Delivered {
        /// The destination it went to.
        destination: String,
        /// The provider's id for the message, when it named one.
        #[serde(skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
    },
    /// The provider was reached and refused it.
    Refused {
        /// The destination that refused.
        destination: String,
        /// The stable snake_case code it refused with.
        code: String,
    },
    /// The provider could not be reached, so whether it arrived is unknown.
    Failed {
        /// The destination that could not be reached.
        destination: String,
        /// What went wrong.
        error: String,
    },
    /// Nothing was sent, because this was a dry run.
    SkippedDryRun {
        /// Where it would have gone.
        destination: String,
    },
}

impl From<run::RunReport> for EvaluateAlarmsResponse {
    fn from(report: run::RunReport) -> Self {
        let evaluated = report
            .targets
            .iter()
            .filter(|target| target.state.is_some())
            .count();
        let transitions = report
            .targets
            .iter()
            .filter(|target| target.transition.is_some())
            .count();
        let delivered = report
            .targets
            .iter()
            .filter(|target| matches!(target.delivery, Some(run::DeliveryReport::Delivered { .. })))
            .count();

        Self {
            evaluated_at: report.evaluated_at,
            compared_with: report.previous_evaluated_at,
            dry_run: report.dry_run,
            definitions: report.definitions,
            readings: report.readings,
            summary: AlarmRunSummary {
                evaluated,
                unevaluated: report.targets.len().saturating_sub(evaluated),
                transitions,
                delivered,
                undelivered: transitions.saturating_sub(delivered),
            },
            failure: report.failure.map(|failure| AlarmRunFailure {
                definitions: failure.definitions,
                reason: failure.reason,
                message: failure.message,
                delivery: failure.delivery.into(),
            }),
            evaluations: report
                .targets
                .into_iter()
                .map(AlarmEvaluation::from)
                .collect(),
        }
    }
}

impl From<run::TargetReport> for AlarmEvaluation {
    fn from(target: run::TargetReport) -> Self {
        Self {
            definition: target.definition,
            classification: target.classification,
            severity: target.severity,
            metric: target.metric,
            dimensions: target.dimensions,
            state: target.state.map(AlarmState::label),
            previous_state: target.previous_state.map(AlarmState::label),
            transitioned: target.transition.is_some(),
            observed: target.observed,
            threshold: target.threshold,
            breaching_datapoints: target.breaching_datapoints,
            missing_datapoints: target
                .evaluation
                .map(|evaluation| evaluation.filled)
                .unwrap_or_default(),
            evaluation_periods: target.evaluation_periods,
            datapoints_to_alarm: target.datapoints_to_alarm,
            error: target.error,
            message: target.message,
            delivery: target.delivery.map(AlarmDelivery::from),
        }
    }
}

impl From<run::DeliveryReport> for AlarmDelivery {
    fn from(delivery: run::DeliveryReport) -> Self {
        match delivery {
            run::DeliveryReport::Delivered {
                destination,
                message_id,
            } => Self::Delivered {
                destination,
                message_id,
            },
            run::DeliveryReport::Refused { destination, code } => {
                Self::Refused { destination, code }
            }
            run::DeliveryReport::Failed { destination, error } => {
                Self::Failed { destination, error }
            }
            run::DeliveryReport::SkippedDryRun { destination } => {
                Self::SkippedDryRun { destination }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn body_of<T: Serialize>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn a_delivery_carries_the_message_id_and_no_error() {
        let body = body_of(&ChatNotifyResponse::from(Outcome::Delivered(ChatReceipt {
            message_id: Some("cmtk931s1".to_owned()),
        })));

        assert_eq!(body["status"], "delivered");
        assert_eq!(body["message_id"], "cmtk931s1");
        assert!(body.get("error_code").is_none());
    }

    #[test]
    fn a_file_delivery_returns_a_file_id_not_a_message_id() {
        let body = body_of(&ChatUploadResponse::from(Outcome::Delivered(
            ChatFileReceipt {
                file_id: Some("file-1".to_owned()),
            },
        )));

        assert_eq!(body["status"], "delivered");
        assert_eq!(body["file_id"], "file-1");
        assert!(body.get("message_id").is_none());
    }

    /// The alert went out; only the ability to thread under it was lost. It must not look like a
    /// failure, because a retry would post the alert twice.
    #[test]
    fn a_delivery_without_an_id_is_still_a_delivery() {
        let body = body_of(&ChatNotifyResponse::from(Outcome::Delivered(ChatReceipt {
            message_id: None,
        })));

        assert_eq!(body["status"], "delivered");
        assert!(body.get("message_id").is_none());
    }

    #[test]
    fn a_refusal_carries_the_code_and_no_message_id() {
        let body = body_of(&ChatNotifyResponse::from(Outcome::Refused(
            Refusal::retry_after("rate_limited", Some(30)),
        )));

        assert_eq!(body["status"], "refused");
        assert_eq!(body["error_code"], "rate_limited");
        assert_eq!(body["retry_after_seconds"], 30);
        assert!(body.get("message_id").is_none());
    }

    /// `status` is what stops a caller reading `200` and assuming delivery, so it is never skipped.
    #[test]
    fn status_is_always_present() {
        for outcome in [
            Outcome::Delivered(()),
            Outcome::Refused(Refusal::new("channel_not_found")),
        ] {
            let body = body_of(&EmailNotifyResponse::from(outcome));
            assert!(body.get("status").is_some());
        }
    }

    #[test]
    fn chat_request_reads_a_threaded_message() {
        let request: ChatNotifyRequest =
            serde_json::from_value(serde_json::json!({ "text": "hi", "reply_to": "cmtk931s1" }))
                .unwrap();

        assert_eq!(request.reply_to.as_deref(), Some("cmtk931s1"));
    }

    /// Threading against a mailing list is a caller bug. `deny_unknown_fields` makes it a rejection
    /// rather than a field that quietly goes nowhere.
    #[test]
    fn email_request_rejects_reply_to() {
        let error = serde_json::from_value::<EmailNotifyRequest>(serde_json::json!({
            "subject": "s",
            "body": "<pre>b</pre>",
            "reply_to": "cmtk931s1",
        }))
        .unwrap_err();

        assert!(error.to_string().contains("reply_to"));
    }

    /// The property is now the type's, not a hand-written `Debug`'s: a field added later cannot
    /// leak by someone forgetting to update an impl.
    #[test]
    fn debug_never_prints_the_message() {
        let chat = ChatNotifyRequest {
            text: "acquirer_declined for merchant_1234".to_owned().into(),
            reply_to: None,
        };
        assert!(!format!("{chat:?}").contains("merchant_1234"));

        let email = EmailNotifyRequest {
            subject: "merchant_1234 not converting".to_owned().into(),
            body: "<pre>4,201 of 5,000 payments lost</pre>".to_owned().into(),
        };
        let rendered = format!("{email:?}");
        assert!(!rendered.contains("merchant_1234"));
        assert!(!rendered.contains("4,201"));
    }
}
