//! Hyperswitch interface
#![warn(missing_docs, missing_debug_implementations)]

pub mod api;
pub mod authentication;
/// Configuration related functionalities
pub mod configs;
/// Connector integration interface module
pub mod connector_integration_interface;
/// definition of the new connector integration trait
pub mod connector_integration_v2;
/// Constants used throughout the application
pub mod consts;
/// Conversion implementations
pub mod conversion_impls;
pub mod disputes;
pub mod encryption_interface;
pub mod errors;
pub mod events;
/// connector integrity check interface
pub mod integrity;
pub mod metrics;
pub mod secrets_interface;
pub mod types;
pub mod webhooks;
