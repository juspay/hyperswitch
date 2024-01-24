use common_utils::events::{ApiEventMetric, ApiEventsType};

use super::{DisputeResponse, DisputeResponsePaymentsRetrieve, SubmitEvidenceRequest};

impl ApiEventMetric for SubmitEvidenceRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Dispute {
            dispute_id: self.dispute_id.clone(),
        })
    }
}
impl ApiEventMetric for DisputeResponsePaymentsRetrieve {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Dispute {
            dispute_id: self.dispute_id.clone(),
        })
    }
}
impl ApiEventMetric for DisputeResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Dispute {
            dispute_id: self.dispute_id.clone(),
        })
    }
}
