//! HTTP surface, laid out as the router lays its own out: [`app`] holds the route tree, and one
//! module per area holds the handlers.

pub mod alerts_info;
pub mod app;
pub mod blacklist;
pub mod cloudwatch;
pub mod health_check;
pub mod notify;
pub mod rule_toggles;
pub mod thresholds;

pub use self::app::{Alerts, Health};
