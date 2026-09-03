//! The domain: what an alert *is* and what delivering one means, independent of HTTP.
//!
//! Traits, the types they exchange and the conversions between them live here rather than in
//! [`crate::core`], which holds the per-request logic a route calls. A handler deserializes, calls
//! into `core`, and serializes what comes back; `core` resolves a destination and asks the domain
//! to deliver. Nothing here knows that HTTP exists.

pub mod notifier;
