//! Format-aware body interpolation for the vault proxy.

use serde_json::Value;

use self::{json::JsonBody, placeholder::Vault, xml::XmlBody};
use crate::{
    fp_utils::when,
    macros::with_format,
    request::{ContentType, Method, Request, RequestBuilder, RequestContent},
};

mod json;
mod placeholder;
pub mod response;
mod xml;

/// The connector body of a proxy request.
pub enum ProxyBody {
    /// A JSON body.
    Json(Value),
    /// A complete, UTF-8 XML/SOAP document.
    Xml(String),
    /// No HTTP body.
    None,
}

impl std::fmt::Debug for ProxyBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ProxyBody")
            .field(&self.as_metric_str())
            .finish()
    }
}

/// Errors omit template and resolved field contents.
#[derive(Debug, thiserror::Error)]
pub enum ProxyBodyError {
    /// A placeholder is syntactically invalid.
    #[error("Malformed vault placeholder")]
    InvalidPlaceholder,
    /// The referenced field does not exist or cannot be used as a scalar.
    #[error("Missing or non-scalar vault field: {0}")]
    InvalidField(String),
    /// The body and Content-Type cannot be interpreted safely.
    #[error("Connector body does not match its Content-Type")]
    InvalidContentType,
    /// An XML document or its template placement is invalid.
    #[error("Invalid XML template")]
    InvalidXml,
    /// Serialization failed.
    #[error("Failed to serialize connector body")]
    Serialization,
}

/// A connector Content-Type header, parsed once and passed to every step that needs it.
#[derive(Debug)]
pub struct ConnectorContentType<'a> {
    header: &'a str,
    /// Lower-cased media type, without parameters.
    media_type: String,
    /// The full MIME type with parameters; `None` when the header is not a valid MIME type.
    mime: Option<mime::Mime>,
}

impl<'a> ConnectorContentType<'a> {
    /// Parse a Content-Type header value.
    pub fn parse(header: &'a str) -> Self {
        Self {
            header,
            media_type: header
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase(),
            mime: header.parse().ok(),
        }
    }

    /// The body format this header declares, for the formats the proxy handles.
    pub fn kind(&self) -> Option<ContentType> {
        match self.media_type.as_str() {
            "application/json" => Some(ContentType::Json),
            "application/xml" | "text/xml" => Some(ContentType::Xml),
            kind if kind.ends_with("+json") => Some(ContentType::Json),
            kind if kind.ends_with("+xml") => Some(ContentType::Xml),
            _ => None,
        }
    }

    /// A valid MIME type whose charset, if declared, is UTF-8. Templates are UTF-8, so a
    /// conflicting declaration would make the connector decode the bytes incorrectly.
    fn is_utf8(&self) -> bool {
        self.mime.as_ref().is_some_and(|parsed| {
            parsed.params().all(|(name, value)| {
                name != mime::CHARSET || is_utf8_label(value.as_str().as_bytes())
            })
        })
    }
}

/// Whether a charset or XML encoding label names UTF-8.
fn is_utf8_label(label: &[u8]) -> bool {
    label.eq_ignore_ascii_case(b"utf-8") || label.eq_ignore_ascii_case(b"utf8")
}

impl ProxyBody {
    /// The connector body of a proxy request. GET and DELETE stay bodyless; an XML document
    /// travels as a string declared by an XML Content-Type; anything else is JSON.
    pub fn new(
        request_body: Value,
        method: Method,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Self {
        let is_xml = matches!(
            content_type.and_then(ConnectorContentType::kind),
            Some(ContentType::Xml)
        );
        match (method, request_body) {
            (Method::Get | Method::Delete, _) => Self::None,
            (_, Value::String(xml)) if is_xml => Self::Xml(xml),
            (_, body) => Self::Json(body),
        }
    }

    /// Stable, bounded body format for structured logs and metric dimensions.
    pub fn as_metric_str(&self) -> &'static str {
        with_format!(self, |format| format.name())
    }

    /// Start turning this body into a connector request. See [`ProxyBodyBuilder`].
    pub fn builder(&self) -> ProxyBodyBuilder<'_, Init> {
        ProxyBodyBuilder {
            body: self,
            state: Init,
        }
    }
}

/// Behavior of one connector body format. Each [`ProxyBody`] variant has its own
/// implementation, so supporting a new format means adding one implementation.
trait ProxyBodyFormat {
    /// Stable, bounded label for structured logs and metric dimensions.
    fn name(&self) -> &'static str;

    /// Content-Type sent when the caller does not declare one.
    fn default_content_type(&self) -> Option<&'static str> {
        None
    }

    /// Whether a declared Content-Type can carry this format. Any Content-Type is accepted
    /// unless the format requires a specific one.
    fn accepts(&self, _content_type: &ConnectorContentType<'_>) -> bool {
        true
    }

    /// The declared Content-Type header, or this format's default when none was declared.
    fn content_type_header(&self, declared: Option<&ConnectorContentType<'_>>) -> Option<String> {
        declared
            .map(|declared| declared.header)
            .or_else(|| self.default_content_type())
            .map(str::to_owned)
    }

    /// Resolve vault placeholders into the connector bytes; `None` means no HTTP body.
    fn render(&self, vault: Vault<'_>) -> Result<Option<Vec<u8>>, ProxyBodyError>;
}

struct NoBody;

impl ProxyBodyFormat for NoBody {
    fn name(&self) -> &'static str {
        "none"
    }

    fn render(&self, _vault: Vault<'_>) -> Result<Option<Vec<u8>>, ProxyBodyError> {
        Ok(None)
    }
}

/// Type-state builder that turns a [`ProxyBody`] into a connector request:
/// [`Init`] → [`Validated`] → [`Prepared`]. Validation checks the Content-Type and the whole
/// body without vault data, so an invalid request never reaches the vault.
pub struct ProxyBodyBuilder<'a, S> {
    body: &'a ProxyBody,
    state: S,
}

/// Builder state before validation.
#[derive(Debug)]
pub struct Init;

/// Builder state after the Content-Type and body passed validation.
#[derive(Debug)]
pub struct Validated<'a> {
    content_type: Option<&'a ConnectorContentType<'a>>,
}

/// Builder state holding the prepared value, ready to be sent to the connector.
#[derive(Debug)]
pub struct Prepared<T>(T);

impl<S: std::fmt::Debug> std::fmt::Debug for ProxyBodyBuilder<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyBodyBuilder")
            .field("format", &self.body.as_metric_str())
            .field("state", &self.state)
            .finish()
    }
}

impl<'a> ProxyBodyBuilder<'a, Init> {
    /// Check the declared Content-Type and the body without vault data: the format, the
    /// body's structure, and placeholder syntax.
    pub fn validate(
        self,
        content_type: Option<&'a ConnectorContentType<'a>>,
    ) -> Result<ProxyBodyBuilder<'a, Validated<'a>>, ProxyBodyError> {
        let accepted = content_type
            .is_none_or(|declared| with_format!(self.body, |format| format.accepts(declared)));
        when(!accepted, || Err(ProxyBodyError::InvalidContentType))?;
        with_format!(self.body, |format| format.render(Vault::Validation))?;
        Ok(ProxyBodyBuilder {
            body: self.body,
            state: Validated { content_type },
        })
    }
}

impl<'a> ProxyBodyBuilder<'a, Validated<'a>> {
    /// Resolve placeholders from vault data and build the connector request. The declared
    /// Content-Type, or the format's default, replaces any caller framing headers, which
    /// interpolation invalidates by changing the body length.
    pub fn build(
        self,
        vault: &Value,
        url: &str,
        method: Method,
        headers: Vec<(String, hyperswitch_masking::Maskable<String>)>,
    ) -> Result<ProxyBodyBuilder<'a, Prepared<Request>>, ProxyBodyError> {
        let (bytes, content_type) = with_format!(self.body, |format| (
            format.render(Vault::Data(vault))?,
            format.content_type_header(self.state.content_type)
        ));
        let headers = headers
            .into_iter()
            .filter(|(name, _)| {
                !["content-type", "content-length", "transfer-encoding"]
                    .iter()
                    .any(|blocked| name.eq_ignore_ascii_case(blocked))
            })
            .chain(
                content_type.map(|content_type| ("Content-Type".to_owned(), content_type.into())),
            )
            .collect();
        let request = RequestBuilder::new()
            .method(method)
            .url(url)
            .attach_default_headers()
            .headers(headers)
            .set_optional_body(bytes.map(RequestContent::RawBytes))
            .build();
        Ok(ProxyBodyBuilder {
            body: self.body,
            state: Prepared(request),
        })
    }
}

impl<T> ProxyBodyBuilder<'_, Prepared<T>> {
    /// The prepared value, to be sent to the connector.
    pub fn into_inner(self) -> T {
        self.state.0
    }
}
