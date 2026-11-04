//! XML and SOAP bodies: placeholders are resolved in text, attribute values and CDATA,
//! escaped for each position.

use quick_xml::{events::Event, Reader};

use super::{
    is_utf8_label,
    placeholder::{has_reference, substitute, Vault},
    ConnectorContentType, ProxyBodyError, ProxyBodyFormat, RenderedBody,
};
use crate::fp_utils::when;

pub(super) struct XmlBody<'a>(pub(super) &'a str);

impl ProxyBodyFormat for XmlBody<'_> {
    fn name(&self) -> &'static str {
        "xml"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("application/xml")
    }

    fn accepts(&self, content_type: &ConnectorContentType<'_>) -> bool {
        content_type.is_xml()
    }

    fn render(
        &self,
        vault: Vault<'_>,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<RenderedBody, ProxyBodyError> {
        Ok(RenderedBody {
            bytes: Some(resolve_xml(self.0, vault)?.into_bytes()),
            content_type: self.content_type_header(content_type),
        })
    }
}

/// `quick_xml::escape::escape` leaves `\r`, `\n` and `\t` as-is, which XML parsers
/// normalize to spaces inside attribute values; they are escaped here to survive.
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

/// Whether an element or attribute name contains a placeholder marker.
fn has_marker(name: &[u8]) -> bool {
    name.windows(2).any(|pair| pair == b"{{")
}

fn resolve_xml(template: &str, vault: Vault<'_>) -> Result<String, ProxyBodyError> {
    let valid_chars = |value: &str| {
        value.chars().all(|c| {
            matches!(c, '\t' | '\r' | '\n')
                || ('\u{20}'..='\u{d7ff}').contains(&c)
                || ('\u{e000}'..='\u{fffd}').contains(&c)
                || ('\u{10000}'..='\u{10ffff}').contains(&c)
        })
    };
    let invalid = || Err(ProxyBodyError::InvalidXml);
    when(!valid_chars(template), invalid)?;
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
                when(has_marker(element.name().as_ref()), invalid)?;
                for attr in element.attributes() {
                    let attr = attr.map_err(|_| ProxyBodyError::InvalidXml)?;
                    when(has_marker(attr.key.as_ref()), invalid)?;
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
                when(depth == 0 && !original.trim().is_empty(), invalid)?;
                output.push_str(&substitute(original, vault, xml_escape)?);
            }
            Event::CData(_) => {
                when(depth == 0, invalid)?;
                output.push_str(&substitute(original, vault, |s| {
                    s.replace("]]>", "]]]]><![CDATA[>")
                })?);
            }
            Event::Decl(ref declaration) => {
                let encoding = declaration
                    .encoding()
                    .transpose()
                    .map_err(|_| ProxyBodyError::InvalidXml)?;
                when(
                    has_reference(original)?
                        || encoding.is_some_and(|encoding| !is_utf8_label(&encoding)),
                    invalid,
                )?;
                output.push_str(original);
            }
            Event::Eof => break,
            _ => {
                // Token references in comments, declarations, DTDs and processing
                // instructions are not card data positions.
                when(has_reference(original)?, invalid)?;
                output.push_str(original);
            }
        }
        position = next;
    }
    when(roots != 1 || depth != 0 || !valid_chars(&output), invalid)?;
    Ok(output)
}
