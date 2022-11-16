//!
//! Logger of the system.
//!

pub use tracing::{debug, error, event as log, info, instrument, warn};

pub mod config;
pub use crate::config::Config;

// mod macros;
pub mod types;
pub use types::{Category, Flow, Level, Tag};

mod setup;
pub use setup::{setup, TelemetryGuard};

pub mod formatter;
pub use formatter::FormattingLayer;

pub mod storage;
pub use storage::{Storage, StorageSubscription};
