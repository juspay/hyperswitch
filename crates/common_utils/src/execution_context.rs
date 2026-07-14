//! Execution context shared by infrastructure clients for request correlation.

/// Minimal request correlation context for infrastructure calls.
///
/// This intentionally avoids the name `Context` to prevent confusion with
/// `error_stack::Context`, and starts with request ID only. Additional
/// correlation fields can be added later when there is a concrete consumer.
pub trait ExecutionContext: Send + Sync {
    /// Return the request ID associated with the current execution, if any.
    fn request_id(&self) -> Option<&str>;
}
