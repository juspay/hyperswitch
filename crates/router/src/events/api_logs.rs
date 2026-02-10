use actix_web::HttpRequest;
use api_models::{admin::MerchantId, organization::OrganizationId};
pub use common_utils::events::{
    ApiEventMetric, ApiEventsType, ExternalServiceCall,
};
use common_utils::{id_type::ProfileId, impl_api_event_type};
use hyperswitch_domain_models::merchant_account;
use router_env::{types::FlowMetric, RequestId};
use serde::Serialize;
use time::OffsetDateTime;

use super::EventType;
#[cfg(feature = "dummy_connector")]
use crate::routes::dummy_connector::types::{
    DummyConnectorPaymentCompleteRequest, DummyConnectorPaymentConfirmRequest,
    DummyConnectorPaymentRequest, DummyConnectorPaymentResponse,
    DummyConnectorPaymentRetrieveRequest, DummyConnectorRefundRequest,
    DummyConnectorRefundResponse, DummyConnectorRefundRetrieveRequest,
};
use crate::{
    core::payments::PaymentsRedirectResponseData,
    services::{authentication::AuthenticationType, kafka::KafkaMessage},
    types::api::{
        AttachEvidenceRequest, Config, ConfigUpdate, CreateFileRequest, DisputeFetchQueryData,
        DisputeId, FileId, FileRetrieveRequest, PollId,
    },
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ApiEvent {
    tenant_id: common_utils::id_type::TenantId,
    merchant_id: Option<common_utils::id_type::MerchantId>,
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
    http_method: String,
    #[serde(flatten)]
    infra_components: Option<serde_json::Value>,
}

impl ApiEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: common_utils::id_type::TenantId,
        merchant_id: Option<common_utils::id_type::MerchantId>,
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
        http_method: &http::Method,
        infra_components: Option<serde_json::Value>,
    ) -> Self {
        Self {
            tenant_id,
            merchant_id,
            api_flow: api_flow.to_string(),
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            request_id: request_id.to_string(),
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
            http_method: http_method.to_string(),
            infra_components,
        }
    }
}

impl KafkaMessage for ApiEvent {
    fn event_type(&self) -> EventType {
        EventType::ApiLogs
    }

    fn key(&self) -> String {
        self.request_id.clone()
    }
}

impl_api_event_type!(
    Miscellaneous,
    (
        Config,
        CreateFileRequest,
        FileId,
        FileRetrieveRequest,
        AttachEvidenceRequest,
        DisputeFetchQueryData,
        ConfigUpdate
    )
);

#[cfg(feature = "dummy_connector")]
impl_api_event_type!(
    Miscellaneous,
    (
        DummyConnectorPaymentCompleteRequest,
        DummyConnectorPaymentRequest,
        DummyConnectorPaymentResponse,
        DummyConnectorPaymentRetrieveRequest,
        DummyConnectorPaymentConfirmRequest,
        DummyConnectorRefundRetrieveRequest,
        DummyConnectorRefundResponse,
        DummyConnectorRefundRequest
    )
);

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsRedirectResponseData {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentRedirectionResponse {
            connector: self.connector.clone(),
            payment_id: match &self.resource_id {
                api_models::payments::PaymentIdType::PaymentIntentId(id) => Some(id.clone()),
                _ => None,
            },
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentsRedirectResponseData {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentRedirectionResponse {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for DisputeId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Dispute {
            dispute_id: self.dispute_id.clone(),
        })
    }
}

impl ApiEventMetric for PollId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Poll {
            poll_id: self.poll_id.clone(),
        })
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct NewApiEvent{
    pub tenant_id: Option<common_utils::id_type::TenantId>,
    pub merchant_id: Option<common_utils::id_type::MerchantId>,
    pub api_flow: Option<String>,
    pub created_at_timestamp: i128,
    pub request_id: Option<String>,
    pub latency: u128,
    pub status_code: i64,
    #[serde(flatten)]
    pub auth_type: Option<AuthenticationType>,
    pub request: Option<String>,
    pub user_agent: Option<String>,
    pub ip_addr: Option<String>,
    pub url_path: Option<String>,
    pub response: Option<String>,
    pub error: Option<serde_json::Value>,
    #[serde(flatten)]
    pub event_type: Option<ApiEventsType>,
    pub hs_latency: Option<u128>,
    pub http_method: Option<String>,
    #[serde(flatten)]
    pub infra_components: Option<serde_json::Value>,
    pub external_service_calls: Vec<ExternalServiceCall>,
}


impl From<ApiEvent> for NewApiEvent {
    fn from(api_event: ApiEvent) -> Self {
        Self {
            tenant_id: Some(api_event.tenant_id),
            merchant_id: api_event.merchant_id,
            api_flow: Some(api_event.api_flow),
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            request_id: Some(api_event.request_id),
            latency: api_event.latency,
            status_code: api_event.status_code,
            auth_type: Some(api_event.auth_type),
            request: Some(api_event.request),
            user_agent: api_event.user_agent,
            ip_addr: api_event.ip_addr,
            url_path: Some(api_event.url_path),
            response: api_event.response,
            error: api_event.error,
            event_type: Some(api_event.event_type),
            hs_latency: api_event.hs_latency,
            http_method: Some(api_event.http_method),
            infra_components: api_event.infra_components,
            external_service_calls: Vec::new(),
        }
    }
}

impl KafkaMessage for NewApiEvent {
    fn event_type(&self) -> EventType {
        EventType::NewApiLogs
    }

    fn key(&self) -> String {
        self.request_id.clone().unwrap_or_default()
    }
}

// #[derive(Debug, Serialize)]
// #[serde(rename_all = "snake_case")]
// pub struct ObservabilityContext {
//     pub merchant_id: Option<common_utils::id_type::MerchantId>,
//     pub organization_id: Option<common_utils::id_type::OrganizationId>,
//     pub profile_id: Option<common_utils::id_type::ProfileId>,
// }
