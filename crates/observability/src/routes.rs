//! HTTP surface, laid out as the router lays its own out: [`app`] holds the route tree, and one
//! module per area holds the handlers.

pub mod app;
pub mod cloudwatch;
pub mod health_check;
#[cfg(feature = "v1")]
pub mod monitoring;
pub mod notify;

#[cfg(feature = "v1")]
pub use self::app::Monitoring;
pub use self::app::{Alerts, Health};
