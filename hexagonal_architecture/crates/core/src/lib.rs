//!
//! Core logic for a hexagonal_architecture.
//!

#![forbid(unsafe_code)]
#![forbid(non_ascii_idents)]
#![warn(missing_docs)]
#![warn(clippy::use_self)]
#![warn(rust_2018_idioms)]
#![warn(missing_debug_implementations)]

/// Module.
pub mod connector;
/// Module.
pub mod payments;
/// Module.
pub mod store;
/// Module.
pub mod types;
