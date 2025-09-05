//! Superposition integration for dynamic configuration management
//!
//! This module provides a simple interface for retrieving configuration values 
//! with automatic fallback logic:
//! 1. Superposition (dynamic config) - if enabled and available
//! 2. Default values - reliable fallback
//!
//! The interface follows the same pattern as existing config methods,
//! requiring minimal changes to application code.

/// Interface definitions for superposition service
pub mod interface;
/// Service implementation for superposition integration
pub mod service;
/// Superposition client wrapper and configuration
pub mod superposition;

// Re-export commonly used types
pub use interface::{ConfigContext, SuperpositionError, SuperpositionInterface};
pub use service::{SuperpositionService, SuperpositionConfig};
pub use superposition::{SuperpositionClient, SuperpositionClientConfig};