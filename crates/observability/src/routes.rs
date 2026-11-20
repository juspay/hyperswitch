//! HTTP surface, laid out as the router lays its own out: [`app`] holds the route tree, and one
//! module per area holds the handlers.

pub mod alert_rules;
pub mod alerts_info;
pub mod app;
pub mod blacklist;
pub mod dictionary;
pub mod evaluator;
pub mod health_check;
pub mod lifecycle_events;
pub mod metadata;
#[cfg(feature = "v1")]
pub mod monitoring;
pub mod notify;
pub mod rule_toggles;
pub mod thresholds;

#[cfg(feature = "v1")]
pub use self::app::Monitoring;
pub use self::app::{Alerts, Health};
