use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::{
    payment_methods::{
        CustomerPaymentMethodsListResponse, PaymentMethodDeleteResponse, PaymentMethodListRequest,
        PaymentMethodListResponse, PaymentMethodResponse, PaymentMethodUpdate,
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
        /// This method returns the corresponding API event type based on the resource ID. If the resource ID is of type PaymentIntentId, it returns Some(ApiEventsType::Payment) with the payment ID. Otherwise, it returns None.
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
        /// This method returns the API event type associated with the payment, wrapped in an Option.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payment {
            payment_id: self.payment_id.clone(),
        })
    }
}

impl ApiEventMetric for PaymentsCaptureRequest {
        /// Returns the API event type, if it is a Payment event, containing the payment ID.
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
        /// Returns the type of API event associated with the payment. If the payment ID is of type PaymentIntentId,
    /// it returns Some(ApiEventsType::Payment) with the corresponding payment ID. Otherwise, it returns None.
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
        /// Retrieves the API event type associated with the payment ID, if available.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.payment_id
            .clone()
            .map(|payment_id| ApiEventsType::Payment { payment_id })
    }
}

impl ApiEventMetric for PaymentMethodResponse {
        /// Returns the API event type as an Option. If the API event type is PaymentMethod, it will return Some containing the payment method details, otherwise it will return None.
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
        /// This method returns an Option containing the API event type for a payment method, 
    /// including the payment method ID and optional details about the payment method and its type.
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
        /// Returns the API event type for the given payment method, if available.
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

impl ApiEventMetric for PaymentMethodListResponse {}

impl ApiEventMetric for PaymentListFilterConstraints {
        /// Returns the type of API event associated with the current instance.
    ///
    /// # Returns
    ///
    /// - `Some(ApiEventsType::ResourceListAPI)` if the API event type is ResourceListAPI
    /// - `None` if there is no API event type associated with the current instance
    ///
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
