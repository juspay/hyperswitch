//! Per-request logic: what happens between deserializing a request and serializing a response.
//!
//! Split from [`crate::routes`] on the router's own lines — `core` decides, `routes` exposes. A
//! handler's job is to deserialize, call in here, and serialize the result.
//!
//! Distinct from [`crate::domain`], which holds the traits and the types they exchange: `domain`
//! says what delivering an alert *is*, `core` says what one HTTP request does about it.

pub mod notifier;
