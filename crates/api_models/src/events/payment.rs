use common_utils::events::{ApiEventMetric, ApiEventsType};

#[cfg(feature = "v2")]
use super::{
    PaymentAttemptListRequest, PaymentAttemptListResponse, PaymentStartRedirectionRequest,
    PaymentsCreateIntentRequest, PaymentsGetIntentRequest, PaymentsIntentResponse, PaymentsRequest,
    RecoveryPaymentListResponse, RecoveryPaymentsCreate, RecoveryPaymentsResponse,
    RevenueRecoveryGetIntentResponse,
};
#[cfg(feature = "v2")]
use crate::payment_methods::{
    ListMethodsForPaymentMethodsRequest, PaymentMethodListResponseForSession,
};
use crate::{
    payment_methods::{
        self, ListCountriesCurrenciesRequest, ListCountriesCurrenciesResponse,
        PaymentMethodCollectLinkRenderRequest, PaymentMethodCollectLinkRequest,
        PaymentMethodCollectLinkResponse, PaymentMethodMigrateResponse, PaymentMethodResponse,
        PaymentMethodUpdate,
    },
    payments::{
        self, PaymentListConstraints, PaymentListFilters, PaymentListFiltersV2,
        PaymentListResponse, PaymentsAggregateResponse, PaymentsSessionResponse,
        RedirectionResponse,
    },
};
#[cfg(feature = "v1")]
use crate::{
    payment_methods::{
        CustomerPaymentMethodUpdateResponse, PaymentMethodListRequest, PaymentMethodListResponse,
    },
    payments::{
        ExtendedCardInfoResponse, PaymentIdType, PaymentListFilterConstraints,
        PaymentListResponseV2, PaymentsApproveRequest, PaymentsCancelPostCaptureRequest,
        PaymentsCancelRequest, PaymentsCaptureRequest, PaymentsCompleteAuthorizeRequest,
        PaymentsDynamicTaxCalculationRequest, PaymentsDynamicTaxCalculationResponse,
        PaymentsExtendAuthorizationRequest, PaymentsExternalAuthenticationRequest,
        PaymentsExternalAuthenticationResponse, PaymentsIncrementalAuthorizationRequest,
        PaymentsManualUpdateRequest, PaymentsManualUpdateResponse,
        PaymentsPostSessionTokensRequest, PaymentsPostSessionTokensResponse, PaymentsRejectRequest,
        PaymentsRetrieveRequest, PaymentsStartRequest, PaymentsUpdateMetadataRequest,
        PaymentsUpdateMetadataResponse,
    },
};

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsRetrieveRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        match self.resource_id {
            PaymentIdType::PaymentIntentId(ref id) => Some(ApiEventsType::Payment {
                payment_id: id.clone(),
            }),
            _ => None,
        }
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsStartRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsCaptureRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.to_owned(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsCompleteAuthorizeRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsDynamicTaxCalculationRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsPostSessionTokensRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsUpdateMetadataRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsUpdateMetadataResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsPostSessionTokensResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsDynamicTaxCalculationResponse {}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsCancelRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsCancelPostCaptureRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}
#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsExtendAuthorizationRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsApproveRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsRejectRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for payments::PaymentsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        match self.payment_id {
            Some(PaymentIdType::PaymentIntentId(ref id)) => Some(ApiEventsType::Payment {
                payment_id: id.clone(),
            }),
            _ => None,
        }
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for payments::PaymentsEligibilityRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for payments::PaymentsEligibilityResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentsCreateIntentRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for payments::CheckAndApplyPaymentMethodDataResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentsGetIntentRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentAttemptListRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_intent_id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentAttemptListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentsIntentResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(all(feature = "v2", feature = "olap"))]
impl ApiEventMetric for RevenueRecoveryGetIntentResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for payments::PaymentsResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for payments::PaymentsCancelRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for payments::PaymentsCancelResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for payments::PaymentsResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentMethodResponse {
    #[cfg(feature = "v1")]
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
        })
    }

    #[cfg(feature = "v2")]
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.id.clone(),
            payment_method_type: self.payment_method_type,
            payment_method_subtype: self.payment_method_subtype,
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for CustomerPaymentMethodUpdateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
        })
    }
}

impl ApiEventMetric for PaymentMethodMigrateResponse {
    #[cfg(feature = "v1")]
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_response.payment_method_id.clone(),
            payment_method: self.payment_method_response.payment_method,
            payment_method_type: self.payment_method_response.payment_method_type,
        })
    }

    #[cfg(feature = "v2")]
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_response.id.clone(),
            payment_method_type: self.payment_method_response.payment_method_type,
            payment_method_subtype: self.payment_method_response.payment_method_subtype,
        })
    }
}

impl ApiEventMetric for PaymentMethodUpdate {}

#[cfg(feature = "v1")]
impl ApiEventMetric for payment_methods::DefaultPaymentMethod {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: None,
            payment_method_type: None,
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for payment_methods::PaymentMethodDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.id.clone(),
            payment_method_type: None,
            payment_method_subtype: None,
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for payment_methods::PaymentMethodDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: None,
            payment_method_type: None,
        })
    }
}

impl ApiEventMetric for payment_methods::CustomerPaymentMethodsListResponse {}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentMethodListRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethodList {
            payment_id: self
                .client_secret
                .as_ref()
                .and_then(|cs| cs.rsplit_once("_secret_"))
                .map(|(pid, _)| pid.to_string()),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for ListMethodsForPaymentMethodsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethodList {
            payment_id: self
                .client_secret
                .as_ref()
                .and_then(|cs| cs.rsplit_once("_secret_"))
                .map(|(pid, _)| pid.to_string()),
        })
    }
}

impl ApiEventMetric for ListCountriesCurrenciesRequest {}

impl ApiEventMetric for ListCountriesCurrenciesResponse {}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentMethodListResponse {}

#[cfg(feature = "v1")]
impl ApiEventMetric for payment_methods::CustomerDefaultPaymentMethodResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.default_payment_method_id.clone().unwrap_or_default(),
            payment_method: Some(self.payment_method),
            payment_method_type: self.payment_method_type,
        })
    }
}

impl ApiEventMetric for PaymentMethodCollectLinkRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.pm_collect_link_id
            .as_ref()
            .map(|id| ApiEventsType::PaymentMethodCollectLink {
                link_id: id.clone(),
            })
    }
}

impl ApiEventMetric for PaymentMethodCollectLinkRenderRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethodCollectLink {
            link_id: self.pm_collect_link_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentMethodCollectLinkResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethodCollectLink {
            link_id: self.pm_collect_link_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentListFilterConstraints {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for PaymentListFilters {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}
impl ApiEventMetric for PaymentListFiltersV2 {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for PaymentListConstraints {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for PaymentListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for RecoveryPaymentsCreate {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for RecoveryPaymentsResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for RecoveryPaymentListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentListResponseV2 {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}
impl ApiEventMetric for PaymentsAggregateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for RedirectionResponse {}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsIncrementalAuthorizationRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsExternalAuthenticationResponse {}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsExternalAuthenticationRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for ExtendedCardInfoResponse {}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsManualUpdateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsManualUpdateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentsSessionResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentStartRedirectionRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for payments::PaymentMethodListResponseForPayments {
    // Payment id would be populated by the request
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}
#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentMethodListResponseForSession {}

#[cfg(feature = "v2")]
impl ApiEventMetric for payments::PaymentsCaptureResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}
