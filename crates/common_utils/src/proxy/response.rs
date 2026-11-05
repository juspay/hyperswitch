//! The proxy response envelope, built from a connector's raw reply.

use serde_json::Value;

/// The `response` value of a proxy envelope: the connector body as JSON, else as UTF-8
/// text, else `null`. The exact bytes always travel in `response_body_base64`.
pub fn body_value(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or_else(|_| {
        std::str::from_utf8(body).map_or(Value::Null, |text| Value::String(text.to_owned()))
    })
}
