//! The observability plane's own state: alert configuration, lifecycle, and what was announced.
//!
//! These tables are not part of `hyperswitch_db`. They live in a database the observability plane
//! owns, with its own migration lineage under `crates/observability/migrations` and its own diesel
//! configuration in `diesel_observability.toml`.

pub mod alerts_dicts;
pub mod alerts_info;
pub mod alerts_intermediate;
pub mod alerts_main;
pub mod merchant_thresholds;
pub mod merchants_alert_external;
pub mod merchants_alert_external_config;
pub mod merchants_alert_external_dimension;
pub mod notification_reads;
pub mod raw_json;
pub mod schema;
