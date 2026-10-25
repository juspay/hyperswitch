use crate::events::{connector_api_logs::ConnectorEvent, microservice_api_logs::MicroserviceEvent};

pub mod connector_api_logs;
pub mod microservice_api_logs;
pub mod routing_api_logs;

/// Event handling interface
#[async_trait::async_trait]
pub trait EventHandlerInterface: dyn_clone::DynClone
where
    Self: Send + Sync,
{
    /// Logs connector events
    #[track_caller]
    fn log_connector_event(&self, event: &ConnectorEvent);

    /// Logs internal microservice call events
    #[track_caller]
    fn log_microservice_event(&self, event: &MicroserviceEvent);
}
