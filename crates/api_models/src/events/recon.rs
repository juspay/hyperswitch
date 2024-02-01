use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::recon::{ReconStatusResponse, ReconTokenResponse, ReconUpdateMerchantRequest};

impl ApiEventMetric for ReconUpdateMerchantRequest {
        /// This method returns the API event type as an Option. 
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Recon)
    }
}

impl ApiEventMetric for ReconTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Recon)
    }
}

impl ApiEventMetric for ReconStatusResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Recon)
    }
}
