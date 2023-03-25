use api_models::webhooks::{self as api};
use serde::Serialize;

use super::{
    payment_intents::types::StripePaymentIntentResponse, refunds::types::StripeRefundResponse,
};

#[derive(Serialize)]
pub struct StripeOutgoingWebhook {
    id: Option<String>,
    #[serde(rename = "type")]
    stype: &'static str,
    data: StripeWebhookObject,
}

impl api::OutgoingWebhookType for StripeOutgoingWebhook {}

#[derive(Serialize)]
#[serde(tag = "type", content = "object", rename_all = "snake_case")]
pub enum StripeWebhookObject {
    PaymentIntent(StripePaymentIntentResponse),
    Refund(StripeRefundResponse),
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
        }
    }
}

impl StripeWebhookObject {
    fn get_id(&self) -> Option<String> {
        match self {
            Self::PaymentIntent(p) => p.id.to_owned(),
            Self::Refund(r) => Some(r.id.to_owned()),
        }
    }
}
