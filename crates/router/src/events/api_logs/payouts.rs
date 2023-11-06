use api_models::payouts::{
    PayoutActionRequest, PayoutCreateRequest, PayoutCreateResponse, PayoutRetrieveRequest,
};

use super::{ApiEventMetric, ApiEventsType};

impl ApiEventMetric for PayoutRetrieveRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout)
    }
}

impl ApiEventMetric for PayoutCreateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout)
    }
}

impl ApiEventMetric for PayoutCreateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout)
    }
}

impl ApiEventMetric for PayoutActionRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout)
    }
}

// impl ApiEventMetric for
