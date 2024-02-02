use super::{EventHandler, RawEvent};
use crate::services::logger;

#[derive(Clone, Debug, Default)]
pub struct EventLogger {}

impl EventHandler for EventLogger {
    #[track_caller]
        /// Log an event with the provided RawEvent data.
    fn log_event(&self, event: RawEvent) {
        logger::info!(event = ?event.payload.to_string(), event_type =? event.event_type, event_id =? event.key, log_type = "event");
    }
}
