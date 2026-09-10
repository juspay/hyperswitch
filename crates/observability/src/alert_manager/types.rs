//! The wire contract for the alert manager's resources: what a caller sends and what it gets back.
//!
//! Separate from [`crate::types`], which is the notifier's contract and describes one message going
//! out. These resources describe rows, and they have their own shapes and their own reasons.

pub mod config;
