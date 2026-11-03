//! URL-encoded form bodies: only changed values are re-encoded, and duplicate keys and
//! their order are kept.

use serde_json::Value;

use super::{
    placeholder::{has_reference, substitute},
    CheckedContentType, PreparedProxyBody, ProxyBodyError, ProxyBodyFormat,
};

pub(super) struct FormUrlEncodedBody<'a>(pub(super) &'a str);

impl ProxyBodyFormat for FormUrlEncodedBody<'_> {
    fn name(&self) -> &'static str {
        "form_url_encoded"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("application/x-www-form-urlencoded")
    }

    fn accepts(&self, media_type: &str) -> bool {
        media_type == "application/x-www-form-urlencoded"
    }

    fn render(
        &self,
        vault: &Value,
        content_type: &CheckedContentType<'_>,
    ) -> Result<PreparedProxyBody, ProxyBodyError> {
        Ok(PreparedProxyBody {
            bytes: Some(resolve_form(self.0, vault)?.into_bytes()),
            content_type: content_type.or_default(self.default_content_type()),
        })
    }
}

fn decode_form(value: &str) -> Result<String, ProxyBodyError> {
    // urlencoding leaves malformed escapes untouched; reject them explicitly.
    let bytes = value.as_bytes();
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%' {
            let valid_escape = bytes
                .get(index + 1..index + 3)
                .is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit));
            if !valid_escape {
                return Err(ProxyBodyError::InvalidForm);
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    // url::form_urlencoded's public parser replaces invalid UTF-8; urlencoding
    // rejects it instead. `+` is replaced first so that `%2B` still decodes to `+`.
    urlencoding::decode(&value.replace('+', " "))
        .map(std::borrow::Cow::into_owned)
        .map_err(|_| ProxyBodyError::InvalidForm)
}

fn resolve_form(template: &str, vault: &Value) -> Result<String, ProxyBodyError> {
    template
        .split('&')
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            let decoded_key = decode_form(key)?;
            if has_reference(&decoded_key)? {
                return Err(ProxyBodyError::InvalidForm);
            }
            let decoded = decode_form(value)?;
            let resolved = substitute(&decoded, vault, str::to_owned)?;
            if resolved == decoded {
                return Ok(part.to_owned());
            }
            let encoded: String =
                url::form_urlencoded::byte_serialize(resolved.as_bytes()).collect();
            Ok(format!("{key}={encoded}"))
        })
        .collect::<Result<Vec<_>, ProxyBodyError>>()
        .map(|fields| fields.join("&"))
}
