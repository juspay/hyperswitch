//! The proxy response envelope, built from a connector's raw reply. SaaS and the injector
//! both build it through these functions, so the two envelopes always agree.

use serde_json::Value;

/// The `response` value of a proxy envelope: the connector body as JSON, else as UTF-8
/// text, else `null`. The exact bytes always travel in `response_body_base64`.
pub fn body_value(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or_else(|_| {
        std::str::from_utf8(body).map_or(Value::Null, |text| Value::String(text.to_owned()))
    })
}

/// Connector headers for `response_headers_multi`, keeping repeated names and their order.
/// A value that is not visible ASCII is converted lossily instead of failing a reply the
/// connector has already processed; `on_lossy` receives its header name.
pub fn headers_multi<'h>(
    headers: impl IntoIterator<Item = (&'h str, &'h [u8])>,
    mut on_lossy: impl FnMut(&str),
) -> Vec<(String, String)> {
    headers
        .into_iter()
        .map(|(name, value)| {
            if !value
                .iter()
                .all(|&byte| byte == b'\t' || (b' '..=b'~').contains(&byte))
            {
                on_lossy(name);
            }
            (name.to_owned(), String::from_utf8_lossy(value).into_owned())
        })
        .collect()
}
