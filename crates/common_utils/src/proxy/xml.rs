//! XML and SOAP bodies: placeholders are resolved in text, attribute values and CDATA,
//! escaped for each position.

use quick_xml::{events::Event, Reader};
use serde_json::Value;

use super::{
    placeholder::{has_reference, substitute},
    CheckedContentType, PreparedProxyBody, ProxyBodyError, ProxyBodyFormat,
};

pub(super) struct XmlBody<'a>(pub(super) &'a str);

impl ProxyBodyFormat for XmlBody<'_> {
    fn name(&self) -> &'static str {
        "xml"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("application/xml")
    }

    fn accepts(&self, media_type: &str) -> bool {
        media_type == "application/xml" || media_type == "text/xml" || media_type.ends_with("+xml")
    }

    fn render(
        &self,
        vault: &Value,
        content_type: &CheckedContentType<'_>,
    ) -> Result<PreparedProxyBody, ProxyBodyError> {
        Ok(PreparedProxyBody {
            bytes: Some(resolve_xml(self.0, vault)?.into_bytes()),
            content_type: content_type.or_default(self.default_content_type()),
        })
    }
}

/// `quick_xml::escape::escape` leaves `\r`, `\n` and `\t` as-is, which XML parsers
/// normalise to spaces inside attribute values; they are escaped here to survive.
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}

fn resolve_xml(template: &str, vault: &Value) -> Result<String, ProxyBodyError> {
    let valid_chars = |value: &str| {
        value.chars().all(|c| {
            matches!(c, '\t' | '\r' | '\n')
                || ('\u{20}'..='\u{d7ff}').contains(&c)
                || ('\u{e000}'..='\u{fffd}').contains(&c)
                || ('\u{10000}'..='\u{10ffff}').contains(&c)
        })
    };
    if !valid_chars(template) {
        return Err(ProxyBodyError::InvalidXml);
    }
    let mut reader = Reader::from_str(template);
    let mut output = String::with_capacity(template.len());
    let mut position = 0;
    let mut depth: usize = 0;
    let mut roots = 0;
    loop {
        let event = reader
            .read_event()
            .map_err(|_| ProxyBodyError::InvalidXml)?;
        let next = reader.buffer_position();
        let original = &template[position..next];
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if element
                    .name()
                    .as_ref()
                    .windows(2)
                    .any(|value| value == b"{{")
                {
                    return Err(ProxyBodyError::InvalidXml);
                }
                for attr in element.attributes() {
                    let attr = attr.map_err(|_| ProxyBodyError::InvalidXml)?;
                    if attr.key.as_ref().windows(2).any(|value| value == b"{{") {
                        return Err(ProxyBodyError::InvalidXml);
                    }
                }
                if depth == 0 {
                    roots += 1;
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
                output.push_str(&substitute(original, vault, xml_escape)?);
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(ProxyBodyError::InvalidXml)?;
                output.push_str(original);
            }
            Event::Text(_) => {
                if depth == 0 && !original.trim().is_empty() {
                    return Err(ProxyBodyError::InvalidXml);
                }
                output.push_str(&substitute(original, vault, xml_escape)?);
            }
            Event::CData(_) => {
                if depth == 0 {
                    return Err(ProxyBodyError::InvalidXml);
                }
                output.push_str(&substitute(original, vault, |s| {
                    s.replace("]]>", "]]]]><![CDATA[>")
                })?);
            }
            Event::Decl(ref declaration) => {
                let encoding = declaration
                    .encoding()
                    .transpose()
                    .map_err(|_| ProxyBodyError::InvalidXml)?;
                if has_reference(original)?
                    || encoding.is_some_and(|encoding| {
                        !encoding.eq_ignore_ascii_case(b"utf-8")
                            && !encoding.eq_ignore_ascii_case(b"utf8")
                    })
                {
                    return Err(ProxyBodyError::InvalidXml);
                }
                output.push_str(original);
            }
            Event::Eof => break,
            _ => {
                // Token references in comments, declarations, DTDs and processing
                // instructions are not card data positions.
                if has_reference(original)? {
                    return Err(ProxyBodyError::InvalidXml);
                }
                output.push_str(original);
            }
        }
        position = next;
    }
    if roots != 1 || depth != 0 || !valid_chars(&output) {
        return Err(ProxyBodyError::InvalidXml);
    }
    Ok(output)
}
