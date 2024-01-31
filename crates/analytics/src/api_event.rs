mod core;
pub mod events;
pub mod filters;
pub mod metrics;
pub mod types;

pub trait APIEventAnalytics: events::ApiLogsFilterAnalytics {}

pub use self::core::{api_events_core, get_api_event_metrics, get_filters};
