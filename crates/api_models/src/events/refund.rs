use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::refunds::{
    RefundListMetaData, RefundListRequest, RefundListResponse, RefundRequest, RefundResponse,
    RefundUpdateRequest, RefundsRetrieveRequest,
};

impl ApiEventMetric for RefundRequest {
        /// Retrieves the API event type based on the payment and refund IDs, if a refund ID is present.
    /// If a refund ID is present, it returns the ApiEventsType::Refund variant with the payment ID and refund ID,
    /// otherwise it returns None.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        let payment_id = self.payment_id.clone();
        self.refund_id
            .clone()
            .map(|refund_id| ApiEventsType::Refund {
                payment_id: Some(payment_id),
                refund_id,
            })
    }
}

impl ApiEventMetric for RefundResponse {
        /// Retrieves the API event type associated with the current instance.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Refund {
            payment_id: Some(self.payment_id.clone()),
            refund_id: self.refund_id.clone(),
        })
    }
}

impl ApiEventMetric for RefundsRetrieveRequest {
        /// This method returns the API event type associated with the instance. It returns an Option type which may contain the API event type. If the API event type is of type Refund, it includes the refund details such as payment_id and refund_id.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Refund {
            payment_id: None,
            refund_id: self.refund_id.clone(),
        })
    }
}

impl ApiEventMetric for RefundUpdateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Refund {
            payment_id: None,
            refund_id: self.refund_id.clone(),
        })
    }
}

impl ApiEventMetric for RefundListRequest {
        /// This method returns the API event type as an optional value. If the API event type is available, it returns Some(ApiEventsType::ResourceListAPI), otherwise it returns None.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for RefundListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for RefundListMetaData {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}
