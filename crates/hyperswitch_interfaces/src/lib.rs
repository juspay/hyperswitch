//! Hyperswitch interface
#![warn(missing_docs, missing_debug_implementations)]

pub mod api;
/// API client interface module
pub mod api_client;
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
/// Event handling interface
pub mod events;
/// helper utils
pub mod helpers;
/// connector integrity check interface
pub mod integrity;
pub mod metrics;
pub mod secrets_interface;
pub mod types;
/// ucs handlers
pub mod unified_connector_service;
pub mod webhooks;

/// Crm interface
pub mod crm;
