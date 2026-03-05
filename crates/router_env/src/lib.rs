#![warn(missing_debug_implementations)]

//! Environment of payment router: logger, basic config, its environment awareness.

#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR" ), "/", "README.md"))]

/// Utilities to identify members of the current cargo workspace.
pub mod cargo_workspace;
pub mod env;
pub mod logger;
pub mod metrics;
#[cfg(feature = "actix_web")]
pub mod request_id;
#[cfg(feature = "actix_web")]
pub mod root_span;
/// `cargo` build instructions generation for obtaining information about the application
/// environment.
#[cfg(feature = "vergen")]
pub mod vergen;

// pub use literally;
#[doc(inline)]
pub use logger::*;
pub use opentelemetry;
// Re-export our internal request_id module for easier migration
#[cfg(feature = "actix_web")]
pub use request_id::{IdReuse, RequestId, RequestIdentifier};
#[cfg(feature = "actix_web")]
pub use root_span::CustomRootSpanBuilder;
pub use tracing;
#[cfg(feature = "actix_web")]
pub use tracing_actix_web;
pub use tracing_appender;

#[doc(inline)]
pub use self::env::*;
