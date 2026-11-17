//! `{{$field}}` placeholder parsing and vault field lookup, shared by every body format.

use common_utils::fp_utils::when;
use serde_json::Value;

use super::ProxyBodyError;

/// Depth-first lookup of a vault field by name, through nested objects and arrays.
pub fn find_field<'a>(vault: &'a Value, field: &str) -> Option<&'a Value> {
    match vault {
        Value::Object(map) => map
            .get(field)
            .or_else(|| map.values().find_map(|v| find_field(v, field))),
        Value::Array(values) => values.iter().find_map(|v| find_field(v, field)),
        _ => None,
    }
}

/// The value of a placeholder embedded in text.
fn scalar(vault: &Value, field: &str) -> Result<String, ProxyBodyError> {
    match find_field(vault, field) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(Value::Bool(b)) => Ok(b.to_string()),
        _ => Err(ProxyBodyError::InvalidField(field.to_owned())),
    }
}

/// The value of a placeholder that is a whole JSON value, keeping its JSON type.
pub(super) fn value(vault: &Value, field: &str) -> Result<Value, ProxyBodyError> {
    find_field(vault, field)
        .cloned()
        .ok_or_else(|| ProxyBodyError::InvalidField(field.to_owned()))
}

/// Parse a `{{$field}}` placeholder at the start of `input`.
///
/// Returns the field name and the number of bytes consumed, `None` when `input` does not
/// start with a placeholder, and an error for a malformed one such as `{{$}}`.
pub fn parse_placeholder(input: &str) -> Result<Option<(&str, usize)>, ProxyBodyError> {
    input
        .strip_prefix("{{")
        .filter(|rest| rest.trim_start().starts_with('$'))
        .map(|rest| -> Result<_, ProxyBodyError> {
            let end = rest.find("}}").ok_or(ProxyBodyError::InvalidPlaceholder)?;
            let field = rest[..end]
                .trim()
                .strip_prefix('$')
                .ok_or(ProxyBodyError::InvalidPlaceholder)?;
            when(
                field.is_empty() || !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                || Err(ProxyBodyError::InvalidPlaceholder),
            )?;
            Ok((field, end + 4))
        })
        .transpose()
}

/// Replace every placeholder in `template` with its scalar vault value, escaped for
/// the surrounding format.
pub(super) fn substitute(
    template: &str,
    vault: &Value,
    escape: impl Fn(&str) -> String,
) -> Result<String, ProxyBodyError> {
    let mut output = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        rest = &rest[start..];
        match parse_placeholder(rest)? {
            Some((field, consumed)) => {
                output.push_str(&escape(&scalar(vault, field)?));
                rest = &rest[consumed..];
            }
            // Not a placeholder: keep the `{{` as text.
            None => {
                output.push_str("{{");
                rest = &rest[2..];
            }
        }
    }
    output.push_str(rest);
    Ok(output)
}

/// Check the syntax of every placeholder in `template`.
pub(super) fn validate_placeholders(template: &str) -> Result<(), ProxyBodyError> {
    template
        .match_indices("{{")
        .try_for_each(|(start, _)| parse_placeholder(&template[start..]).map(|_| ()))
}

/// Whether `template` contains at least one placeholder.
pub(super) fn has_reference(template: &str) -> Result<bool, ProxyBodyError> {
    // The first valid or malformed placeholder decides; other `{{` text is skipped.
    template
        .match_indices("{{")
        .find_map(|(start, _)| parse_placeholder(&template[start..]).transpose())
        .transpose()
        .map(|placeholder| placeholder.is_some())
}
