use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::payouts::{
    PayoutActionRequest, PayoutCreateRequest, PayoutCreateResponse, PayoutLinkInitiateRequest,
    PayoutListConstraints, PayoutListFilterConstraints, PayoutListFilters, PayoutListResponse,
    PayoutRetrieveRequest,
};

impl ApiEventMetric for PayoutRetrieveRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout {
            payout_id: self.payout_id.clone(),
        })
    }
}

impl ApiEventMetric for PayoutCreateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.payout_id.as_ref().map(|id| ApiEventsType::Payout {
            payout_id: id.clone(),
        })
    }
}

impl ApiEventMetric for PayoutCreateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout {
            payout_id: self.payout_id.clone(),
        })
    }
}

impl ApiEventMetric for PayoutActionRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout {
            payout_id: self.payout_id.clone(),
        })
    }
}

impl ApiEventMetric for PayoutListConstraints {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for PayoutListFilterConstraints {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for PayoutListResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for PayoutListFilters {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for PayoutLinkInitiateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout {
            payout_id: self.payout_id.clone(),
        })
    }
}
