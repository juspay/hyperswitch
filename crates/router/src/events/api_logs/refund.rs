use api_models::refunds::{
    RefundListMetaData, RefundListRequest, RefundListResponse, RefundRequest, RefundResponse,
    RefundUpdateRequest, RefundsRetrieveRequest,
};

use super::{ApiEventMetric, ApiEventsType};
use crate::types::storage::Refund;

impl ApiEventMetric for RefundRequest {
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
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Refund {
            payment_id: Some(self.payment_id.clone()),
            refund_id: self.refund_id.clone(),
        })
    }
}

impl ApiEventMetric for Refund {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Refund {
            payment_id: Some(self.payment_id.clone()),
            refund_id: self.refund_id.clone(),
        })
    }
}

impl ApiEventMetric for RefundsRetrieveRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Refund {
            payment_id: None,
            refund_id: self.refund_id.clone(),
        })
    }
}

impl ApiEventMetric for (&String, RefundUpdateRequest) {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Refund {
            payment_id: None,
            refund_id: self.0.clone(),
        })
    }
}

impl ApiEventMetric for RefundListRequest {
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
