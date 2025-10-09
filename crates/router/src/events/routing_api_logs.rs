pub use hyperswitch_interfaces::events::routing_api_logs::RoutingEvent;

use super::EventType;
use crate::services::kafka::KafkaMessage;

impl KafkaMessage for RoutingEvent {
    fn event_type(&self) -> EventType {
        EventType::RoutingApiLogs
    }

    fn key(&self) -> String {
        format!(
            "{}-{}-{}",
            self.get_merchant_id(),
            self.get_profile_id(),
            self.get_payment_id()
        )
    }
}
