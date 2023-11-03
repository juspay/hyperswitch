use actix_web::HttpRequest;
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
    user_agent: Option<String>,
    ip_addr: Option<String>,
    url_path: String,
    response: Option<serde_json::Value>,
}

impl ApiEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_flow: &impl FlowMetric,
        request_id: &RequestId,
        latency: u128,
        status_code: i64,
        request: serde_json::Value,
        response: Option<serde_json::Value>,
        auth_type: AuthenticationType,
        http_req: &HttpRequest,
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
            ip_addr: http_req
                .connection_info()
                .realip_remote_addr()
                .map(ToOwned::to_owned),
            user_agent: http_req
                .headers()
                .get("user-agent")
                .and_then(|user_agent_value| user_agent_value.to_str().ok().map(ToOwned::to_owned)),
            url_path: http_req.path().to_string(),
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
