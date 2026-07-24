//! Request correlation context shared by infrastructure clients.

/// Minimal request correlation context for infrastructure calls.
///
/// This intentionally avoids the name `Context` to prevent confusion with
/// `error_stack::Context`, and carries the request ID only.
pub trait RequestIdContext: Send + Sync {
    /// Return the request ID associated with the current execution, if any.
    fn request_id(&self) -> Option<&str>;
}
