use api_models::{enums::EventType as OutgoingWebhookEventType, webhooks::OutgoingWebhookContent};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

use super::{EventType, RawEvent};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OutgoingWebhookEvent {
    merchant_id: String,
    event_id: String,
    event_type: OutgoingWebhookEventType,
    #[serde(flatten)]
    content: Option<OutgoingWebhookEventContent>,
    is_error: bool,
    error: Option<Value>,
    created_at_timestamp: i128,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "outgoing_webhook_event_type", rename_all = "snake_case")]
pub enum OutgoingWebhookEventContent {
    Payment {
        payment_id: Option<String>,
        content: Value,
    },
    Refund {
        payment_id: String,
        refund_id: String,
        content: Value,
    },
    Dispute {
        payment_id: String,
        attempt_id: String,
        dispute_id: String,
        content: Value,
    },
    Mandate {
        payment_method_id: String,
        mandate_id: String,
        content: Value,
    },
}
pub trait OutgoingWebhookEventMetric {
    fn get_outgoing_webhook_event_type(&self) -> Option<OutgoingWebhookEventContent>;
}
impl OutgoingWebhookEventMetric for OutgoingWebhookContent {
        /// Returns the type of outgoing webhook event content based on the enum variant of the current instance.
    fn get_outgoing_webhook_event_type(&self) -> Option<OutgoingWebhookEventContent> {
        match self {
            Self::PaymentDetails(payment_payload) => Some(OutgoingWebhookEventContent::Payment {
                payment_id: payment_payload.payment_id.clone(),
                content: masking::masked_serialize(&payment_payload)
                    .unwrap_or(serde_json::json!({"error":"failed to serialize"})),
            }),
            Self::RefundDetails(refund_payload) => Some(OutgoingWebhookEventContent::Refund {
                payment_id: refund_payload.payment_id.clone(),
                refund_id: refund_payload.refund_id.clone(),
                content: masking::masked_serialize(&refund_payload)
                    .unwrap_or(serde_json::json!({"error":"failed to serialize"})),
            }),
            Self::DisputeDetails(dispute_payload) => Some(OutgoingWebhookEventContent::Dispute {
                payment_id: dispute_payload.payment_id.clone(),
                attempt_id: dispute_payload.attempt_id.clone(),
                dispute_id: dispute_payload.dispute_id.clone(),
                content: masking::masked_serialize(&dispute_payload)
                    .unwrap_or(serde_json::json!({"error":"failed to serialize"})),
            }),
            Self::MandateDetails(mandate_payload) => Some(OutgoingWebhookEventContent::Mandate {
                payment_method_id: mandate_payload.payment_method_id.clone(),
                mandate_id: mandate_payload.mandate_id.clone(),
                content: masking::masked_serialize(&mandate_payload)
                    .unwrap_or(serde_json::json!({"error":"failed to serialize"})),
            }),
        }
    }
}

impl OutgoingWebhookEvent {
        /// Creates a new OutgoingWebhookEvent with the provided merchant ID, event ID, event type, event content, error status, and error value. The created_at_timestamp is set to the current UTC time in milliseconds.
    pub fn new(
        merchant_id: String,
        event_id: String,
        event_type: OutgoingWebhookEventType,
        content: Option<OutgoingWebhookEventContent>,
        is_error: bool,
        error: Option<Value>,
    ) -> Self {
        Self {
            merchant_id,
            event_id,
            event_type,
            content,
            is_error,
            error,
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
        }
    }
}

impl TryFrom<OutgoingWebhookEvent> for RawEvent {
    type Error = serde_json::Error;

        /// Attempts to convert an OutgoingWebhookEvent into an instance of the current type.
    /// If successful, returns a Result containing the converted instance, otherwise returns an error.
    fn try_from(value: OutgoingWebhookEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            event_type: EventType::OutgoingWebhookLogs,
            key: value.merchant_id.clone(),
            payload: serde_json::to_value(value)?,
        })
    }
}
