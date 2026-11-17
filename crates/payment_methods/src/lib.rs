#[cfg(all(
    feature = "required_fields_sync",
    any(not(feature = "v1"), feature = "v2")
))]
compile_error!("required_fields_sync supports v1 only; enable v1 and disable v2");

pub mod client;
pub mod configs;
pub mod controller;
pub mod core;
pub mod helpers;
pub mod single_use_token;
pub mod state;
#[cfg(feature = "v1")]
pub mod types;

pub use crate::core::errors;
