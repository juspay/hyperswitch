//! Logger of the system.

pub use tracing::{debug, error, event as log, info, warn};
pub use tracing_attributes::instrument;

pub mod config;
mod defaults;
pub use crate::config::Config;

// mod macros;
pub mod types;
pub use types::{Category, Flow, Level, Tag};

mod setup;
pub use setup::{setup, TelemetryGuard};

pub mod formatter;
pub use formatter::FormattingLayer;

pub mod span_path;
pub use span_path::current_span_path;

pub mod storage;
pub use storage::{Storage, StorageSubscription};
