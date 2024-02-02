use common_utils::request::Method;
use router_env::tracing_actix_web::RequestId;
use serde::Serialize;
use time::OffsetDateTime;

use super::{EventType, RawEvent};

#[derive(Debug, Serialize)]
pub struct ConnectorEvent {
    connector_name: String,
    flow: String,
    request: String,
    response: Option<String>,
    url: String,
    method: String,
    payment_id: String,
    merchant_id: String,
    created_at: i128,
    request_id: String,
    latency: u128,
    refund_id: Option<String>,
    dispute_id: Option<String>,
    status_code: u16,
}

impl ConnectorEvent {
    #[allow(clippy::too_many_arguments)]
        /// Creates a new instance of the struct with the provided parameters.
    pub fn new(
        connector_name: String,
        flow: &str,
        request: serde_json::Value,
        response: Option<String>,
        url: String,
        method: Method,
        payment_id: String,
        merchant_id: String,
        request_id: Option<&RequestId>,
        latency: u128,
        refund_id: Option<String>,
        dispute_id: Option<String>,
        status_code: u16,
    ) -> Self {
        Self {
            connector_name,
            flow: flow
                .rsplit_once("::")
                .map(|(_, s)| s)
                .unwrap_or(flow)
                .to_string(),
            request: request.to_string(),
            response,
            url,
            method: method.to_string(),
            payment_id,
            merchant_id,
            created_at: OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000,
            request_id: request_id
                .map(|i| i.as_hyphenated().to_string())
                .unwrap_or("NO_REQUEST_ID".to_string()),
            latency,
            refund_id,
            dispute_id,
            status_code,
        }
    }
}

impl TryFrom<ConnectorEvent> for RawEvent {
    type Error = serde_json::Error;

        /// Attempts to convert a ConnectorEvent into a Result<Self, Self::Error>.
    fn try_from(value: ConnectorEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            event_type: EventType::ConnectorApiLogs,
            key: value.request_id.clone(),
            payload: serde_json::to_value(value)?,
        })
    }
}
