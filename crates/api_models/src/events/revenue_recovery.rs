use common_utils::events::{ApiEventMetric, ApiEventsType};

use crate::process_tracker::revenue_recovery::{
    RevenueRecoveryId, RevenueRecoveryResponse, RevenueRecoveryRetriggerRequest,
};

impl ApiEventMetric for RevenueRecoveryResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ProcessTracker)
    }
}
impl ApiEventMetric for RevenueRecoveryId {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ProcessTracker)
    }
}
impl ApiEventMetric for RevenueRecoveryRetriggerRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::ProcessTracker)
    }
}
