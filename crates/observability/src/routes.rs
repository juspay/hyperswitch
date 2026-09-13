//! HTTP surface, laid out as the router lays its own out: [`app`] holds the route tree, and one
//! module per area holds the handlers.

pub mod app;
pub mod health_check;
pub mod notify;

pub use self::app::{Alerts, Health};
