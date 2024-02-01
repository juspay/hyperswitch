pub mod connector_onboarding;
pub mod customer;
pub mod dispute;
pub mod gsm;
mod locker_migration;
pub mod payment;
#[cfg(feature = "payouts")]
pub mod payouts;
#[cfg(feature = "recon")]
pub mod recon;
pub mod refund;
pub mod routing;
pub mod user;
pub mod user_role;

use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    impl_misc_api_event_type,
};

#[allow(unused_imports)]
use crate::{
    admin::*,
    analytics::{
        api_event::*, connector_events::ConnectorEventsRequest,
        outgoing_webhook_event::OutgoingWebhookLogsRequest, sdk_events::*, *,
    },
    api_keys::*,
    cards_info::*,
    disputes::*,
    files::*,
    mandates::*,
    payment_methods::*,
    payments::*,
    verifications::*,
};

impl ApiEventMetric for TimeRange {}

impl_misc_api_event_type!(
    PaymentMethodId,
    PaymentsSessionResponse,
    PaymentMethodCreate,
    PaymentLinkInitiateRequest,
    RetrievePaymentLinkResponse,
    MandateListConstraints,
    CreateFileResponse,
    MerchantConnectorResponse,
    MerchantConnectorId,
    MandateResponse,
    MandateRevokedResponse,
    RetrievePaymentLinkRequest,
    PaymentLinkListConstraints,
    MandateId,
    DisputeListConstraints,
    RetrieveApiKeyResponse,
    BusinessProfileResponse,
    BusinessProfileUpdate,
    BusinessProfileCreate,
    RevokeApiKeyResponse,
    ToggleKVResponse,
    ToggleKVRequest,
    MerchantAccountDeleteResponse,
    MerchantAccountUpdate,
    CardInfoResponse,
    CreateApiKeyResponse,
    CreateApiKeyRequest,
    MerchantConnectorDeleteResponse,
    MerchantConnectorUpdate,
    MerchantConnectorCreate,
    MerchantId,
    CardsInfoRequest,
    MerchantAccountResponse,
    MerchantAccountListRequest,
    MerchantAccountCreate,
    PaymentsSessionRequest,
    ApplepayMerchantVerificationRequest,
    ApplepayMerchantResponse,
    ApplepayVerifiedDomainsResponse,
    UpdateApiKeyRequest,
    GetApiEventFiltersRequest,
    ApiEventFiltersResponse,
    GetInfoResponse,
    GetPaymentMetricRequest,
    GetRefundMetricRequest,
    GetSdkEventMetricRequest,
    GetPaymentFiltersRequest,
    PaymentFiltersResponse,
    GetRefundFilterRequest,
    RefundFiltersResponse,
    GetSdkEventFiltersRequest,
    SdkEventFiltersResponse,
    ApiLogsRequest,
    GetApiEventMetricRequest,
    SdkEventsRequest,
    ReportRequest,
    ConnectorEventsRequest,
    OutgoingWebhookLogsRequest
);

#[cfg(feature = "stripe")]
impl_misc_api_event_type!(
    StripeSetupIntentResponse,
    StripeRefundResponse,
    StripePaymentIntentListResponse,
    StripePaymentIntentResponse,
    CustomerDeleteResponse,
    CustomerPaymentMethodListResponse,
    CreateCustomerResponse
);

impl<T> ApiEventMetric for MetricsResponse<T> {
        /// Retrieves the API event type. 
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}
