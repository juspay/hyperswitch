use common_utils::events::{ApiEventMetric, ApiEventsType};

#[cfg(feature = "v2")]
use super::{
    PaymentsConfirmIntentResponse, PaymentsCreateIntentRequest, PaymentsCreateIntentResponse,
    PaymentsRetrieveResponse,
};
#[cfg(all(
    any(feature = "v2", feature = "v1"),
    not(feature = "payment_methods_v2")
))]
use crate::payment_methods::CustomerPaymentMethodsListResponse;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::payment_methods::CustomerPaymentMethodsListResponse;
use crate::{
    payment_methods::{
        CustomerDefaultPaymentMethodResponse, DefaultPaymentMethod, ListCountriesCurrenciesRequest,
        ListCountriesCurrenciesResponse, PaymentMethodCollectLinkRenderRequest,
        PaymentMethodCollectLinkRequest, PaymentMethodCollectLinkResponse,
        PaymentMethodDeleteResponse, PaymentMethodListRequest, PaymentMethodListResponse,
        PaymentMethodResponse, PaymentMethodUpdate,
    },
    payments::{
        self, ExtendedCardInfoResponse, PaymentIdType, PaymentListConstraints,
        PaymentListFilterConstraints, PaymentListFilters, PaymentListFiltersV2,
        PaymentListResponse, PaymentListResponseV2, PaymentsAggregateResponse,
        PaymentsApproveRequest, PaymentsCancelRequest, PaymentsCaptureRequest,
        PaymentsCompleteAuthorizeRequest, PaymentsDynamicTaxCalculationRequest,
        PaymentsDynamicTaxCalculationResponse, PaymentsExternalAuthenticationRequest,
        PaymentsExternalAuthenticationResponse, PaymentsIncrementalAuthorizationRequest,
        PaymentsManualUpdateRequest, PaymentsManualUpdateResponse,
        PaymentsPostSessionTokensRequest, PaymentsPostSessionTokensResponse, PaymentsRejectRequest,
        PaymentsResponse, PaymentsRetrieveRequest, PaymentsSessionResponse, PaymentsStartRequest,
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
impl ApiEventMetric for PaymentsCreateIntentResponse {
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
impl ApiEventMetric for PaymentsRetrieveResponse {
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
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: self.payment_method,
            payment_method_type: self.payment_method_type,
        })
    }
}

impl ApiEventMetric for PaymentMethodUpdate {}

impl ApiEventMetric for DefaultPaymentMethod {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: None,
            payment_method_type: None,
        })
    }
}

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

impl ApiEventMetric for CustomerDefaultPaymentMethodResponse {
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

impl ApiEventMetric for PaymentListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

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

#[cfg(feature = "v1")]
impl ApiEventMetric for PaymentsSessionResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}
