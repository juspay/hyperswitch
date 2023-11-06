mod customer;
mod payment;
mod payouts;
mod refund;
mod routing;
use api_models::{
    admin::{
        BusinessProfileCreate, BusinessProfileResponse, BusinessProfileUpdate,
        MerchantAccountCreate, MerchantAccountDeleteResponse, MerchantAccountListRequest,
        MerchantAccountResponse, MerchantAccountUpdate, MerchantConnectorCreate,
        MerchantConnectorDeleteResponse, MerchantConnectorId, MerchantConnectorResponse,
        MerchantConnectorUpdate, MerchantId, ToggleKVRequest, ToggleKVResponse,
    },
    api_keys::{
        CreateApiKeyRequest, CreateApiKeyResponse, RetrieveApiKeyResponse, RevokeApiKeyResponse,
        UpdateApiKeyRequest,
    },
    cards_info::{CardInfoResponse, CardsInfoRequest},
    disputes::{
        DisputeEvidenceBlock, DisputeListConstraints, DisputeResponse, SubmitEvidenceRequest,
    },
    enums as api_enums,
    files::CreateFileResponse,
    mandates::{MandateId, MandateListConstraints, MandateResponse, MandateRevokedResponse},
    payment_methods::{PaymentMethodCreate, PaymentMethodId, PaymentMethodListResponse},
    payments::{
        PaymentLinkInitiateRequest, PaymentsSessionRequest, PaymentsSessionResponse,
        RetrievePaymentLinkRequest, RetrievePaymentLinkResponse,
    },
    refunds::RefundUpdateRequest,
    verifications::{
        ApplepayMerchantResponse, ApplepayMerchantVerificationRequest,
        ApplepayVerifiedDomainsResponse,
    },
};
use diesel_models::EphemeralKey;
use router_env::{tracing_actix_web::RequestId, types::FlowMetric};
use serde::Serialize;
use time::OffsetDateTime;

use super::{EventType, RawEvent};
use crate::{
    compatibility::stripe::{
        customers::types::{
            CreateCustomerResponse, CustomerDeleteResponse, CustomerPaymentMethodListResponse,
        },
        payment_intents::types::{StripePaymentIntentListResponse, StripePaymentIntentResponse},
        refunds::types::StripeRefundResponse,
        setup_intents::types::StripeSetupIntentResponse,
    },
    routes::dummy_connector::types::{
        DummyConnectorPaymentCompleteRequest, DummyConnectorPaymentConfirmRequest,
        DummyConnectorPaymentRequest, DummyConnectorPaymentResponse,
        DummyConnectorPaymentRetrieveRequest, DummyConnectorRefundRequest,
        DummyConnectorRefundResponse, DummyConnectorRefundRetrieveRequest,
    },
    services::{authentication::AuthenticationType, ApplicationResponse, PaymentLinkFormData},
    types::api::{
        AttachEvidenceRequest, Config, ConfigUpdate, CreateFileRequest, CustomerResponse,
        DisputeId, FileId,
    },
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ApiEvent {
    api_flow: String,
    created_at_timestamp: i128,
    request_id: String,
    latency: u128,
    status_code: i64,
    #[serde(flatten)]
    auth_type: AuthenticationType,
    request: serde_json::Value,
    response: Option<serde_json::Value>,
    #[serde(flatten)]
    event_type: ApiEventsType,
}

impl ApiEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_flow: &impl FlowMetric,
        request_id: &RequestId,
        latency: u128,
        status_code: i64,
        request: serde_json::Value,
        response: Option<serde_json::Value>,
        auth_type: AuthenticationType,
        event_type: ApiEventsType,
    ) -> Self {
        Self {
            api_flow: api_flow.to_string(),
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos(),
            request_id: request_id.as_hyphenated().to_string(),
            latency,
            status_code,
            request,
            response,
            auth_type,
            event_type,
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

pub trait ApiEventMetric {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "flow_type")]
pub enum ApiEventsType {
    Payout,
    Payment {
        payment_id: String,
    },
    Refund {
        payment_id: Option<String>,
        refund_id: String,
    },
    PaymentMethod {
        payment_method_id: String,
        payment_method: Option<api_enums::PaymentMethod>,
        payment_method_type: Option<api_enums::PaymentMethodType>,
    },
    Customer {
        customer_id: String,
    },
    User {
        //specified merchant_id will overridden on global defined
        merchant_id: String,
        user_id: String,
    },
    PaymentMethodList {
        payment_id: Option<String>,
    },
    Webhooks {
        connector: String,
        payment_id: Option<String>,
    },
    Routing,
    ResourceListAPI,
    PaymentRedirectionResponse,
    // TODO: This has to be removed once the corresponding apiEventTypes are created
    Miscellaneous,
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

impl ApiEventMetric for serde_json::Value {}
impl ApiEventMetric for () {}
impl ApiEventMetric for api_models::payments::TimeRange {}

impl<Q: ApiEventMetric, E> ApiEventMetric for Result<Q, E> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        match self {
            Ok(q) => q.get_api_event_type(),
            Err(_) => None,
        }
    }
}

macro_rules! impl_misc_api_event_type {
    ($($type:ty),+) => {
        $(
            impl ApiEventMetric for $type {
                fn get_api_event_type(&self) -> Option<ApiEventsType> {
                    Some(ApiEventsType::Miscellaneous)
                }
            }
        )+
     };
}

impl_misc_api_event_type!(
    PaymentMethodId,
    PaymentsSessionResponse,
    PaymentMethodListResponse,
    PaymentMethodCreate,
    PaymentLinkFormData,
    PaymentLinkInitiateRequest,
    RetrievePaymentLinkResponse,
    MandateListConstraints,
    DummyConnectorPaymentCompleteRequest,
    DummyConnectorPaymentRequest,
    DummyConnectorPaymentResponse,
    DummyConnectorPaymentRetrieveRequest,
    DummyConnectorPaymentConfirmRequest,
    Vec<DisputeEvidenceBlock>,
    DisputeId,
    CreateFileResponse,
    AttachEvidenceRequest,
    DisputeResponse,
    SubmitEvidenceRequest,
    bool,
    MerchantConnectorResponse,
    MerchantConnectorId,
    MandateResponse,
    FileId,
    MandateRevokedResponse,
    DummyConnectorRefundResponse,
    RetrievePaymentLinkRequest,
    MandateId,
    DummyConnectorRefundRetrieveRequest,
    DummyConnectorRefundRequest,
    EphemeralKey,
    String,
    DisputeListConstraints,
    RetrieveApiKeyResponse,
    Vec<BusinessProfileResponse>,
    BusinessProfileResponse,
    BusinessProfileUpdate,
    BusinessProfileCreate,
    CreateFileRequest,
    CustomerResponse,
    Config,
    ConfigUpdate,
    RevokeApiKeyResponse,
    ToggleKVResponse,
    ToggleKVRequest,
    Vec<DisputeResponse>,
    MerchantAccountDeleteResponse,
    MerchantAccountUpdate,
    Vec<CustomerResponse>,
    CardInfoResponse,
    Vec<RetrieveApiKeyResponse>,
    (Option<i64>, Option<i64>, String),
    (&String, &String),
    (&String, &String, UpdateApiKeyRequest),
    CreateApiKeyResponse,
    CreateApiKeyRequest,
    (String, ToggleKVRequest),
    MerchantConnectorDeleteResponse,
    MerchantConnectorUpdate,
    Vec<MerchantConnectorResponse>,
    MerchantConnectorCreate,
    MerchantId,
    CardsInfoRequest,
    MerchantAccountResponse,
    Vec<MerchantAccountResponse>,
    MerchantAccountListRequest,
    MerchantAccountCreate,
    PaymentsSessionRequest,
    ApplepayMerchantVerificationRequest,
    ApplepayMerchantResponse,
    ApplepayVerifiedDomainsResponse,
    StripeSetupIntentResponse,
    StripeRefundResponse,
    StripePaymentIntentListResponse,
    StripePaymentIntentResponse,
    CustomerDeleteResponse,
    CustomerPaymentMethodListResponse,
    CreateCustomerResponse,
    RefundUpdateRequest
);

impl<T: ApiEventMetric> ApiEventMetric for &T {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        T::get_api_event_type(self)
    }
}
