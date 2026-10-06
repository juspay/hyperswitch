mod core;
pub mod events;
pub trait ConnectorEventAnalytics: events::ConnectorEventLogAnalytics {}

pub use self::core::connector_events_core;

/// Which set of physical connector-event tables a query targets.
#[derive(Debug, Clone, Copy)]
pub enum ConnectorEventSource {
    /// Native Hyperswitch connector events
    /// (`connector_events_audit` / `connector_events_payout_audit`).
    Hyperswitch,
    /// UCS/Prism-emitted connector events
    /// (`prism_connector_events_audit` / `prism_connector_events_payout_audit`).
    Prism,
}
