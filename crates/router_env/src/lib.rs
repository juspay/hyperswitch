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
pub use tracing;
#[cfg(feature = "actix_web")]
pub use tracing_actix_web;
pub use tracing_appender;

#[doc(inline)]
pub use self::env::*;
