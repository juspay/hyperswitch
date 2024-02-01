use error_stack::{IntoReport, ResultExt};
use router_env::tracing;

use super::{EventHandler, RawEvent};
use crate::{
    db::MQResult,
    services::kafka::{KafkaError, KafkaMessage, KafkaProducer},
};
impl EventHandler for KafkaProducer {
        /// Logs the given event to Kafka after retrieving the appropriate topic based on the event type.
    fn log_event(&self, event: RawEvent) {
        let topic = self.get_topic(event.event_type);
        if let Err(er) = self.log_kafka_event(topic, &event) {
            tracing::error!("Failed to log event to kafka: {:?}", er);
        }
    }
}

impl KafkaMessage for RawEvent {
        /// Returns the key of the current instance as a String.
    fn key(&self) -> String {
        self.key.clone()
    }

        /// Retrieves the value of the message payload as a vector of bytes. 
    /// 
    fn value(&self) -> MQResult<Vec<u8>> {
        // Add better error logging here
        serde_json::to_vec(&self.payload)
            .into_report()
            .change_context(KafkaError::GenericError)
    }
}
