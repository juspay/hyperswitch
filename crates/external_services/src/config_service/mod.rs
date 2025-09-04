//! Configuration service for unified config management with Superposition support
//!
//! This module provides a simple interface for retrieving configuration values 
//! from multiple sources with automatic fallback logic:
//! 1. Superposition (feature flags) - if enabled
//! 2. Database - traditional config storage
//! 3. Default values - fallback
//!
//! The interface follows the same pattern as existing DB config methods,
//! requiring minimal changes to application code.

pub mod interface;
pub mod service;
pub mod superposition;

// Re-export commonly used types
pub use interface::{ConfigContext, ConfigServiceError, ConfigServiceInterface};
pub use service::{ConfigService, ConfigServiceConfig};
pub use superposition::{SuperpositionClient, SuperpositionConfig};