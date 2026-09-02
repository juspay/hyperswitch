pub use hyperswitch_interfaces::events::microservice_api_logs::MicroserviceEvent;

use super::EventType;
use crate::services::kafka::KafkaMessage;

impl KafkaMessage for MicroserviceEvent {
    fn event_type(&self) -> EventType {
        EventType::MicroserviceApiLogs
    }

    fn key(&self) -> String {
        format!(
            "{}-{}",
            self.get_merchant_id().unwrap_or("NO_MERCHANT_ID"),
            self.get_request_id(),
        )
    }
}
