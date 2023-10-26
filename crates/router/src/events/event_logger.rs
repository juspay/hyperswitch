use super::{EventHandler, RawEvent};
use crate::services::logger;

#[derive(Clone, Debug, Default)]
pub struct EventLogger {}

impl EventHandler for EventLogger {
    fn log_event(&self, event: RawEvent) {
        logger::info!(event = ?serde_json::to_string(&event.payload).unwrap_or(r#"{ "error": "Serialization failed" }"#.to_string()), event_type =? event.event_type, event_id =? event.key, log_type = "event");
    }
}
