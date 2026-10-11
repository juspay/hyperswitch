// Builds Smithy models at compile time and does not depend on common_utils.
#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

// // crates/smithy-core/lib.rs

pub mod generator;
pub mod types;

pub use generator::SmithyGenerator;
pub use types::*;
