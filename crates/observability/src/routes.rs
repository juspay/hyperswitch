//! HTTP surface, laid out as the router lays its own out: [`app`] holds the route tree, and one
//! module per area holds the handlers.

#[path = "alert_manager.rs"]
pub mod alert_manager;
pub mod app;
pub mod cloudwatch;
pub mod health_check;
pub mod notify;

pub use self::app::{Alerts, Health};
