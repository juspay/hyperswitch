//! JSON bodies: placeholders are resolved inside string values only, and a placeholder
//! that is a whole value keeps the vault value's JSON type.

use serde_json::Value;

use super::{
    placeholder::{find_field, parse_placeholder, substitute},
    CheckedContentType, PreparedProxyBody, ProxyBodyError, ProxyBodyFormat,
};

pub(super) struct JsonBody<'a>(pub(super) &'a str);

impl ProxyBodyFormat for JsonBody<'_> {
    fn name(&self) -> &'static str {
        "json"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("application/json")
    }

    fn accepts(&self, media_type: &str) -> bool {
        media_type == "application/json" || media_type.ends_with("+json")
    }

    fn legacy_value(&self) -> Value {
        serde_json::from_str(self.0).unwrap_or_else(|_| Value::String(self.0.to_owned()))
    }

    fn render(
        &self,
        vault: &Value,
        content_type: &CheckedContentType<'_>,
    ) -> Result<PreparedProxyBody, ProxyBodyError> {
        Ok(PreparedProxyBody {
            bytes: Some(resolve_json_template(self.0, vault)?.into_bytes()),
            content_type: content_type.or_default(self.default_content_type()),
        })
    }
}

fn resolve_json_string(value: &str, vault: &Value) -> Result<Value, ProxyBodyError> {
    if let Some((field, consumed)) = parse_placeholder(value)? {
        if consumed == value.len() {
            return find_field(vault, field)
                .cloned()
                .ok_or_else(|| ProxyBodyError::InvalidField(field.to_owned()));
        }
    }
    substitute(value, vault, str::to_owned).map(Value::String)
}

fn resolve_json_template(template: &str, vault: &Value) -> Result<String, ProxyBodyError> {
    // Validate structure once, then replace only string value tokens. This keeps
    // whitespace, field order, numeric spelling and existing escapes unchanged.
    serde_json::from_str::<Value>(template).map_err(|_| ProxyBodyError::Serialization)?;
    let bytes = template.as_bytes();
    let mut output = String::with_capacity(template.len());
    let mut copied = 0;
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte != b'"' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while let Some(&byte) = bytes.get(index) {
            if byte == b'"' {
                index += 1;
                break;
            }
            index += if byte == b'\\' { 2 } else { 1 };
        }
        let token = template
            .get(start..index)
            .ok_or(ProxyBodyError::Serialization)?;
        let is_key = bytes
            .get(index..)
            .unwrap_or_default()
            .iter()
            .copied()
            .find(|b| !b.is_ascii_whitespace())
            == Some(b':');
        if is_key {
            continue;
        }
        let original: String =
            serde_json::from_str(token).map_err(|_| ProxyBodyError::Serialization)?;
        let resolved = resolve_json_string(&original, vault)?;
        if resolved.as_str() != Some(original.as_str()) {
            output.push_str(&template[copied..start]);
            output.push_str(
                &serde_json::to_string(&resolved).map_err(|_| ProxyBodyError::Serialization)?,
            );
            copied = index;
        }
    }
    output.push_str(&template[copied..]);
    Ok(output)
}
