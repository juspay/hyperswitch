//! XML and SOAP bodies: placeholders are resolved in text, attribute values and CDATA,
//! escaped for each position.

use quick_xml::{events::Event, Reader};

use super::{
    is_utf8_label,
    placeholder::{has_reference, substitute, Vault},
    ConnectorContentType, ProxyBodyError, ProxyBodyFormat,
};
use crate::{fp_utils::when, request::ContentType};

pub(super) struct XmlBody<'a>(pub(super) &'a str);

impl ProxyBodyFormat for XmlBody<'_> {
    fn name(&self) -> &'static str {
        "xml"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("application/xml")
    }

    fn accepts(&self, content_type: &ConnectorContentType<'_>) -> bool {
        matches!(content_type.kind(), Some(ContentType::Xml)) && content_type.is_utf8()
    }

    fn render(&self, vault: Vault<'_>) -> Result<Option<Vec<u8>>, ProxyBodyError> {
        resolve_xml(self.0, vault).map(|xml| Some(xml.into_bytes()))
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
    let (output, depth, roots) = events(template).try_fold(
        (String::with_capacity(template.len()), 0_usize, 0_usize),
        |(output, depth, roots), event| {
            let (event, original) = event?;
            let (piece, depth, roots) = match event {
                Event::Start(ref element) | Event::Empty(ref element) => {
                    when(has_marker(element.name().as_ref()), invalid)?;
                    for attr in element.attributes() {
                        let attr = attr.map_err(|_| ProxyBodyError::InvalidXml)?;
                        when(has_marker(attr.key.as_ref()), invalid)?;
                    }
                    (
                        substitute(original, vault, xml_escape)?,
                        depth + usize::from(matches!(event, Event::Start(_))),
                        roots + usize::from(depth == 0),
                    )
                }
                Event::End(_) => (
                    original.to_owned(),
                    depth.checked_sub(1).ok_or(ProxyBodyError::InvalidXml)?,
                    roots,
                ),
                Event::Text(_) => {
                    when(depth == 0 && !original.trim().is_empty(), invalid)?;
                    (substitute(original, vault, xml_escape)?, depth, roots)
                }
                Event::CData(_) => {
                    when(depth == 0, invalid)?;
                    let piece =
                        substitute(original, vault, |s| s.replace("]]>", "]]]]><![CDATA[>"))?;
                    (piece, depth, roots)
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
                    (original.to_owned(), depth, roots)
                }
                _ => {
                    // Token references in comments, declarations, DTDs and processing
                    // instructions are not card data positions.
                    when(has_reference(original)?, invalid)?;
                    (original.to_owned(), depth, roots)
                }
            };
            Ok::<_, ProxyBodyError>((output + &piece, depth, roots))
        },
    )?;
    when(roots != 1 || depth != 0 || !valid_chars(&output), invalid)?;
    Ok(output)
}

/// Each XML event with the exact source text it was read from, up to the end of the document.
fn events(template: &str) -> impl Iterator<Item = Result<(Event<'_>, &str), ProxyBodyError>> {
    std::iter::repeat(()).scan(
        (Reader::from_str(template), 0),
        move |(reader, position), ()| match reader.read_event() {
            Ok(Event::Eof) => None,
            Ok(event) => {
                let next = reader.buffer_position();
                let original = template.get(*position..next);
                *position = next;
                Some(
                    original
                        .map(|original| (event, original))
                        .ok_or(ProxyBodyError::InvalidXml),
                )
            }
            Err(_) => Some(Err(ProxyBodyError::InvalidXml)),
        },
    )
}
