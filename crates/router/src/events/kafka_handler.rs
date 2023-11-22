use super::events::{EventHandler, RawEvent};
use router_env::tracing;

use crate::services::kafka::KafkaProducer;

impl EventHandler for KafkaProducer {
    fn log_event(&self, event: RawEvent) {
        let topic = self.get_topic(event.event_type);
        if let Err(er) = self.log_kafka_event(topic, &event) {
            tracing::error!("Failed to log event to kafka: {:?}", er);
        }
    }
}
