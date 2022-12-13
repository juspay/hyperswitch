#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::panicking_unwrap,
    clippy::unreachable,
    clippy::unwrap_in_result,
    clippy::unwrap_used
)]

//!
//! Environment of payment router: logger, basic config, its environment awareness.
//!

#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR" ), "/", "README.md"))]

pub mod env;
pub mod logger;
/// `cargo` build instructions generation for obtaining information about the application
/// environment.
#[cfg(feature = "vergen")]
pub mod vergen;

// pub use literally;
#[doc(inline)]
pub use logger::*;
pub use opentelemetry;
pub use tracing;
#[cfg(feature = "actix_web")]
pub use tracing_actix_web;
pub use tracing_appender;

#[doc(inline)]
pub use self::env::*;
