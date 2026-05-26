use common_utils::external_service::ExternalServiceCall;
use serde::Serialize;

use super::EventType;
use crate::services::kafka::KafkaMessage;

#[derive(Debug, Serialize)]
pub struct KafkaExternalServiceCall<'a> {
    #[serde(flatten)]
    pub event: &'a ExternalServiceCall,
}

impl KafkaMessage for KafkaExternalServiceCall<'_> {
    fn key(&self) -> String {
        self.event.request_id.clone()
    }

    fn event_type(&self) -> EventType {
        EventType::ExternalServiceCall
    }
}
