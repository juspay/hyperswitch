//! The workspace's way to start a task.
//!
//! A spawned future runs outside the caller's span unless it carries one, so its
//! logs lose the request and, under deja, its boundaries record uncorrelated.
//! These take only an instrumented future, so a bare one does not compile:
//! `.in_current_span()` for work that belongs to the caller, or
//! `.instrument(tracing::debug_span!("name"))` to give concurrent siblings
//! addresses of their own.

use std::future::Future;

use tokio::task::{AbortHandle, JoinHandle, JoinSet};
use tracing::instrument::Instrumented;

/// `tokio::spawn` for a future that already carries its span.
#[allow(
    clippy::disallowed_methods,
    reason = "the sanctioned spawn entry point"
)]
pub fn spawn<F>(future: Instrumented<F>) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

/// `JoinSet::spawn` for a future that already carries its span.
#[allow(
    clippy::disallowed_methods,
    reason = "the sanctioned spawn entry point"
)]
pub fn spawn_in_set<F>(set: &mut JoinSet<F::Output>, future: Instrumented<F>) -> AbortHandle
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    set.spawn(future)
}
