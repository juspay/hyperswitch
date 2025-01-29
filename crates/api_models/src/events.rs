pub mod apple_pay_certificates_migration;
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
    impl_api_event_type,
};

use crate::customers::CustomerListRequest;
#[allow(unused_imports)]
use crate::{
    admin::*,
    analytics::{
        api_event::*, auth_events::*, connector_events::ConnectorEventsRequest,
        outgoing_webhook_event::OutgoingWebhookLogsRequest, sdk_events::*, search::*, *,
    },
    api_keys::*,
    cards_info::*,
    disputes::*,
    files::*,
    mandates::*,
    organization::{
        OrganizationCreateRequest, OrganizationId, OrganizationResponse, OrganizationUpdateRequest,
    },
    payment_methods::*,
    payments::*,
    user::{UserKeyTransferRequest, UserTransferKeyResponse},
    verifications::*,
};

impl ApiEventMetric for GetPaymentIntentFiltersRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Analytics)
    }
}

impl ApiEventMetric for GetPaymentIntentMetricRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Analytics)
    }
}

impl ApiEventMetric for PaymentIntentFiltersResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Analytics)
    }
}

impl_api_event_type!(
    Miscellaneous,
    (
        PaymentMethodId,
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
        DisputeListGetConstraints,
        RetrieveApiKeyResponse,
        ProfileResponse,
        ProfileUpdate,
        ProfileCreate,
        RevokeApiKeyResponse,
        ToggleKVResponse,
        ToggleKVRequest,
        ToggleAllKVRequest,
        ToggleAllKVResponse,
        MerchantAccountDeleteResponse,
        MerchantAccountUpdate,
        CardInfoResponse,
        CreateApiKeyResponse,
        CreateApiKeyRequest,
        ListApiKeyConstraints,
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
        GetActivePaymentsMetricRequest,
        GetSdkEventMetricRequest,
        GetAuthEventMetricRequest,
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
        OutgoingWebhookLogsRequest,
        GetGlobalSearchRequest,
        GetSearchRequest,
        GetSearchResponse,
        GetSearchRequestWithIndex,
        GetDisputeFilterRequest,
        DisputeFiltersResponse,
        GetDisputeMetricRequest,
        SankeyResponse,
        OrganizationResponse,
        OrganizationCreateRequest,
        OrganizationUpdateRequest,
        OrganizationId,
        CustomerListRequest
    )
);

impl_api_event_type!(
    Keymanager,
    (
        TransferKeyResponse,
        MerchantKeyTransferRequest,
        UserKeyTransferRequest,
        UserTransferKeyResponse
    )
);

impl<T> ApiEventMetric for MetricsResponse<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

impl<T> ApiEventMetric for PaymentsMetricsResponse<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

impl<T> ApiEventMetric for PaymentIntentsMetricsResponse<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

impl<T> ApiEventMetric for RefundsMetricsResponse<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

impl<T> ApiEventMetric for DisputesMetricsResponse<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl ApiEventMetric for PaymentMethodIntentConfirmInternal {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.id.clone(),
            payment_method_type: Some(self.request.payment_method_type),
            payment_method_subtype: Some(self.request.payment_method_subtype),
        })
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl ApiEventMetric for PaymentMethodIntentCreate {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethodCreate)
    }
}

impl ApiEventMetric for DisputeListFilters {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentMethodSessionRequest {}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentMethodsSessionResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethodSession {
            payment_method_session_id: self.id.clone(),
        })
    }
}
