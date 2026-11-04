//! URL-encoded form bodies: only changed values are re-encoded, and duplicate keys and
//! their order are kept.

use super::{
    placeholder::{has_reference, substitute, Vault},
    ConnectorContentType, ProxyBodyError, ProxyBodyFormat, RenderedBody,
};
use crate::fp_utils::when;

pub(super) struct FormUrlEncodedBody<'a>(pub(super) &'a str);

impl ProxyBodyFormat for FormUrlEncodedBody<'_> {
    fn name(&self) -> &'static str {
        "form_url_encoded"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("application/x-www-form-urlencoded")
    }

    fn accepts(&self, content_type: &ConnectorContentType<'_>) -> bool {
        content_type.is_form_url_encoded()
    }

    fn render(
        &self,
        vault: Vault<'_>,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<RenderedBody, ProxyBodyError> {
        Ok(RenderedBody {
            bytes: Some(resolve_form(self.0, vault)?.into_bytes()),
            content_type: self.content_type_header(content_type),
        })
    }
}

fn decode_form(value: &str) -> Result<String, ProxyBodyError> {
    // urlencoding leaves malformed escapes untouched; reject them explicitly.
    let valid_escapes = value.split('%').skip(1).all(|escape| {
        escape
            .as_bytes()
            .get(..2)
            .is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit))
    });
    when(!valid_escapes, || Err(ProxyBodyError::InvalidForm))?;
    // url::form_urlencoded's public parser replaces invalid UTF-8; urlencoding
    // rejects it instead. `+` is replaced first so that `%2B` still decodes to `+`.
    urlencoding::decode(&value.replace('+', " "))
        .map(std::borrow::Cow::into_owned)
        .map_err(|_| ProxyBodyError::InvalidForm)
}

fn resolve_form(template: &str, vault: Vault<'_>) -> Result<String, ProxyBodyError> {
    template
        .split('&')
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            // Keys are never interpolated.
            when(has_reference(&decode_form(key)?)?, || {
                Err(ProxyBodyError::InvalidForm)
            })?;
            let decoded = decode_form(value)?;
            let resolved = substitute(&decoded, vault, str::to_owned)?;
            Ok(if resolved == decoded {
                part.to_owned()
            } else {
                let encoded: String =
                    url::form_urlencoded::byte_serialize(resolved.as_bytes()).collect();
                format!("{key}={encoded}")
            })
        })
        .collect::<Result<Vec<_>, ProxyBodyError>>()
        .map(|fields| fields.join("&"))
}
