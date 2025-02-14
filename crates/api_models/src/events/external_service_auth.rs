use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::external_service_auth::{
    ExternalSignoutTokenRequest, ExternalTokenResponse, ExternalVerifyTokenRequest,
    ExternalVerifyTokenResponse,
};

impl ApiEventMetric for ExternalTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceAuth)
    }
}

impl ApiEventMetric for ExternalVerifyTokenRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceAuth)
    }
}

impl ApiEventMetric for ExternalVerifyTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            user_id: self.get_user_id().to_string(),
        })
    }
}

impl ApiEventMetric for ExternalSignoutTokenRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceAuth)
    }
}
