//! External service call event emission types and traits.

use serde::Serialize;

/// Represents a completed call to an external service.
#[derive(Debug, Clone, Serialize)]
pub struct ExternalServiceCall {
    /// Name of the external service (e.g., "keymanager", "database", "redis")
    pub service_name: String,
    /// The endpoint or resource accessed (e.g., "key/create", table name, cache key)
    pub endpoint: String,
    /// The operation performed (e.g., "POST", "INSERT", "GET")
    pub method: String,
    /// Request ID for tracking
    pub request_id: String,
    /// Status code of the response (e.g., HTTP status code, 0 if not applicable)
    pub status_code: u16,
    /// Whether the call was successful
    pub success: bool,
    /// Latency in milliseconds
    pub latency_ms: u128,
    /// Timestamp when the call completed (nanoseconds since Unix epoch)
    pub created_at_timestamp: i128,
}

/// Trait for emitting external service call events.
/// Implementations typically send events to Kafka or log them.
pub trait ExternalServiceEventEmitter: std::fmt::Debug + Send + Sync {
    /// Emit an external service call event.
    fn emit_external_service_call(&self, event: ExternalServiceCall);
}

/// A no-op event emitter that discards all events.
/// Used in tests and when no event emission is needed.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpEventEmitter;

impl ExternalServiceEventEmitter for NoOpEventEmitter {
    fn emit_external_service_call(&self, _event: ExternalServiceCall) {
        // No-op: discard the event
    }
}
