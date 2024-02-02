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
        /// Creates and returns a default instance of the current struct, initializing the logs with an empty EventLogger.
    fn default() -> Self {
        Self::Logs(event_logger::EventLogger {})
    }
}

impl EventsConfig {
        /// Asynchronously retrieves the event handler based on the storage type.
    /// 
    /// # Returns
    /// 
    /// Returns a `StorageResult` with the `EventsHandler` corresponding to the storage type.
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

        /// Validate the configuration for the application.
    ///
    /// This method will validate the configuration for the application, based on the type of configuration.
    /// If the configuration is of type Kafka, it will call the validate method on the Kafka configuration.
    /// If the configuration is of type Logs, it will return Ok(()).
    ///
    /// # Returns
    ///
    /// * `Result<(), ApplicationError>` - Ok(()) if the validation is successful, ApplicationError if there is an error.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        match self {
            Self::Kafka { kafka } => kafka.validate(),
            Self::Logs => Ok(()),
        }
    }
}

impl EventsHandler {
        /// Logs the given event using the appropriate logging mechanism based on the type of `self`.
    pub fn log_event(&self, event: RawEvent) {
        match self {
            Self::Kafka(kafka) => kafka.log_event(event),
            Self::Logs(logger) => logger.log_event(event),
        }
    }
}
