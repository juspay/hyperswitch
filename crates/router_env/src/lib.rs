#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

//!
//! Environment of payment router: logger, basic config, its environment awareness.
//!

#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR" ), "/", "README.md"))]

/// Utilities to identify members of the current cargo workspace.
pub mod cargo_workspace;
pub mod env;
pub mod logger;
pub mod metrics;
/// `cargo` build instructions generation for obtaining information about the application
/// environment.
#[cfg(feature = "vergen")]
pub mod vergen;

// pub use literally;
#[doc(inline)]
pub use logger::*;
pub use once_cell;
pub use opentelemetry;
use strum::Display;
pub use tracing;
#[cfg(feature = "actix_web")]
pub use tracing_actix_web;
pub use tracing_appender;

#[doc(inline)]
pub use self::env::*;
use crate::types::FlowMetric;

/// Analytics Flow routes Enums
/// Info - Dimensions and filters available for the domain
/// Filters - Set of values present for the dimension
/// Metrics - Analytical data on dimensions and metrics
#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum AnalyticsFlow {
    GetInfo,
    GetPaymentMetrics,
    GetRefundsMetrics,
    GetSdkMetrics,
    GetPaymentFilters,
    GetRefundFilters,
    GetSdkEventFilters,
    GetApiEvents,
    GetSdkEvents,
    GeneratePaymentReport,
    GenerateDisputeReport,
    GenerateRefundReport,
    GetApiEventMetrics,
    GetApiEventFilters,
    GetConnectorEvents,
    GetOutgoingWebhookEvents,
}

impl FlowMetric for AnalyticsFlow {}
