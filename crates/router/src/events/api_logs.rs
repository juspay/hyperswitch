use router_env::{tracing_actix_web::RequestId, types::FlowMetric};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::Event;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApiEvent {
    api_flow: String,
    created_at_timestamp: i128,
    request_id: String,
    latency: u128,
    status_code: i64,
}

impl ApiEvent {
    pub fn new(
        api_flow: &impl FlowMetric,
        request_id: &RequestId,
        latency: u128,
        status_code: i64,
    ) -> Self {
        Self {
            api_flow: api_flow.to_string(),
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos(),
            request_id: request_id.as_hyphenated().to_string(),
            latency,
            status_code,
        }
    }
}

impl Event for ApiEvent {
    fn event_type() -> super::EventType {
        super::EventType::ApiLogs
    }

    fn key(&self) -> String {
        "HEALTH".to_string()
    }
}
