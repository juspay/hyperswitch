//! HTTP surface, laid out as the router lays its own out:

pub mod app;
pub mod health_check;
pub mod notify;

pub use self::app::{Alerts, Health};
