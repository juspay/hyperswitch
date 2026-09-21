use events::{EventsError, Message, MessagingInterface};
use hyperswitch_masking::ErasedMaskSerialize;
use time::PrimitiveDateTime;

use super::EventType;
use crate::services::{kafka::KafkaMessage, logger};

#[derive(Clone, Debug, Default)]
pub struct EventLogger {}

impl EventLogger {
    #[track_caller]
    pub(super) fn log_event<T: KafkaMessage>(&self, event: &T) {
        logger::info!(event = ?event.masked_serialize().unwrap_or_else(|e| serde_json::json!({"error": e.to_string()})), event_type =? event.event_type(), event_id =? event.key(), log_type =? "event");
    }
}

impl MessagingInterface for EventLogger {
    type MessageClass = EventType;

    // The `events` crate's trait fixes the metadata type, and it does not use the facade.
    #[allow(clippy::disallowed_types)]
    fn send_message<T>(
        &self,
        data: T,
        metadata: std::collections::HashMap<String, String>,
        _timestamp: PrimitiveDateTime,
    ) -> error_stack::Result<(), EventsError>
    where
        T: Message<Class = Self::MessageClass> + ErasedMaskSerialize,
    {
        logger::info!(event =? data.masked_serialize().unwrap_or_else(|e| serde_json::json!({"error": e.to_string()})), event_type =? data.get_message_class(), event_id =? data.identifier(), log_type =? "event", metadata = ?metadata);
        Ok(())
    }
}
