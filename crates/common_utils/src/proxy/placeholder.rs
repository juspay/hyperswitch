//! `{{$field}}` placeholder parsing and vault field lookup, shared by every body
//! format and by the injector.

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

fn scalar(vault: &Value, field: &str) -> Result<String, ProxyBodyError> {
    match find_field(vault, field) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(Value::Bool(b)) => Ok(b.to_string()),
        _ => Err(ProxyBodyError::InvalidField(field.to_owned())),
    }
}

/// Parse a `{{$field}}` placeholder at the start of `input`.
///
/// Returns the field name and the number of bytes consumed, `None` when `input` does not
/// start with a placeholder, and an error for a malformed one such as `{{$}}`.
pub fn parse_placeholder(input: &str) -> Result<Option<(&str, usize)>, ProxyBodyError> {
    let Some(rest) = input.strip_prefix("{{") else {
        return Ok(None);
    };
    if !rest.trim_start().starts_with('$') {
        return Ok(None);
    }
    let end = rest.find("}}").ok_or(ProxyBodyError::InvalidPlaceholder)?;
    let field = rest[..end]
        .trim()
        .strip_prefix('$')
        .ok_or(ProxyBodyError::InvalidPlaceholder)?;
    if field.is_empty() || !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ProxyBodyError::InvalidPlaceholder);
    }
    Ok(Some((field, end + 4)))
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
        if let Some((field, consumed)) = parse_placeholder(rest)? {
            output.push_str(&escape(&scalar(vault, field)?));
            rest = &rest[consumed..];
        } else {
            output.push_str("{{");
            rest = &rest[2..];
        }
    }
    output.push_str(rest);
    Ok(output)
}

/// Whether `template` contains at least one placeholder.
pub(super) fn has_reference(template: &str) -> Result<bool, ProxyBodyError> {
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        rest = &rest[start..];
        if parse_placeholder(rest)?.is_some() {
            return Ok(true);
        }
        rest = &rest[2..];
    }
    Ok(false)
}
