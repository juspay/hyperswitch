use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use external_services::chat_service::ChatSeverity;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    core::cloudwatch::announce,
    domain::{
        cloudwatch::{self, State},
        notifier::{
            chat::{ChatFileOutcome, ChatFileReceipt, ChatOutcome, ChatReceipt},
            email::EmailOutcome,
            Outcome, Refusal,
        },
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatNotifyRequest {
    pub text: Secret<String>,

    #[serde(default)]
    pub reply_to: Option<String>,

    #[serde(default)]
    pub heading: Option<Secret<String>>,

    #[serde(default)]
    pub severity: Option<NotifySeverity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotifySeverity {
    Critical,
    Warning,
    Resolved,
}

impl From<NotifySeverity> for ChatSeverity {
    fn from(severity: NotifySeverity) -> Self {
        match severity {
            NotifySeverity::Critical => Self::Critical,
            NotifySeverity::Warning => Self::Warning,
            NotifySeverity::Resolved => Self::Resolved,
        }
    }
}

#[derive(Debug, MultipartForm)]
#[multipart(deny_unknown_fields, duplicate_field = "deny")]
pub struct ChatUploadForm {
    pub file: Bytes,
    pub filename: Option<Text<String>>,
    pub title: Option<Text<String>>,
    pub comment: Option<Text<String>>,
    pub reply_to: Option<Text<String>>,
}

#[derive(Debug)]
pub struct ChatUploadRequest {
    pub bytes: Secret<Vec<u8>>,
    pub filename: Option<Secret<String>>,
    pub title: Option<Secret<String>>,
    pub comment: Option<Secret<String>>,
    pub reply_to: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailNotifyRequest {
    pub subject: Secret<String>,

    pub body: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotifyStatus {
    Delivered,
    Refused,
}

#[derive(Debug, Serialize)]
pub struct ChatNotifyResponse {
    pub status: NotifyStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}

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

#[derive(Debug, Serialize)]
pub struct EmailNotifyResponse {
    pub status: NotifyStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,

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

    #[test]
    fn chat_request_reads_a_banner() {
        let request: ChatNotifyRequest = serde_json::from_value(serde_json::json!({
            "text": "45 of 45 payments failed",
            "heading": "🔴 SEV1 · Zero SR",
            "severity": "critical",
        }))
        .unwrap();

        assert_eq!(
            request
                .heading
                .as_ref()
                .map(hyperswitch_masking::PeekInterface::peek),
            Some(&"🔴 SEV1 · Zero SR".to_owned())
        );
        assert_eq!(request.severity, Some(NotifySeverity::Critical));
    }

    #[test]
    fn every_severity_has_a_wire_spelling() {
        for (wire, expected) in [
            ("critical", NotifySeverity::Critical),
            ("warning", NotifySeverity::Warning),
            ("resolved", NotifySeverity::Resolved),
        ] {
            let severity: NotifySeverity = serde_json::from_value(serde_json::json!(wire)).unwrap();
            assert_eq!(severity, expected);
        }
    }

    #[test]
    fn chat_request_needs_no_banner() {
        let request: ChatNotifyRequest =
            serde_json::from_value(serde_json::json!({ "text": "hi" })).unwrap();

        assert!(request.heading.is_none());
        assert!(request.severity.is_none());
    }

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

    #[test]
    fn debug_never_prints_the_message() {
        let chat = ChatNotifyRequest {
            text: "acquirer_declined for merchant_1234".to_owned().into(),
            reply_to: None,
            heading: Some("zero SR on merchant_1234".to_owned().into()),
            severity: Some(NotifySeverity::Critical),
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

/// The body of both CloudWatch routes.
///
/// Every definition appears, in id order, whether or not it could be read: one that failed
/// carries no `rules` at all rather than states derived from an absence. `announcements` holds
/// only the rules that *changed* state, with the message each produced — on the dry run those
/// messages are rendered and not sent, which is what `"delivery": "skipped"` says.
#[derive(Debug, Serialize)]
pub struct EvaluateResponse {
    pub definitions: Vec<DefinitionState>,
    pub announcements: Vec<AnnouncementResponse>,
}

#[derive(Debug, Serialize)]
pub struct DefinitionState {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub metric_name: String,
    pub dimensions: std::collections::BTreeMap<String, String>,
    pub period_seconds: u32,
    #[serde(flatten)]
    pub outcome: DefinitionOutcome,
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum DefinitionOutcome {
    Evaluated {
        /// Oldest first, `null` where the period had no datapoint. Included because "why did this
        /// say alarm" is the question the route exists to answer.
        readings: Vec<Option<f64>>,
        rules: Vec<RuleStateResponse>,
    },
    Unread {
        reason: UnreadReason,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnreadReason {
    QueryFailed,
    SeriesIncomplete,
    SeriesMissing,
}

#[derive(Debug, Serialize)]
pub struct RuleStateResponse {
    pub severity: String,
    pub state: AlarmState,
    pub threshold: f64,
    pub description: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlarmState {
    Ok,
    Alarm,
    InsufficientData,
}

impl From<cloudwatch::Evaluation> for DefinitionState {
    fn from(evaluation: cloudwatch::Evaluation) -> Self {
        Self {
            id: evaluation.id,
            name: evaluation.name,
            classification: evaluation.classification,
            metric_name: evaluation.metric_name,
            dimensions: evaluation.dimensions,
            period_seconds: evaluation.period,
            outcome: evaluation.outcome.into(),
        }
    }
}

impl From<cloudwatch::Outcome> for DefinitionOutcome {
    fn from(outcome: cloudwatch::Outcome) -> Self {
        match outcome {
            cloudwatch::Outcome::Evaluated { readings, rules } => Self::Evaluated {
                readings,
                rules: rules.into_iter().map(RuleStateResponse::from).collect(),
            },
            cloudwatch::Outcome::Unread { reason } => Self::Unread {
                reason: reason.into(),
            },
        }
    }
}

impl From<cloudwatch::Unread> for UnreadReason {
    fn from(reason: cloudwatch::Unread) -> Self {
        match reason {
            cloudwatch::Unread::QueryFailed => Self::QueryFailed,
            cloudwatch::Unread::SeriesIncomplete => Self::SeriesIncomplete,
            cloudwatch::Unread::SeriesMissing => Self::SeriesMissing,
        }
    }
}

impl From<cloudwatch::RuleState> for RuleStateResponse {
    fn from(rule: cloudwatch::RuleState) -> Self {
        Self {
            severity: rule.severity,
            state: rule.state.into(),
            threshold: rule.threshold,
            description: rule.description,
        }
    }
}

impl From<State> for AlarmState {
    fn from(state: State) -> Self {
        match state {
            State::Ok => Self::Ok,
            State::Alarm => Self::Alarm,
            State::InsufficientData => Self::InsufficientData,
        }
    }
}

/// The body of `POST /alerts/cloudwatch/notify`.
///
/// The same definitions the dry run returns, plus what was said about them and whether it arrived.

#[derive(Debug, Serialize)]
pub struct AnnouncementResponse {
    pub definition_id: String,
    pub severity: String,
    pub destination: String,
    pub message: String,
    #[serde(flatten)]
    pub delivery: DeliveryResponse,
}

#[derive(Debug, Serialize)]
#[serde(tag = "delivery", rename_all = "snake_case")]
pub enum DeliveryResponse {
    Delivered { message_id: Option<String> },
    Refused { code: String },
    Failed,
    UnknownDestination,
    Skipped,
}

impl From<announce::Announced> for EvaluateResponse {
    fn from(announced: announce::Announced) -> Self {
        Self {
            definitions: announced
                .comparison
                .current
                .definitions
                .into_iter()
                .map(DefinitionState::from)
                .collect(),
            announcements: announced
                .announcements
                .into_iter()
                .map(AnnouncementResponse::from)
                .collect(),
        }
    }
}

impl From<announce::Announcement> for AnnouncementResponse {
    fn from(announcement: announce::Announcement) -> Self {
        Self {
            definition_id: announcement.definition_id,
            severity: announcement.severity,
            destination: announcement.destination,
            message: announcement.message,
            delivery: announcement.delivery.into(),
        }
    }
}

impl From<announce::Delivery> for DeliveryResponse {
    fn from(delivery: announce::Delivery) -> Self {
        match delivery {
            announce::Delivery::Delivered { message_id } => Self::Delivered { message_id },
            announce::Delivery::Refused { code } => Self::Refused { code },
            announce::Delivery::Failed => Self::Failed,
            announce::Delivery::UnknownDestination => Self::UnknownDestination,
            announce::Delivery::Skipped => Self::Skipped,
        }
    }
}
