//! HTTP surface, laid out as the router lays its own out: [`app`] holds the route tree, and one
//! module per area holds the handlers.
//!
//! [`app`] holds *every* route this service serves, including the alert manager's, whose handlers
//! live with the rest of that concern in [`crate::alert_manager::routes`].

pub mod app;
pub mod health_check;
pub mod notify;

pub use self::app::{Alerts, Health};
