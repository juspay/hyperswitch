//! HTTP handlers for the alert manager's resources, one module per resource.
//!
//! Only the handlers live here. The route tree that mounts them stays in [`crate::routes::app`]
//! with every other path this service serves.

pub mod config;
