//! XML and SOAP bodies: `quick_xml` reads and writes the document, and placeholders are
//! resolved in text, attribute values and CDATA.

use common_utils::{fp_utils::when, request::is_utf8_label};
use quick_xml::{
    escape::escape,
    events::{BytesCData, BytesStart, BytesText, Event},
    Reader, Writer,
};
use serde_json::Value;

use super::{
    placeholder::{has_reference, substitute, validate_placeholders},
    ProxyBodyError,
};

/// Whether an element or attribute name contains a placeholder marker.
fn has_marker(name: &[u8]) -> bool {
    name.windows(2).any(|pair| pair == b"{{")
}

/// Characters allowed in an XML 1.0 document.
fn valid_chars(value: &str) -> bool {
    value.chars().all(|c| {
        matches!(c, '\t' | '\r' | '\n')
            || ('\u{20}'..='\u{d7ff}').contains(&c)
            || ('\u{e000}'..='\u{fffd}').contains(&c)
            || ('\u{10000}'..='\u{10ffff}').contains(&c)
    })
}

/// The raw bytes of an event as text.
fn as_text(bytes: &[u8]) -> Result<&str, ProxyBodyError> {
    std::str::from_utf8(bytes).map_err(|_| ProxyBodyError::InvalidXml)
}

/// Check the document structure, placeholder syntax and placeholder positions.
pub(super) fn validate(template: &str) -> Result<(), ProxyBodyError> {
    let invalid = || Err(ProxyBodyError::InvalidXml);
    when(!valid_chars(template), invalid)?;
    validate_placeholders(template)?;
    let mut reader = Reader::from_str(template);
    let (mut depth, mut roots) = (0_usize, 0_usize);
    loop {
        let event = reader
            .read_event()
            .map_err(|_| ProxyBodyError::InvalidXml)?;
        let misplaced = match &event {
            Event::Eof => break,
            Event::Start(element) | Event::Empty(element) => {
                roots += usize::from(depth == 0);
                depth += usize::from(matches!(event, Event::Start(_)));
                has_marker(element.name().as_ref())
                    || element
                        .attributes()
                        .any(|attr| attr.map_or(true, |attr| has_marker(attr.key.as_ref())))
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(ProxyBodyError::InvalidXml)?;
                false
            }
            Event::Text(text) => depth == 0 && !text.iter().all(u8::is_ascii_whitespace),
            Event::CData(_) => depth == 0,
            Event::Decl(declaration) => {
                has_reference(as_text(declaration)?)?
                    || declaration.encoding().is_some_and(|encoding| {
                        encoding.map_or(true, |encoding| !is_utf8_label(&encoding))
                    })
            }
            // Token references in comments, DTDs and processing instructions are not card
            // data positions.
            other => has_reference(as_text(other)?)?,
        };
        when(misplaced, invalid)?;
    }
    when(roots != 1 || depth != 0, invalid)
}

/// Replace placeholders with escaped vault values. The template has passed [`validate`],
/// so placeholders appear only in text, attribute values and CDATA.
pub(super) fn render(template: &str, vault: &Value) -> Result<Vec<u8>, ProxyBodyError> {
    let fill = |raw: &str| substitute(raw, vault, |value| escape(value).into_owned());
    let mut reader = Reader::from_str(template);
    let mut writer = Writer::new(Vec::with_capacity(template.len()));
    loop {
        let event = match reader
            .read_event()
            .map_err(|_| ProxyBodyError::InvalidXml)?
        {
            Event::Eof => break,
            Event::Start(element) => Event::Start(BytesStart::from_content(
                fill(as_text(&element)?)?,
                element.name().as_ref().len(),
            )),
            Event::Empty(element) => Event::Empty(BytesStart::from_content(
                fill(as_text(&element)?)?,
                element.name().as_ref().len(),
            )),
            Event::Text(text) => Event::Text(BytesText::from_escaped(fill(as_text(&text)?)?)),
            // A CDATA section cannot contain `]]>`, so a value holding it is split across two.
            Event::CData(data) => Event::CData(BytesCData::new(substitute(
                as_text(&data)?,
                vault,
                |value| value.replace("]]>", "]]]]><![CDATA[>"),
            )?)),
            other => other,
        };
        writer
            .write_event(event)
            .map_err(|_| ProxyBodyError::InvalidXml)?;
    }
    let output = writer.into_inner();
    when(!valid_chars(as_text(&output)?), || {
        Err(ProxyBodyError::InvalidXml)
    })?;
    Ok(output)
}
