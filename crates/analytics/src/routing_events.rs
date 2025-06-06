mod core;
pub mod events;
pub trait RoutingEventAnalytics: events::RoutingEventLogAnalytics {}

pub use self::core::routing_events_core;
