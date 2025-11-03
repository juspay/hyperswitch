//! Payment Gateway Implementations
//!
//! This module contains concrete implementations of the PaymentGateway trait
//! for different payment flows using the Unified Connector Service (UCS).
//!
//! Each flow (Authorize, PSync, SetupMandate, etc.) has its own implementation
//! that handles the specific GRPC endpoint and request/response transformations.

#[macro_use]
pub mod macros;

pub mod authorize;
pub mod context;
pub mod helpers;
pub mod psync;
pub mod setup_mandate;
pub mod ucs_context;
pub mod ucs_execution_context;
pub mod ucs_executors;
// pub mod ucs_state_provider;

// Re-export for convenience
pub use authorize::*;
pub use context::*;
pub use helpers::*;
pub use psync::*;
pub use setup_mandate::*;
// pub use ucs_executors::*;s