//! The alert manager's own state: what an alert is, and whether it runs.
//!
//! This crate does two unrelated things, and this module is the boundary between them. The
//! notifier — [`crate::core::notifier`], [`crate::domain::notifier`] and the routes beside
//! them — *delivers* an alert somebody else decided to raise: a message goes out, nothing is kept.
//! Everything under here *stores* the alert manager's configuration and serves it back: rows go in
//! and out, and no message is sent. Neither half calls the other, and neither has a type the other
//! names.
//!
//! They share a crate because they share exactly two things — the HTTP server that mounts both
//! scopes, and the database pool on [`AppState`](crate::state::AppState). That is the whole of the
//! coupling, and keeping it that small is what this split is for: the two concerns were files
//! sitting side by side, which made "is this delivery or is this state?" a question about a
//! filename rather than about a directory.
//!
//! Laid out on the same lines as the crate around it, one level down: [`core`] decides, [`routes`]
//! exposes, [`types`] is the wire contract. The route *tree* is the deliberate exception and stays
//! in [`crate::routes::app`], which holds every path this service serves in one file — see its
//! module docs. The handlers it mounts for `/alerts/config` are the ones in [`routes`].

pub mod core;
pub mod routes;
pub mod types;
