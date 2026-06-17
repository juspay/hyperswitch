pub use hyperswitch_interfaces::events::connector_api_logs::ConnectorEvent;

use super::EventType;
use crate::services::kafka::KafkaMessage;

impl KafkaMessage for ConnectorEvent {
    fn event_type(&self) -> EventType {
        EventType::ConnectorApiLogs
    }

    fn key(&self) -> String {
        self.request_id.clone()
    }
}
