use api_models::{
    enums::DisputeStatus,
    webhooks::{self as api},
};
use serde::Serialize;

use super::{
    payment_intents::types::StripePaymentIntentResponse, refunds::types::StripeRefundResponse,
};

#[derive(Serialize, Debug)]
pub struct StripeOutgoingWebhook {
    id: Option<String>,
    #[serde(rename = "type")]
    stype: &'static str,
    data: StripeWebhookObject,
}

impl api::OutgoingWebhookType for StripeOutgoingWebhook {}

#[derive(Serialize, Debug)]
#[serde(tag = "type", content = "object", rename_all = "snake_case")]
pub enum StripeWebhookObject {
    PaymentIntent(StripePaymentIntentResponse),
    Refund(StripeRefundResponse),
    Dispute(StripeDisputeResponse),
}

#[derive(Serialize, Debug)]
pub struct StripeDisputeResponse {
    pub id: String,
    pub amount: String,
    pub currency: String,
    pub payment_intent: String,
    pub reason: Option<String>,
    pub status: StripeDisputeStatus,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StripeDisputeStatus {
    WarningNeedsResponse,
    WarningUnderReview,
    WarningClosed,
    NeedsResponse,
    UnderReview,
    ChargeRefunded,
    Won,
    Lost,
}

impl From<api_models::disputes::DisputeResponse> for StripeDisputeResponse {
    fn from(res: api_models::disputes::DisputeResponse) -> Self {
        Self {
            id: res.dispute_id,
            amount: res.amount,
            currency: res.currency,
            payment_intent: res.payment_id,
            reason: res.connector_reason,
            status: StripeDisputeStatus::from(res.dispute_status),
        }
    }
}

impl From<DisputeStatus> for StripeDisputeStatus {
    fn from(status: DisputeStatus) -> Self {
        match status {
            DisputeStatus::DisputeOpened => Self::WarningNeedsResponse,
            DisputeStatus::DisputeExpired => Self::Lost,
            DisputeStatus::DisputeAccepted => Self::Lost,
            DisputeStatus::DisputeCancelled => Self::WarningClosed,
            DisputeStatus::DisputeChallenged => Self::WarningUnderReview,
            DisputeStatus::DisputeWon => Self::Won,
            DisputeStatus::DisputeLost => Self::Lost,
        }
    }
}

impl From<api::OutgoingWebhook> for StripeOutgoingWebhook {
    fn from(value: api::OutgoingWebhook) -> Self {
        let data: StripeWebhookObject = value.content.into();
        Self {
            id: data.get_id(),
            stype: "webhook_endpoint",
            data,
        }
    }
}

impl From<api::OutgoingWebhookContent> for StripeWebhookObject {
    fn from(value: api::OutgoingWebhookContent) -> Self {
        match value {
            api::OutgoingWebhookContent::PaymentDetails(payment) => {
                Self::PaymentIntent(payment.into())
            }
            api::OutgoingWebhookContent::RefundDetails(refund) => Self::Refund(refund.into()),
            api::OutgoingWebhookContent::DisputeDetails(dispute) => {
                Self::Dispute((*dispute).into())
            }
        }
    }
}

impl StripeWebhookObject {
    fn get_id(&self) -> Option<String> {
        match self {
            Self::PaymentIntent(p) => p.id.to_owned(),
            Self::Refund(r) => Some(r.id.to_owned()),
            Self::Dispute(d) => Some(d.id.to_owned()),
        }
    }
}
