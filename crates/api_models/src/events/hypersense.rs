use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::hypersense::{
    HypersenseLogoutTokenRequest, HypersenseTokenResponse, HypersenseVerifyTokenRequest, HypersenseVerifyTokenResponse
};

impl ApiEventMetric for HypersenseTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Hypersense)
    }
}

impl ApiEventMetric for HypersenseVerifyTokenRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Hypersense)
    }
}

impl ApiEventMetric for HypersenseVerifyTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            user_id: self.user_id.clone(),
        })
    }
}

impl ApiEventMetric for HypersenseLogoutTokenRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Hypersense)
    }
}