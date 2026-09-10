//! Per-request logic for the alert manager's resources: lease a connection, run one query, turn
//! what comes back into the wire shape.
//!
//! The same split [`crate::core`] draws for the notifier, drawn again for state — `core` decides,
//! [`super::routes`] exposes. Nothing here is a notifier concern, and nothing here sends anything.

pub mod config;
