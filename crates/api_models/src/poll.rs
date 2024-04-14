use common_utils::events::{ApiEventMetric, ApiEventsType};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, ToSchema, Clone, Serialize)]
pub struct PollResponse {
    /// The poll id
    pub poll_id: String,
    /// Status of the poll
    pub status: PollStatus,
}

#[derive(Debug, strum::Display, strum::EnumString, Clone, serde::Serialize, ToSchema)]
#[strum(serialize_all = "snake_case")]
pub enum PollStatus {
    Pending,
    Completed,
    NotFound,
}

impl ApiEventMetric for PollResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Poll {
            poll_id: self.poll_id.clone(),
        })
    }
}
