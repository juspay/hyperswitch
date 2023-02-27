use common_utils::custom_serde;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as api_enums, payments};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncomingWebhookEvent {
    PaymentIntentFailure,
    PaymentIntentSuccess,
    EndpointVerification,
}

pub enum WebhookFlow {
    Payment,
    Refund,
    Subscription,
    ReturnResponse,
}

impl From<IncomingWebhookEvent> for WebhookFlow {
    fn from(evt: IncomingWebhookEvent) -> Self {
        match evt {
            IncomingWebhookEvent::PaymentIntentFailure => Self::Payment,
            IncomingWebhookEvent::PaymentIntentSuccess => Self::Payment,
            IncomingWebhookEvent::EndpointVerification => Self::ReturnResponse,
        }
    }
}

pub struct IncomingWebhookRequestDetails<'a> {
    pub method: actix_web::http::Method,
    pub headers: &'a actix_web::http::header::HeaderMap,
    pub body: &'a [u8],
}

pub type MerchantWebhookConfig = std::collections::HashSet<IncomingWebhookEvent>;

pub struct IncomingWebhookDetails {
    pub object_reference_id: String,
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
}
