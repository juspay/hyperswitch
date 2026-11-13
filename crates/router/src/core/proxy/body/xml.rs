//! XML and SOAP bodies: `quick_xml` reads and writes the document, and placeholders are
//! resolved in text, attribute values and CDATA.

use common_utils::{fp_utils::when, request::is_utf8_label};
use quick_xml::{
    escape::escape,
    events::{BytesCData, BytesStart, BytesText, Event},
    Reader, Writer,
};
use roxmltree::{Document, ParsingOptions};
use serde_json::Value;

use super::{
    placeholder::{has_reference, substitute, validate_placeholders},
    ProxyBodyError,
};

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

/// Check that the template is well-formed UTF-8 XML whose placeholders are valid and sit in
/// text or attribute values only. Being well-formed keeps them out of element and attribute
/// names, as `{` is not allowed in a name.
pub(super) fn validate(template: &str) -> Result<(), ProxyBodyError> {
    let invalid = || Err(ProxyBodyError::InvalidXml);
    validate_placeholders(template)?;
    let options = ParsingOptions {
        allow_dtd: true,
        nodes_limit: 10_000,
    };
    let document =
        Document::parse_with_options(template, options).map_err(|_| ProxyBodyError::InvalidXml)?;
    // Token references in the declaration and DTD (before the root element), comments and
    // processing instructions are not card data positions.
    let prolog = template
        .get(..document.root_element().range().start)
        .unwrap_or_default();
    let comments = document.descendants().filter_map(|node| {
        if node.is_comment() {
            node.text()
        } else {
            node.pi().and_then(|pi| pi.value)
        }
    });
    std::iter::once(prolog)
        .chain(comments)
        .try_for_each(|text| when(has_reference(text)?, invalid))?;
    when(!declares_utf8(template), invalid)
}

/// Whether the XML declaration, if any, names UTF-8, the encoding the template is sent in.
/// `roxmltree` does not check the declared encoding.
fn declares_utf8(template: &str) -> bool {
    match Reader::from_str(template).read_event() {
        Ok(Event::Decl(declaration)) => declaration
            .encoding()
            .is_none_or(|encoding| encoding.is_ok_and(|encoding| is_utf8_label(&encoding))),
        _ => true,
    }
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
