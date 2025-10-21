use crate::events::connector_api_logs::ConnectorEvent;

pub mod connector_api_logs;
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
}
