use common_utils::custom_serde;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{disputes, enums as api_enums, payments, refunds};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncomingWebhookEvent {
    PaymentIntentFailure,
    PaymentIntentSuccess,
    PaymentIntentProcessing,
    PaymentIntentPartiallyFunded,
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
    EndpointVerification,
}

pub enum WebhookFlow {
    Payment,
    Refund,
    Dispute,
    Subscription,
    ReturnResponse,
    BankTransfer,
}

impl From<IncomingWebhookEvent> for WebhookFlow {
    fn from(evt: IncomingWebhookEvent) -> Self {
        match evt {
            IncomingWebhookEvent::PaymentIntentFailure => Self::Payment,
            IncomingWebhookEvent::PaymentIntentSuccess => Self::Payment,
            IncomingWebhookEvent::PaymentIntentProcessing => Self::Payment,
            IncomingWebhookEvent::PaymentActionRequired => Self::Payment,
            IncomingWebhookEvent::PaymentIntentPartiallyFunded => Self::Payment,
            IncomingWebhookEvent::EventNotSupported => Self::ReturnResponse,
            IncomingWebhookEvent::RefundSuccess => Self::Refund,
            IncomingWebhookEvent::RefundFailure => Self::Refund,
            IncomingWebhookEvent::DisputeOpened => Self::Dispute,
            IncomingWebhookEvent::DisputeAccepted => Self::Dispute,
            IncomingWebhookEvent::DisputeExpired => Self::Dispute,
            IncomingWebhookEvent::DisputeCancelled => Self::Dispute,
            IncomingWebhookEvent::DisputeChallenged => Self::Dispute,
            IncomingWebhookEvent::DisputeWon => Self::Dispute,
            IncomingWebhookEvent::DisputeLost => Self::Dispute,
            IncomingWebhookEvent::EndpointVerification => Self::ReturnResponse,
            IncomingWebhookEvent::SourceChargeable
            | IncomingWebhookEvent::SourceTransactionCreated => Self::BankTransfer,
        }
    }
}

pub type MerchantWebhookConfig = std::collections::HashSet<IncomingWebhookEvent>;

pub enum RefundIdType {
    RefundId(String),
    ConnectorRefundId(String),
}

pub enum ObjectReferenceId {
    PaymentId(payments::PaymentIdType),
    RefundId(RefundIdType),
}

pub struct IncomingWebhookDetails {
    pub object_reference_id: ObjectReferenceId,
    pub resource_object: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutgoingWebhook {
    pub merchant_id: String,
    pub event_id: String,
    pub event_type: api_enums::EventType,
    pub content: OutgoingWebhookContent,
    #[serde(default, with = "custom_serde::iso8601")]
    pub timestamp: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "object", rename_all = "snake_case")]
pub enum OutgoingWebhookContent {
    PaymentDetails(payments::PaymentsResponse),
    RefundDetails(refunds::RefundResponse),
    DisputeDetails(Box<disputes::DisputeResponse>),
}

pub trait OutgoingWebhookType:
    Serialize + From<OutgoingWebhook> + Sync + Send + std::fmt::Debug
{
}
impl OutgoingWebhookType for OutgoingWebhook {}
