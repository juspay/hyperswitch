use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::{
    payment_methods::{
        CustomerPaymentMethodsListResponse, PaymentMethodDeleteResponse, PaymentMethodListRequest,
        PaymentMethodResponse, PaymentMethodUpdate,
    },
    payments::{
        PaymentIdType, PaymentListConstraints, PaymentListFilterConstraints, PaymentListFilters,
        PaymentListResponse, PaymentListResponseV2, PaymentsApproveRequest, PaymentsCancelRequest,
        PaymentsCaptureRequest, PaymentsIncrementalAuthorizationRequest, PaymentsRejectRequest,
        PaymentsRequest, PaymentsResponse, PaymentsRetrieveRequest, PaymentsStartRequest,
        RedirectionResponse,
    },
};
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

impl ApiEventMetric for PaymentsStartRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentsCaptureRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.to_owned(),
        })
    }
}

impl ApiEventMetric for PaymentsCancelRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentsApproveRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentsRejectRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentsRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        match self.payment_id {
            Some(PaymentIdType::PaymentIntentId(ref id)) => Some(ApiEventsType::Payment {
                payment_id: id.clone(),
            }),
            _ => None,
        }
    }
}

impl ApiEventMetric for PaymentsResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.payment_id
            .clone()
            .map(|payment_id| ApiEventsType::Payment { payment_id })
    }
}

impl ApiEventMetric for PaymentMethodResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::PaymentMethod {
            payment_method_id: self.payment_method_id.clone(),
            payment_method: Some(self.payment_method),
            payment_method_type: self.payment_method_type,
        })
    }
}

impl ApiEventMetric for PaymentMethodUpdate {}

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

impl ApiEventMetric for RedirectionResponse {}

impl ApiEventMetric for PaymentsIncrementalAuthorizationRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}
