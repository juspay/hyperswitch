//! JSON bodies: placeholders are resolved inside string values, and a placeholder that is a
//! whole value keeps the vault value's JSON type.

use serde_json::Value;

use super::{
    placeholder::{parse_placeholder, substitute, Vault},
    ProxyBodyError, ProxyBodyFormat,
};

pub(super) struct JsonBody<'a>(pub(super) &'a Value);

impl ProxyBodyFormat for JsonBody<'_> {
    fn name(&self) -> &'static str {
        "json"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("application/json")
    }

    fn render(&self, vault: Vault<'_>) -> Result<Option<Vec<u8>>, ProxyBodyError> {
        serde_json::to_vec(&resolve(self.0, vault)?)
            .map(Some)
            .map_err(|_| ProxyBodyError::Serialization)
    }
}

/// Resolve placeholders in string values; object keys are never interpolated.
fn resolve(value: &Value, vault: Vault<'_>) -> Result<Value, ProxyBodyError> {
    match value {
        Value::String(text) => match parse_placeholder(text)? {
            Some((field, consumed)) if consumed == text.len() => vault.value(field),
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
