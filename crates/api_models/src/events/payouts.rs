use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::payouts::{
    PayoutActionRequest, PayoutCreateRequest, PayoutCreateResponse, PayoutLinkInitiateRequest,
    PayoutListConstraints, PayoutListFilterConstraints, PayoutListFilters, PayoutListFiltersV2,
    PayoutListResponse, PayoutRetrieveRequest,
};

impl ApiEventMetric for PayoutRetrieveRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout {
            payout_id: self.payout_id.to_owned(),
        })
    }
}

impl ApiEventMetric for PayoutCreateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.payout_id.as_ref().map(|id| ApiEventsType::Payout {
            payout_id: id.to_owned(),
        })
    }
}

impl ApiEventMetric for PayoutCreateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout {
            payout_id: self.payout_id.to_owned(),
        })
    }
}

impl ApiEventMetric for PayoutActionRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout {
            payout_id: self.payout_id.to_owned(),
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

impl ApiEventMetric for PayoutListFiltersV2 {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ResourceListAPI)
    }
}

impl ApiEventMetric for PayoutLinkInitiateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Payout {
            payout_id: self.payout_id.to_owned(),
        })
    }
}
