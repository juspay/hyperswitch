use common_utils::events::{ApiEventMetric, ApiEventsType};

#[cfg(feature = "v2")]
use super::{
    PaymentStartRedirectionRequest, PaymentsConfirmIntentResponse, PaymentsCreateIntentRequest,
    PaymentsGetIntentRequest, PaymentsIntentResponse, PaymentsRequest,
};
#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
use crate::payment_methods::CustomerPaymentMethodsListResponse;
#[cfg(feature = "v1")]
use crate::payments::{PaymentListResponse, PaymentListResponseV2};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::{events, payment_methods::CustomerPaymentMethodsListResponse};
use crate::{
    payment_methods::{
        self, ListCountriesCurrenciesRequest, ListCountriesCurrenciesResponse,
        PaymentMethodCollectLinkRenderRequest, PaymentMethodCollectLinkRequest,
        PaymentMethodCollectLinkResponse, PaymentMethodDeleteResponse, PaymentMethodListRequest,
        PaymentMethodListResponse, PaymentMethodMigrateResponse, PaymentMethodResponse,
        PaymentMethodUpdate,
    },
    payments::{
        self, ExtendedCardInfoResponse, PaymentIdType, PaymentListConstraints,
        PaymentListFilterConstraints, PaymentListFilters, PaymentListFiltersV2,
        PaymentsAggregateResponse, PaymentsApproveRequest, PaymentsCancelRequest,
        PaymentsCaptureRequest, PaymentsCompleteAuthorizeRequest,
        PaymentsDynamicTaxCalculationRequest, PaymentsDynamicTaxCalculationResponse,
        PaymentsExternalAuthenticationRequest, PaymentsExternalAuthenticationResponse,
        PaymentsIncrementalAuthorizationRequest, PaymentsManualUpdateRequest,
        PaymentsManualUpdateResponse, PaymentsPostSessionTokensRequest,
        PaymentsPostSessionTokensResponse, PaymentsRejectRequest, PaymentsResponse,
        PaymentsRetrieveRequest, PaymentsSessionResponse, PaymentsStartRequest,
        RedirectionResponse,
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

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentsCreateIntentRequest {
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
impl ApiEventMetric for payments::PaymentsResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
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
impl ApiEventMetric for PaymentsIntentResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for PaymentsConfirmIntentResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(feature = "v2")]
impl ApiEventMetric for super::PaymentsRetrieveResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentMethodResponse {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
        })
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.id.clone(),
            payment_method_type: self.payment_method_type,
            payment_method_subtype: self.payment_method_subtype,
        })
    }
}

impl ApiEventMetric for PaymentMethodMigrateResponse {
    #[cfg(all(
        any(feature = "v1", feature = "v2"),
        not(feature = "payment_methods_v2")
    ))]
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_response.payment_method_id.clone(),
            payment_method: self.payment_method_response.payment_method,
            payment_method_type: self.payment_method_response.payment_method_type,
        })
    }

    #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
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
impl ApiEventMetric for PaymentMethodDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.id.clone(),
            payment_method_type: None,
            payment_method_subtype: None,
        })
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentMethodDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: None,
            payment_method_type: None,
        })
    }
}

impl ApiEventMetric for CustomerPaymentMethodsListResponse {}

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

impl ApiEventMetric for ListCountriesCurrenciesRequest {}

impl ApiEventMetric for ListCountriesCurrenciesResponse {}
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

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentListResponse {
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
impl ApiEventMetric for payments::PaymentsCaptureResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.id.clone(),
        })
    }
}
