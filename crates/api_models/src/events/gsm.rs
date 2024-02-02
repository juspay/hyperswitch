use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::gsm;

impl ApiEventMetric for gsm::GsmCreateRequest {
        /// Retrieves the API event type, if available.
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Gsm)
    }
}

impl ApiEventMetric for gsm::GsmUpdateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Gsm)
    }
}

impl ApiEventMetric for gsm::GsmRetrieveRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Gsm)
    }
}

impl ApiEventMetric for gsm::GsmDeleteRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Gsm)
    }
}

impl ApiEventMetric for gsm::GsmDeleteResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Gsm)
    }
}

impl ApiEventMetric for gsm::GsmResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Gsm)
    }
}
