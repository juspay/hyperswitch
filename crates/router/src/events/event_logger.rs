use super::{EventHandler, RawEvent};
use crate::services::logger;

#[derive(Clone, Debug, Default)]
pub struct EventLogger {}

impl EventHandler for EventLogger {
    fn log_event(&self, event: RawEvent, previous: Option<RawEvent>) {
        if let Some(prev) = previous {
            logger::info!(previous = ?serde_json::to_string(&prev.payload).unwrap_or(r#"{ "error": "Serialization failed" }"#.to_string()), current = ?serde_json::to_string(&event.payload).unwrap_or(r#"{ "error": "Serialization failed" }"#.to_string()), event_type =? event.event_type, event_id =? event.key, log_type = "event");
        } else {
            logger::info!(current = ?serde_json::to_string(&event.payload).unwrap_or(r#"{ "error": "Serialization failed" }"#.to_string()), event_type =? event.event_type, event_id =? event.key, log_type = "event");
        }
    }
}
