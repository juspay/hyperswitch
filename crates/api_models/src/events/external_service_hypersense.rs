use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::external_service_hypersense::{
    ExternalFeeEstimatePayload, ExternalFeeEstimateRequest, ExternalFeeEstimateResponse,
};

impl ApiEventMetric for ExternalFeeEstimatePayload {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceHypersense)
    }
}

impl ApiEventMetric for ExternalFeeEstimateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceHypersense)
    }
}

impl ApiEventMetric for ExternalFeeEstimateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ExternalServiceHypersense)
    }
}
