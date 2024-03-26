use crate::services::{kafka::KafkaMessage, logger};

#[derive(Clone, Debug, Default)]
pub struct EventLogger {}

impl EventLogger {
    #[track_caller]
    pub(super) fn log_event<T: KafkaMessage>(&self, event: &T) {
        logger::info!(event = ?serde_json::to_value(event).unwrap_or(serde_json::json!({"error": "serialization failed"})), event_type =? event.event_type(), event_id =? event.key(), log_type = "event");
    }
}
