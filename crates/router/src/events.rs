use data_models::errors::{StorageError, StorageResult};
use error_stack::ResultExt;
use events::{EventSink, EventsError};
use router_env::logger;
use serde::{Deserialize, Serialize};
use storage_impl::errors::ApplicationError;
use time::PrimitiveDateTime;

use crate::{
    db::KafkaProducer,
    services::kafka::{KafkaMessage, KafkaSettings},
};
use crate::{
    db::KafkaProducer,
    services::kafka::{KafkaMessage, KafkaSettings},
};

pub mod api_logs;
pub mod audit_events;
pub mod connector_api_logs;
pub mod event_logger;
pub mod outgoing_webhook_logs;

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    PaymentIntent,
    PaymentAttempt,
    Refund,
    ApiLogs,
    ConnectorApiLogs,
    OutgoingWebhookLogs,
    Dispute,
    AuditEvent,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(tag = "source")]
#[serde(rename_all = "lowercase")]
pub enum EventsConfig {
    Kafka {
        kafka: Box<KafkaSettings>,
    },
    #[default]
    Logs,
}

#[derive(Debug, Clone)]
pub enum EventsHandler {
    Kafka(KafkaProducer),
    Logs(event_logger::EventLogger),
}

impl Default for EventsHandler {
    fn default() -> Self {
        Self::Logs(event_logger::EventLogger {})
    }
}

impl EventsConfig {
    pub async fn get_event_handler(&self) -> StorageResult<EventsHandler> {
        Ok(match self {
            Self::Kafka { kafka } => EventsHandler::Kafka(
                KafkaProducer::create(kafka)
                    .await
                    .change_context(StorageError::InitializationError)?,
            ),
            Self::Logs => EventsHandler::Logs(event_logger::EventLogger::default()),
        })
    }

    pub fn validate(&self) -> Result<(), ApplicationError> {
        match self {
            Self::Kafka { kafka } => kafka.validate(),
            Self::Logs => Ok(()),
        }
    }
}

impl EventsHandler {
    pub fn log_event<T: KafkaMessage>(&self, event: &T) {
    pub fn log_event<T: KafkaMessage>(&self, event: &T) {
        match self {
            Self::Kafka(kafka) => kafka.log_event(event).map_or((), |e| {
                logger::error!("Failed to log event: {:?}", e);
            }),
            Self::Kafka(kafka) => kafka.log_event(event).map_or((), |e| {
                logger::error!("Failed to log event: {:?}", e);
            }),
            Self::Logs(logger) => logger.log_event(event),
        };
    }
}

impl EventSink<EventType> for EventsHandler {
    fn publish_event(
        &self,
        data: serde_json::Value,
        identifier: String,
        topic: EventType,
        timestamp: PrimitiveDateTime,
    ) -> error_stack::Result<(), EventsError> {
        match self {
            Self::Kafka(kafka) => kafka.publish_event(data, identifier, topic, timestamp),
            Self::Logs(_) => {
                logger::info!(event = ?data, event_type=?topic, event_id =?identifier, log_type = "event");
                Ok(())
            }
        }
    }
}
