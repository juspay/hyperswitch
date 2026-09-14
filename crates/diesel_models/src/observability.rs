//! The observability plane's own state: alert configuration, lifecycle, and what was announced.
//!
//! These tables are not part of `hyperswitch_db`. They live in a database the observability plane
//! owns, with its own migration lineage under `crates/observability/migrations` and its own diesel
//! configuration in `diesel_observability.toml`.

pub mod alerts_dicts;
pub mod alerts_info;
pub mod merchant_thresholds;
pub mod merchants_alert_external_config;
pub mod notification_reads;
pub mod raw_json;
pub mod schema;
