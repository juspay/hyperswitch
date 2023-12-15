use api_models::{
    disputes, enums::EventType as OutgoingWebhookEventType, mandates, payments, refunds,
    webhooks::OutgoingWebhookContent,
};
use serde::Serialize;
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
    error: Option<serde_json::Value>,
    created_at_timestamp: i128,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "outgoing_webhook_event_type",
    content = "payload",
    rename_all = "snake_case"
)]
pub enum OutgoingWebhookEventContent {
    Payment {
        payment_id: Option<String>,
        content: payments::PaymentsResponse,
    },
    Refund {
        payment_id: String,
        refund_id: String,
        content: refunds::RefundResponse,
    },
    Dispute {
        payment_id: String,
        attempt_id: String,
        dispute_id: String,
        content: disputes::DisputeResponse,
    },
    Mandate {
        payment_method_id: String,
        mandate_id: String,
        content: mandates::MandateResponse,
    },
}
pub trait OutgoingWebhookEventMetric {
    fn get_outgoing_webhook_event_type(&self) -> Option<OutgoingWebhookEventContent> {
        None
    }
}
impl OutgoingWebhookEventMetric for OutgoingWebhookContent {
    fn get_outgoing_webhook_event_type(&self) -> Option<OutgoingWebhookEventContent> {
        match self {
            Self::PaymentDetails(reponse) => Some(OutgoingWebhookEventContent::Payment {
                payment_id: reponse.payment_id.clone(),
                content: reponse.clone(),
            }),
            Self::RefundDetails(reponse) => Some(OutgoingWebhookEventContent::Refund {
                payment_id: reponse.payment_id.clone(),
                refund_id: reponse.refund_id.clone(),
                content: reponse.clone(),
            }),
            Self::DisputeDetails(reponse) => Some(OutgoingWebhookEventContent::Dispute {
                payment_id: reponse.payment_id.clone(),
                attempt_id: reponse.attempt_id.clone(),
                dispute_id: reponse.dispute_id.clone(),
                content: *reponse.clone(),
            }),
            Self::MandateDetails(reponse) => Some(OutgoingWebhookEventContent::Mandate {
                payment_method_id: reponse.payment_method_id.clone(),
                mandate_id: reponse.mandate_id.clone(),
                content: *reponse.clone(),
            }),
        }
    }
}

impl OutgoingWebhookEvent {
    pub fn new(
        merchant_id: String,
        event_id: String,
        event_type: OutgoingWebhookEventType,
        content: Option<OutgoingWebhookEventContent>,
        is_error: bool,
        error: Option<serde_json::Value>,
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

    fn try_from(value: OutgoingWebhookEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            event_type: EventType::OutgoingWebhookLogs,
            key: value.merchant_id.clone(),
            payload: serde_json::to_value(value)?,
        })
    }
}
