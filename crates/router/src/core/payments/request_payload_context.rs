use std::cell::RefCell;

/// Request payload context storage using thread-local storage
///
/// This module provides a thread-safe way to store and retrieve request payloads
/// throughout the payment processing pipeline without modifying all function signatures.
///
/// The approach uses thread-local storage which is automatically cleaned up at the
/// end of each request, ensuring no cross-request contamination.
use serde_json::Value;

thread_local! {
    /// Thread-local storage for the current request's serialized payload
    static REQUEST_PAYLOAD: RefCell<Option<Value>> = const { RefCell::new(None) };
}

/// Store a request payload in thread-local context
///
/// This should be called early in the request handling, typically in route handlers.
/// The payload is stored as JSON Value to support multiple request types generically.
///
/// # Example
/// ```rust
/// let payload = json_payload.into_inner();
/// let serialized = payments::request_payload_helpers::try_serialize_request_to_json(&payload);
/// payments::request_payload_context::set_request_payload(serialized);
/// ```
pub fn set_request_payload(payload: Option<Value>) {
    REQUEST_PAYLOAD.with(|p| {
        *p.borrow_mut() = payload;
    });
}

/// Retrieve a request payload from thread-local context
///
/// Returns a clone of the stored payload, or None if no payload was set.
/// This is safe to call from anywhere in the request processing pipeline.
///
/// # Example
/// ```rust
/// if let Some(payload) = payments::request_payload_context::get_request_payload() {
///     let field = payload.get("field_name");
/// }
/// ```
pub fn get_request_payload() -> Option<Value> {
    REQUEST_PAYLOAD.with(|p| p.borrow().clone())
}

/// Clear the request payload context
///
/// This is automatically called at the end of each request, but can be manually
/// invoked if needed for cleanup.
pub fn clear_request_payload() {
    REQUEST_PAYLOAD.with(|p| {
        *p.borrow_mut() = None;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_payload() {
        let payload = serde_json::json!({"field": "value"});
        set_request_payload(Some(payload.clone()));

        let retrieved = get_request_payload();
        assert_eq!(retrieved, Some(payload));
    }

    #[test]
    fn test_get_empty_payload() {
        clear_request_payload();
        let retrieved = get_request_payload();
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_clear_payload() {
        let payload = serde_json::json!({"field": "value"});
        set_request_payload(Some(payload));

        clear_request_payload();
        let retrieved = get_request_payload();
        assert_eq!(retrieved, None);
    }
}
