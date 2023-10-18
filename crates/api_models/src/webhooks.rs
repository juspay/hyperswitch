use common_utils::custom_serde;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::{disputes, enums as api_enums, mandates, payments, refunds};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
#[serde(rename_all = "snake_case")]
pub enum IncomingWebhookEvent {
    PaymentIntentFailure,
    PaymentIntentSuccess,
    PaymentIntentProcessing,
    PaymentIntentPartiallyFunded,
    PaymentIntentCancelled,
    PaymentActionRequired,
    EventNotSupported,
    SourceChargeable,
    SourceTransactionCreated,
    RefundFailure,
    RefundSuccess,
    DisputeOpened,
    DisputeExpired,
    DisputeAccepted,
    DisputeCancelled,
    DisputeChallenged,
    // dispute has been successfully challenged by the merchant
    DisputeWon,
    // dispute has been unsuccessfully challenged
    DisputeLost,
    MandateActive,
    MandateRevoked,
    EndpointVerification,
}

pub enum WebhookFlow {
    Payment,
    Refund,
    Dispute,
    Subscription,
    ReturnResponse,
    BankTransfer,
    Mandate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// This enum tells about the affect a webhook had on an object
pub enum WebhookResponseTracker {
    Payment {
        payment_id: String,
        status: common_enums::IntentStatus,
    },
    Refund {
        payment_id: String,
        refund_id: String,
        status: common_enums::RefundStatus,
    },
    Dispute {
        dispute_id: String,
        payment_id: String,
        status: common_enums::DisputeStatus,
    },
    Mandate {
        mandate_id: String,
        status: common_enums::MandateStatus,
    },
    NoEffect,
}

impl WebhookResponseTracker {
    pub fn get_payment_id(&self) -> Option<String> {
        match self {
            Self::Payment { payment_id, .. }
            | Self::Refund { payment_id, .. }
            | Self::Dispute { payment_id, .. } => Some(payment_id.to_string()),
            Self::NoEffect | Self::Mandate { .. } => None,
        }
    }
}

impl From<IncomingWebhookEvent> for WebhookFlow {
    fn from(evt: IncomingWebhookEvent) -> Self {
        match evt {
            IncomingWebhookEvent::PaymentIntentFailure
            | IncomingWebhookEvent::PaymentIntentSuccess
            | IncomingWebhookEvent::PaymentIntentProcessing
            | IncomingWebhookEvent::PaymentActionRequired
            | IncomingWebhookEvent::PaymentIntentPartiallyFunded
            | IncomingWebhookEvent::PaymentIntentCancelled => Self::Payment,
            IncomingWebhookEvent::EventNotSupported => Self::ReturnResponse,
            IncomingWebhookEvent::RefundSuccess | IncomingWebhookEvent::RefundFailure => {
                Self::Refund
            }
            IncomingWebhookEvent::MandateActive | IncomingWebhookEvent::MandateRevoked => {
                Self::Mandate
            }
            IncomingWebhookEvent::DisputeOpened
            | IncomingWebhookEvent::DisputeAccepted
            | IncomingWebhookEvent::DisputeExpired
            | IncomingWebhookEvent::DisputeCancelled
            | IncomingWebhookEvent::DisputeChallenged
            | IncomingWebhookEvent::DisputeWon
            | IncomingWebhookEvent::DisputeLost => Self::Dispute,
            IncomingWebhookEvent::EndpointVerification => Self::ReturnResponse,
            IncomingWebhookEvent::SourceChargeable
            | IncomingWebhookEvent::SourceTransactionCreated => Self::BankTransfer,
        }
    }
}

pub type MerchantWebhookConfig = std::collections::HashSet<IncomingWebhookEvent>;

#[derive(Clone)]
pub enum RefundIdType {
    RefundId(String),
    ConnectorRefundId(String),
}

#[derive(Clone)]
pub enum MandateIdType {
    MandateId(String),
    ConnectorMandateId(String),
}

#[derive(Clone)]
pub enum ObjectReferenceId {
    PaymentId(payments::PaymentIdType),
    RefundId(RefundIdType),
    MandateId(MandateIdType),
}

pub struct IncomingWebhookDetails {
    pub object_reference_id: ObjectReferenceId,
    pub resource_object: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OutgoingWebhook {
    /// The merchant id of the merchant
    pub merchant_id: String,

    /// The unique event id for each webhook
    pub event_id: String,

    /// The type of event this webhook corresponds to.
    #[schema(value_type = EventType)]
    pub event_type: api_enums::EventType,

    /// This is specific to the flow, for ex: it will be `PaymentsResponse` for payments flow
    pub content: OutgoingWebhookContent,
    #[serde(default, with = "custom_serde::iso8601")]

    /// The time at which webhook was sent
    pub timestamp: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", content = "object", rename_all = "snake_case")]
pub enum OutgoingWebhookContent {
    #[schema(value_type = PaymentsResponse)]
    PaymentDetails(payments::PaymentsResponse),
    #[schema(value_type = RefundResponse)]
    RefundDetails(refunds::RefundResponse),
    #[schema(value_type = DisputeResponse)]
    DisputeDetails(Box<disputes::DisputeResponse>),
    #[schema(value_type = MandateResponse)]
    MandateDetails(Box<mandates::MandateResponse>),
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectorWebhookSecrets {
    pub secret: Vec<u8>,
    pub additional_secret: Option<masking::Secret<String>>,
}
