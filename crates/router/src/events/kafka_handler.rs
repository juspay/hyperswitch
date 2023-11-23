use router_env::tracing;

use super::{EventHandler, RawEvent};
use crate::services::kafka::{KafkaMessage, KafkaProducer};

impl EventHandler for KafkaProducer {
    fn log_event(&self, event: RawEvent) {
        let topic = self.get_topic(event.event_type);
        if let Err(er) = self.log_kafka_event(topic, &event) {
            tracing::error!("Failed to log event to kafka: {:?}", er);
        }
    }
}

impl KafkaMessage for RawEvent {
    fn key(&self) -> String {
        self.key.clone()
    }
}
