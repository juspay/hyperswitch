use router_env::{tracing_actix_web::RequestId, types::FlowMetric};
use serde::Serialize;
use time::OffsetDateTime;

use super::{EventType, RawEvent};
use crate::services::authentication::AuthenticationType;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ApiEvent {
    api_flow: String,
    created_at_timestamp: i128,
    request_id: String,
    latency: u128,
    status_code: i64,
    #[serde(flatten)]
    auth_type: AuthenticationType,
    request: serde_json::Value,
    response: Option<serde_json::Value>,
}

impl ApiEvent {
    pub fn new(
        api_flow: &impl FlowMetric,
        request_id: &RequestId,
        latency: u128,
        status_code: i64,
        request: serde_json::Value,
        response: Option<serde_json::Value>,
        auth_type: AuthenticationType,
    ) -> Self {
        Self {
            api_flow: api_flow.to_string(),
            created_at_timestamp: OffsetDateTime::now_utc().unix_timestamp_nanos(),
            request_id: request_id.as_hyphenated().to_string(),
            latency,
            status_code,
            request,
            response,
            auth_type,
        }
    }
}

impl TryFrom<ApiEvent> for RawEvent {
    type Error = serde_json::Error;

    fn try_from(value: ApiEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            event_type: EventType::ApiLogs,
            key: value.request_id.clone(),
            payload: serde_json::to_value(value)?,
        })
    }
}
