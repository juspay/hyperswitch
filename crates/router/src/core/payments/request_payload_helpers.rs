use crate::core::errors::ApiErrorResponse;
use error_stack::{Report, ResultExt};
/// Helper functions and utilities for propagating JSON request payloads through the payment processing pipeline
///
/// This module provides utilities to:
/// 1. Store and retrieve original request payloads in PaymentAdditionalData
/// 2. Extract specific fields from stored payloads
/// 3. Enable transformers to access the original request data when needed
///
/// Example usage:
/// ```rust
/// // In a transformer TryFrom implementation
/// if let Some(request_payload) = &additional_data.request_payload {
///     let field_value = request_payload.get("some_field").and_then(|v| v.as_str());
/// }
/// ```
use serde_json::{json, Value};

/// Serializes any serde-serializable request into a JSON Value for generic storage
pub fn serialize_request_to_json<T: serde::Serialize>(
    request: &T,
) -> Result<Value, Report<ApiErrorResponse>> {
    serde_json::to_value(request)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize request payload to JSON")
}

/// Extracts a specific field from the stored request payload
pub fn get_field_from_payload(payload: &Option<Value>, field_name: &str) -> Option<Value> {
    payload.as_ref().and_then(|v| v.get(field_name).cloned())
}

/// Extracts a string value from the stored request payload
pub fn get_string_field_from_payload(payload: &Option<Value>, field_name: &str) -> Option<String> {
    payload
        .as_ref()
        .and_then(|v| v.get(field_name))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extracts an i64 value from the stored request payload
pub fn get_i64_field_from_payload(payload: &Option<Value>, field_name: &str) -> Option<i64> {
    payload
        .as_ref()
        .and_then(|v| v.get(field_name))
        .and_then(|v| v.as_i64())
}

/// Extracts a boolean value from the stored request payload
pub fn get_bool_field_from_payload(payload: &Option<Value>, field_name: &str) -> Option<bool> {
    payload
        .as_ref()
        .and_then(|v| v.get(field_name))
        .and_then(|v| v.as_bool())
}

/// Extracts a nested object from the stored request payload
pub fn get_object_field_from_payload(payload: &Option<Value>, field_name: &str) -> Option<Value> {
    payload
        .as_ref()
        .and_then(|v| v.get(field_name))
        .filter(|v| v.is_object())
        .cloned()
}

/// Merges multiple field values from different payloads
pub fn merge_payload_fields(payload: &Option<Value>, fields: Vec<&str>) -> Value {
    let mut merged = json!({});

    if let Some(p) = payload {
        for field in fields {
            if let Some(value) = p.get(field) {
                if let Some(merged_obj) = merged.as_object_mut() {
                    merged_obj.insert(field.to_owned(), value.clone());
                }
            }
        }
    }

    merged
}

/// Convenience function to try serialize and return Option instead of Result
pub fn try_serialize_request_to_json<T: serde::Serialize>(request: &T) -> Option<Value> {
    serialize_request_to_json(request).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_string_field_from_payload() {
        let payload = json!({ "name": "test" });
        let result = get_string_field_from_payload(&Some(payload), "name");
        assert_eq!(result, Some("test".to_string()));
    }

    #[test]
    fn test_get_field_from_nonexistent_payload() {
        let result = get_string_field_from_payload(&None, "name");
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_i64_field() {
        let payload = json!({ "amount": 100 });
        let result = get_i64_field_from_payload(&Some(payload), "amount");
        assert_eq!(result, Some(100));
    }

    #[test]
    fn test_merge_payload_fields() {
        let payload = json!({
            "field1": "value1",
            "field2": "value2",
            "field3": "value3"
        });
        let merged = merge_payload_fields(&Some(payload), vec!["field1", "field2"]);
        assert!(merged.get("field1").is_some());
        assert!(merged.get("field2").is_some());
        assert!(merged.get("field3").is_none());
    }
}
