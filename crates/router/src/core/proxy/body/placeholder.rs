//! `{{$field}}` and `{{$field | filter}}` placeholder parsing and vault field lookup, shared by
//! every body format.

use common_utils::{date_time, fp_utils::when};
use serde_json::Value;

use super::ProxyBodyError;

/// A function applied to a vault value before it is written, as `{{$field | filter}}`.
#[derive(Clone, Copy, Debug, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Filter {
    /// The value's length in UTF-8 bytes, such as the field lengths a connector needs next to an
    /// encrypted block.
    ByteLength,
    /// A 2- or 4-digit year as 4 digits; 2 digits are taken as the current century.
    #[strum(serialize = "exp_year_4")]
    ExpYear4,
}

impl Filter {
    /// The filtered value; `None` when the value is not valid input for the filter.
    fn apply(self, value: &str) -> Option<String> {
        match self {
            Self::ByteLength => Some(value.len().to_string()),
            Self::ExpYear4 => {
                let digits = value.bytes().all(|byte| byte.is_ascii_digit());
                match value.len() {
                    2 if digits => Some(format!("{}{value}", date_time::now().year() / 100)),
                    4 if digits => Some(value.to_owned()),
                    _ => None,
                }
            }
        }
    }
}

/// A parsed placeholder: the vault field and an optional filter.
#[derive(Debug)]
pub struct Placeholder<'a> {
    /// The vault field name
    pub field: &'a str,
    /// The filter applied to the field's value
    pub filter: Option<Filter>,
}

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

/// The value of a placeholder embedded in text, after its filter.
fn scalar(vault: &Value, placeholder: &Placeholder<'_>) -> Result<String, ProxyBodyError> {
    let field = placeholder.field;
    let text = match find_field(vault, field) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(Value::Bool(b)) => Ok(b.to_string()),
        _ => Err(ProxyBodyError::InvalidField(field.to_owned())),
    }?;
    match placeholder.filter {
        None => Ok(text),
        Some(filter) => filter
            .apply(&text)
            .ok_or_else(|| ProxyBodyError::InvalidFilterInput(field.to_owned())),
    }
}

/// The value of a placeholder that is a whole JSON value, keeping its JSON type.
pub(super) fn value(vault: &Value, field: &str) -> Result<Value, ProxyBodyError> {
    find_field(vault, field)
        .cloned()
        .ok_or_else(|| ProxyBodyError::InvalidField(field.to_owned()))
}

/// Parse a `{{$field}}` or `{{$field | filter}}` placeholder at the start of `input`.
///
/// Returns the placeholder and the number of bytes consumed, `None` when `input` does not
/// start with a placeholder, and an error for a malformed one such as `{{$}}` or one with an
/// unknown filter.
pub fn parse_placeholder(input: &str) -> Result<Option<(Placeholder<'_>, usize)>, ProxyBodyError> {
    input
        .strip_prefix("{{")
        .filter(|rest| rest.trim_start().starts_with('$'))
        .map(|rest| -> Result<_, ProxyBodyError> {
            let end = rest.find("}}").ok_or(ProxyBodyError::InvalidPlaceholder)?;
            let inner = &rest[..end];
            let (field, filter) = inner
                .split_once('|')
                .map_or((inner, None), |(field, filter)| {
                    (field, Some(filter.trim()))
                });
            let field = field
                .trim()
                .strip_prefix('$')
                .ok_or(ProxyBodyError::InvalidPlaceholder)?;
            when(
                field.is_empty() || !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                || Err(ProxyBodyError::InvalidPlaceholder),
            )?;
            let filter = filter
                .map(|name| name.parse().map_err(|_| ProxyBodyError::InvalidPlaceholder))
                .transpose()?;
            Ok((Placeholder { field, filter }, end + 4))
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
            Some((placeholder, consumed)) => {
                output.push_str(&escape(&scalar(vault, &placeholder)?));
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
