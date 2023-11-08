use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::payouts::{
    PayoutActionRequest, PayoutCreateRequest, PayoutCreateResponse, PayoutRetrieveRequest,
};

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
