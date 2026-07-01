//! Per-request context made available to deeply-nested code without threading
//! it through every call site.
//!
//! The `REQUEST_ID` task-local is entered once per HTTP request (by the
//! `RequestIdentifier` middleware) and can be read by any async code running
//! inside that request's task — including code in lower-level crates that
//! doesn't know about actix-web or HTTP.
//!
//! Background work (drainer, scheduler, consumer) and code reached via
//! `tokio::spawn` from a request handler will not see a value — `try_get`
//! returns `None`. Callers should treat that as legitimate (background work),
//! not an error.

tokio::task_local! {
    /// The request ID associated with the currently executing task, if any.
    pub static REQUEST_ID: String;
}

/// Read the request ID for the current task, if one has been set.
///
/// Returns `None` when called outside any `scope(...)` — e.g. from background
/// workers, or from a `tokio::spawn`'d task that didn't re-enter the scope.
pub fn try_get() -> Option<String> {
    REQUEST_ID.try_with(|id| id.clone()).ok()
}

/// Run `fut` with `id` bound as the current task's request ID.
///
/// Note: `tokio::task_local!` values are not inherited across `tokio::spawn`.
/// If a spawned task needs the request ID, capture it before the spawn and
/// re-enter the scope inside the spawned future.
pub async fn scope<F: std::future::Future>(id: String, fut: F) -> F::Output {
    REQUEST_ID.scope(id, fut).await
}
