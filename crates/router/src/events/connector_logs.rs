use serde::Serialize;

use super::{EventType, RawEvent};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ConnectorEvent {
    #[serde(rename = "created_at")]
    created_at_timestamp: i128,
    request_id: String,
    latency: u128,
    status_code: i64,
    request: serde_json::Value,
    url_path: String,
    response: Option<serde_json::Value>,
}

impl TryFrom<ConnectorEvent> for RawEvent {
    type Error = serde_json::Error;

    fn try_from(value: ConnectorEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            event_type: EventType::ConnectorLogs,
            key: value.request_id.clone(),
            payload: serde_json::to_value(value)?,
        })
    }
}
