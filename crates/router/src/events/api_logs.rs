use actix_web::HttpRequest;
pub use common_utils::events::{ApiEventMetric, ApiEventsType};
use common_utils::impl_misc_api_event_type;
use router_env::{tracing_actix_web::RequestId, types::FlowMetric};
use serde::Serialize;
use time::OffsetDateTime;

use super::{EventType, RawEvent};
#[cfg(feature = "dummy_connector")]
use crate::routes::dummy_connector::types::{
    DummyConnectorPaymentCompleteRequest, DummyConnectorPaymentConfirmRequest,
    DummyConnectorPaymentRequest, DummyConnectorPaymentResponse,
    DummyConnectorPaymentRetrieveRequest, DummyConnectorRefundRequest,
    DummyConnectorRefundResponse, DummyConnectorRefundRetrieveRequest,
};
use crate::{
    core::payments::PaymentsRedirectResponseData,
    services::{authentication::AuthenticationType, ApplicationResponse, PaymentLinkFormData},
    types::api::{
        AttachEvidenceRequest, Config, ConfigUpdate, CreateFileRequest, DisputeId, FileId,
    },
};
use api_models::{enums::EventType as OutgoingWebhookEventType, webhooks::OutgoingWebhookContent, payments, refunds, disputes, mandates};
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ApiEvent {
    merchant_id: Option<String>,
    api_flow: String,
    created_at_timestamp: i128,
    request_id: String,
    latency: u128,
    status_code: i64,
    #[serde(flatten)]
    auth_type: AuthenticationType,
    request: String,
    user_agent: Option<String>,
    ip_addr: Option<String>,
    url_path: String,
    response: Option<String>,
    error: Option<serde_json::Value>,
    #[serde(flatten)]
    event_type: ApiEventsType,
    hs_latency: Option<u128>,
    http_method: Option<String>,
}

impl ApiEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        merchant_id: Option<String>,
        api_flow: &impl FlowMetric,
        request_id: &RequestId,
        latency: u128,
        status_code: i64,
        request: serde_json::Value,
        response: Option<serde_json::Value>,
        hs_latency: Option<u128>,
        auth_type: AuthenticationType,
        error: Option<serde_json::Value>,
        event_type: ApiEventsType,
        http_req: &HttpRequest,
        http_method: Option<String>,
    ) -> Self {
        Self {
            merchant_id,
            api_flow: api_flow.to_string(),
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            request_id: request_id.as_hyphenated().to_string(),
            latency,
            status_code,
            request: request.to_string(),
            response: response.map(|resp| resp.to_string()),
            auth_type,
            error,
            ip_addr: http_req
                .connection_info()
                .realip_remote_addr()
                .map(ToOwned::to_owned),
            user_agent: http_req
                .headers()
                .get("user-agent")
                .and_then(|user_agent_value| user_agent_value.to_str().ok().map(ToOwned::to_owned)),
            url_path: http_req.path().to_string(),
            event_type,
            hs_latency,
            http_method,
        }
    }
}

impl TryFrom<ApiEvent> for RawEvent {
    type Error = serde_json::Error;

    fn try_from(value: ApiEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            event_type: EventType::ApiLogs,
            key: value.request_id.clone(),
            payload: serde_json::to_value(value)?,
        })
    }
}

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
pub enum OutgoingWebhookEventContent{
    Payment{payment_id: Option<String>,content: payments::PaymentsResponse,},
    Refund{payment_id: String, refund_id: String, content: refunds::RefundResponse,},
    Dispute{payment_id: String, attempt_id: String, dispute_id: String, content: disputes::DisputeResponse,},
    Mandate{payment_method_id: String, mandate_id: String, content: mandates::MandateResponse,},
}
pub trait OutgoingWebhookEventMetric {
    fn get_outgoing_webhook_event_type(&self) -> Option<OutgoingWebhookEventContent> {
        None
    }
}
impl OutgoingWebhookEventMetric for OutgoingWebhookContent{
    fn get_outgoing_webhook_event_type(&self) -> Option<OutgoingWebhookEventContent> {
        match self {
            Self::PaymentDetails(reponse) => Some(OutgoingWebhookEventContent::Payment{
                payment_id: reponse.payment_id.clone(),
                content: reponse.clone(),
            }),
            Self::RefundDetails(reponse) => Some(OutgoingWebhookEventContent::Refund{
                payment_id: reponse.payment_id.clone(),
                refund_id: reponse.refund_id.clone(),
                content: reponse.clone(),
            }),
            Self::DisputeDetails(reponse) => Some(OutgoingWebhookEventContent::Dispute{
                payment_id: reponse.payment_id.clone(),
                attempt_id: reponse.attempt_id.clone(),
                dispute_id: reponse.dispute_id.clone(),
                content: *reponse.clone(),
            }),
            Self::MandateDetails(reponse) => Some(OutgoingWebhookEventContent::Mandate{
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




impl<T: ApiEventMetric> ApiEventMetric for ApplicationResponse<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        match self {
            Self::Json(r) => r.get_api_event_type(),
            Self::JsonWithHeaders((r, _)) => r.get_api_event_type(),
            _ => None,
        }
    }
}
impl_misc_api_event_type!(
    Config,
    CreateFileRequest,
    FileId,
    AttachEvidenceRequest,
    DisputeId,
    PaymentLinkFormData,
    PaymentsRedirectResponseData,
    ConfigUpdate
);

#[cfg(feature = "dummy_connector")]
impl_misc_api_event_type!(
    DummyConnectorPaymentCompleteRequest,
    DummyConnectorPaymentRequest,
    DummyConnectorPaymentResponse,
    DummyConnectorPaymentRetrieveRequest,
    DummyConnectorPaymentConfirmRequest,
    DummyConnectorRefundRetrieveRequest,
    DummyConnectorRefundResponse,
    DummyConnectorRefundRequest
);
