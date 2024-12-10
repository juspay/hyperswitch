use api_models::{enums::EventType as OutgoingWebhookEventType, webhooks::OutgoingWebhookContent};
use common_enums::WebhookDeliveryAttempt;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;

use super::EventType;
use crate::services::kafka::KafkaMessage;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OutgoingWebhookEvent {
    tenant_id: common_utils::id_type::TenantId,
    merchant_id: common_utils::id_type::MerchantId,
    event_id: String,
    event_type: OutgoingWebhookEventType,
    #[serde(flatten)]
    content: Option<OutgoingWebhookEventContent>,
    is_error: bool,
    error: Option<Value>,
    created_at_timestamp: i128,
    initial_attempt_id: Option<String>,
    status_code: Option<u16>,
    delivery_attempt: Option<WebhookDeliveryAttempt>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "outgoing_webhook_event_type", rename_all = "snake_case")]
pub enum OutgoingWebhookEventContent {
    #[cfg(feature = "v1")]
    Payment {
        payment_id: common_utils::id_type::PaymentId,
        content: Value,
    },
    #[cfg(feature = "v2")]
    Payment {
        payment_id: common_utils::id_type::GlobalPaymentId,
        content: Value,
    },
    Payout {
        payout_id: String,
        content: Value,
    },
    #[cfg(feature = "v1")]
    Refund {
        payment_id: common_utils::id_type::PaymentId,
        refund_id: String,
        content: Value,
    },
    #[cfg(feature = "v2")]
    Refund {
        payment_id: common_utils::id_type::GlobalPaymentId,
        refund_id: String,
        content: Value,
    },
    Dispute {
        payment_id: common_utils::id_type::PaymentId,
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
    fn get_outgoing_webhook_event_content(&self) -> Option<OutgoingWebhookEventContent>;
}

#[cfg(feature = "v1")]
impl OutgoingWebhookEventMetric for OutgoingWebhookContent {
    fn get_outgoing_webhook_event_content(&self) -> Option<OutgoingWebhookEventContent> {
        match self {
            Self::PaymentDetails(payment_payload) => Some(OutgoingWebhookEventContent::Payment {
                payment_id: payment_payload.payment_id.clone(),
                content: masking::masked_serialize(&payment_payload)
                    .unwrap_or(serde_json::json!({"error":"failed to serialize"})),
            }),
            Self::RefundDetails(refund_payload) => Some(OutgoingWebhookEventContent::Refund {
                payment_id: refund_payload.payment_id.clone(),
                refund_id: refund_payload.get_refund_id_as_string(),
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
            #[cfg(feature = "payouts")]
            Self::PayoutDetails(payout_payload) => Some(OutgoingWebhookEventContent::Payout {
                payout_id: payout_payload.payout_id.clone(),
                content: masking::masked_serialize(&payout_payload)
                    .unwrap_or(serde_json::json!({"error":"failed to serialize"})),
            }),
        }
    }
}

#[cfg(feature = "v2")]
impl OutgoingWebhookEventMetric for OutgoingWebhookContent {
    fn get_outgoing_webhook_event_content(&self) -> Option<OutgoingWebhookEventContent> {
        match self {
            Self::PaymentDetails(payment_payload) => Some(OutgoingWebhookEventContent::Payment {
                payment_id: payment_payload.id.clone(),
                content: masking::masked_serialize(&payment_payload)
                    .unwrap_or(serde_json::json!({"error":"failed to serialize"})),
            }),
            Self::RefundDetails(refund_payload) => Some(OutgoingWebhookEventContent::Refund {
                payment_id: refund_payload.payment_id.clone(),
                refund_id: refund_payload.get_refund_id_as_string(),
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
            #[cfg(feature = "payouts")]
            Self::PayoutDetails(payout_payload) => Some(OutgoingWebhookEventContent::Payout {
                payout_id: payout_payload.payout_id.clone(),
                content: masking::masked_serialize(&payout_payload)
                    .unwrap_or(serde_json::json!({"error":"failed to serialize"})),
            }),
        }
    }
}

impl OutgoingWebhookEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: common_utils::id_type::TenantId,
        merchant_id: common_utils::id_type::MerchantId,
        event_id: String,
        event_type: OutgoingWebhookEventType,
        content: Option<OutgoingWebhookEventContent>,
        error: Option<Value>,
        initial_attempt_id: Option<String>,
        status_code: Option<u16>,
        delivery_attempt: Option<WebhookDeliveryAttempt>,
    ) -> Self {
        Self {
            tenant_id,
            merchant_id,
            event_id,
            event_type,
            content,
            is_error: error.is_some(),
            error,
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            initial_attempt_id,
            status_code,
            delivery_attempt,
        }
    }
}

impl KafkaMessage for OutgoingWebhookEvent {
    fn event_type(&self) -> EventType {
        EventType::OutgoingWebhookLogs
    }

    fn key(&self) -> String {
        self.event_id.clone()
    }
}
