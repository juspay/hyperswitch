use events::{EventsError, Message, MessagingInterface};
use masking::masked_serialize;
use serde::Serialize;
use time::PrimitiveDateTime;

use super::EventType;
use crate::services::{kafka::KafkaMessage, logger};

#[derive(Clone, Debug, Default)]
pub struct EventLogger {}

impl EventLogger {
    #[track_caller]
    pub(super) fn log_event<T: KafkaMessage>(&self, event: &T) {
        logger::info!(event = ?serde_json::to_value(event).unwrap_or(serde_json::json!({"error": "serialization failed"})), event_type =? event.event_type(), event_id =? event.key(), log_type = "event");
    }
}

impl MessagingInterface for EventLogger {
    type MessageClass = EventType;

    fn send_message<T>(
        &self,
        data: T,
        _timestamp: PrimitiveDateTime,
    ) -> error_stack::Result<(), EventsError>
    where
        T: Message<Class = Self::MessageClass> + Serialize,
    {
        logger::info!(event = ?masked_serialize(&data).unwrap_or(serde_json::json!({"error": "serialization failed"})), event_type =? data.get_message_class(), event_id =? data.identifier(), log_type = "event");
        Ok(())
    }
}
