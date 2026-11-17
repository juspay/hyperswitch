//! JSON bodies: placeholders are resolved inside string values, and a placeholder that is a
//! whole value keeps the vault value's JSON type.

use serde_json::Value;

use super::{
    placeholder::{parse_placeholder, substitute, validate_placeholders, value},
    ProxyBodyError,
};

/// Check placeholder syntax in every string value.
pub(super) fn validate(item: &Value) -> Result<(), ProxyBodyError> {
    match item {
        Value::String(text) => validate_placeholders(text),
        Value::Array(items) => items.iter().try_for_each(validate),
        Value::Object(fields) => fields.values().try_for_each(validate),
        _ => Ok(()),
    }
}

/// The JSON bytes with placeholders replaced by vault values.
pub(super) fn render(body: &Value, vault: &Value) -> Result<Vec<u8>, ProxyBodyError> {
    serde_json::to_vec(&resolve(body, vault)?).map_err(|_| ProxyBodyError::Serialization)
}

/// Resolve placeholders in string values; object keys are never interpolated.
fn resolve(item: &Value, vault: &Value) -> Result<Value, ProxyBodyError> {
    match item {
        Value::String(text) => match parse_placeholder(text)? {
            Some((field, consumed)) if consumed == text.len() => value(vault, field),
            _ => substitute(text, vault, str::to_owned).map(Value::String),
        },
        Value::Array(items) => items
            .iter()
            .map(|item| resolve(item, vault))
            .collect::<Result<_, _>>()
            .map(Value::Array),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, item)| resolve(item, vault).map(|item| (key.clone(), item)))
            .collect::<Result<_, _>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}
