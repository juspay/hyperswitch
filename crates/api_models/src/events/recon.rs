use common_utils::events::{ApiEventMetric, ApiEventsType};
use masking::PeekInterface;

use crate::recon::{
    ReconStatusResponse, ReconTokenResponse, ReconUpdateMerchantRequest, VerifyTokenResponse,
};

impl ApiEventMetric for ReconUpdateMerchantRequest {
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

impl ApiEventMetric for VerifyTokenResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::User {
            user_id: self.user_email.peek().to_string(),
        })
    }
}
