use common_utils::request::Method;
use router_env::tracing_actix_web::RequestId;
use serde::Serialize;
use serde_json::json;
use time::OffsetDateTime;

use super::{EventType, RawEvent};

#[derive(Debug, Serialize)]
pub struct ConnectorEvent {
    connector_name: String,
    flow: String,
    request: String,
    masked_response: Option<String>,
    error: Option<String>,
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
    pub fn new(
        connector_name: String,
        flow: &str,
        request: serde_json::Value,
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
            masked_response: None,
            error: None,
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

    pub fn set_response_body<T: Serialize>(&mut self, response: &T) {
        match masking::masked_serialize(response) {
            Ok(masked) => {
                self.masked_response = Some(masked.to_string());
            }
            Err(er) => self.set_error(json!({"error": er.to_string()})),
        }
    }

    pub fn set_error(&mut self, error: serde_json::Value) {
        self.error = Some(error.to_string());
    }
}

impl TryFrom<ConnectorEvent> for RawEvent {
    type Error = serde_json::Error;

    fn try_from(value: ConnectorEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            event_type: EventType::ConnectorApiLogs,
            key: value.request_id.clone(),
            payload: serde_json::to_value(value)?,
        })
    }
}
