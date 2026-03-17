use common_utils::types::keymanager::ExternalServiceCall;
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
        self.event
            .request_id
            .clone()
            .unwrap_or_else(|| "no_request_id".to_string())
    }

    fn event_type(&self) -> EventType {
        EventType::ExternalServiceCall
    }
}
