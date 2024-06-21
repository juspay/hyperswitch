//! Hyperswitch interface
#![warn(missing_docs, missing_debug_implementations)]

pub mod api;
pub mod configs;
/// definition of the new connector integration trait
pub mod connector_integration_v2;
pub mod consts;
pub mod encryption_interface;
pub mod errors;
pub mod events;
pub mod metrics;
pub mod secrets_interface;
pub mod types;
