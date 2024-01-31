use data_models::errors::{StorageError, StorageResult};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use storage_impl::errors::ApplicationError;

use crate::{db::KafkaProducer, services::kafka::KafkaSettings};

pub mod api_logs;
pub mod connector_api_logs;
pub mod event_logger;
pub mod kafka_handler;
pub mod outgoing_webhook_logs;

pub(super) trait EventHandler: Sync + Send + dyn_clone::DynClone {
    fn log_event(&self, event: RawEvent);
}

dyn_clone::clone_trait_object!(EventHandler);

#[derive(Debug, Serialize)]
pub struct RawEvent {
    pub event_type: EventType,
    pub key: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    PaymentIntent,
    PaymentAttempt,
    Refund,
    ApiLogs,
    ConnectorApiLogs,
    OutgoingWebhookLogs,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(tag = "source")]
#[serde(rename_all = "lowercase")]
pub enum EventsConfig {
    Kafka {
        kafka: KafkaSettings,
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
    pub fn log_event(&self, event: RawEvent) {
        match self {
            Self::Kafka(kafka) => kafka.log_event(event),
            Self::Logs(logger) => logger.log_event(event),
        }
    }
}
