mod core;
pub mod events;
pub trait ConnectorEventAnalytics: events::ConnectorEventLogAnalytics {}

pub use self::core::connector_events_core;
