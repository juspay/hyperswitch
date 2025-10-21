//! Payment Gateway Implementations
//!
//! This module contains concrete implementations of the PaymentGateway trait
//! for different payment flows using the Unified Connector Service (UCS).
//!
//! Each flow (Authorize, PSync, SetupMandate, etc.) has its own implementation
//! that handles the specific GRPC endpoint and request/response transformations.

pub mod authorize;
pub mod helpers;
pub mod psync;
pub mod setup_mandate;

// Re-export for convenience
pub use authorize::*;
pub use helpers::*;
pub use psync::*;
pub use setup_mandate::*;