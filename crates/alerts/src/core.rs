//! The domain layer: what this service actually *does*, independent of HTTP.
//!
//! Split from [`crate::routes`] on the router's own lines — `core` decides, `routes` exposes. A
//! handler's job is to deserialize, call in here, and serialize the result; anything a handler
//! knows that a background job or a test could not use belongs on this side of the boundary.
//!
//! `alerts` is the alerting plane and [`notifier`] is its first tenant, not its synonym. Further
//! alerting concerns are expected to arrive as siblings here.

pub mod notifier;
