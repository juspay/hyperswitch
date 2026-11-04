//! Lossless body transport and format-aware interpolation for the vault proxy.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use self::{
    form::FormUrlEncodedBody,
    json::JsonBody,
    multipart::MultipartBody,
    placeholder::{substitute, Vault},
    xml::XmlBody,
};
use crate::fp_utils::when;

mod form;
mod json;
mod multipart;
pub mod placeholder;
pub mod response;
mod xml;

/// The connector body inside the JSON proxy envelope. Binary bodies are opaque.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ProxyBody {
    /// Serialized JSON template. Unchanged tokens retain their original bytes.
    Json(String),
    /// A complete, UTF-8 XML/SOAP document.
    Xml(String),
    /// An encoded form. Duplicate keys and ordering are significant.
    FormUrlEncoded(String),
    /// A UTF-8 text template.
    Text(String),
    /// Typed multipart parts; file contents are opaque base64.
    Multipart(Vec<ProxyMultipartPart>),
    /// Opaque base64-encoded bytes, with no token interpolation.
    RawBytes(String),
    /// An absent HTTP body, distinct from JSON null or an empty body.
    None,
}

impl hyperswitch_masking::SerializableSecret for ProxyBody {}

impl std::fmt::Debug for ProxyBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ProxyBody")
            .field(&self.as_metric_str())
            .finish()
    }
}

/// A multipart field or file. Metadata is never interpolated.
#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProxyMultipartPart {
    /// A text field whose value may reference vaulted data.
    Text {
        /// Form field name.
        name: String,
        /// UTF-8 value that may contain vault placeholders.
        value: String,
    },
    /// A binary file; bytes must survive the proxy without modification.
    File {
        /// Form field name.
        name: String,
        /// Original file name.
        filename: String,
        /// Content-Type of the opaque file bytes.
        mime_type: String,
        /// Standard base64-encoded file bytes.
        data_base64: String,
    },
}

impl std::fmt::Debug for ProxyMultipartPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Text { .. } => "ProxyMultipartPart::Text",
            Self::File { .. } => "ProxyMultipartPart::File",
        })
    }
}

/// Connector bytes and the effective Content-Type produced by a format, before they are
/// framed into a request.
struct RenderedBody {
    /// `None` means there must be no HTTP body.
    bytes: Option<Vec<u8>>,
    /// Original Content-Type, or a default derived from the explicit body type.
    content_type: Option<String>,
}

impl RenderedBody {
    /// Frame the bytes into a connector request. Framing headers are recomputed because
    /// interpolation changes the body length and the multipart boundary.
    fn into_request(
        self,
        url: &str,
        method: crate::request::Method,
        headers: Vec<(String, hyperswitch_masking::Maskable<String>)>,
    ) -> crate::request::Request {
        let mut headers: Vec<_> = headers
            .into_iter()
            .filter(|(name, _)| {
                !["content-type", "content-length", "transfer-encoding"]
                    .iter()
                    .any(|blocked| name.eq_ignore_ascii_case(blocked))
            })
            .collect();
        if let Some(content_type) = self.content_type {
            headers.push(("Content-Type".to_owned(), content_type.into()));
        }
        let mut builder = crate::request::RequestBuilder::new()
            .method(method)
            .url(url)
            .attach_default_headers()
            .headers(headers);
        if let Some(bytes) = self.bytes {
            builder = builder.set_body(crate::request::RequestContent::RawBytes(bytes));
        }
        builder.build()
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
    /// Invalid binary transport data.
    #[error("Invalid base64 body")]
    InvalidBase64,
    /// Invalid multipart metadata, or a boundary that occurs inside a part.
    #[error("Invalid multipart part or boundary")]
    InvalidMultipart,
    /// Form percent escapes must be valid and decode to UTF-8.
    #[error("Invalid URL-encoded template")]
    InvalidForm,
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
    /// The full MIME type with parameters; `None` when the header is not a valid MIME
    /// type, which only opaque bodies accept.
    mime: Option<mime::Mime>,
}

impl<'a> ConnectorContentType<'a> {
    /// Parse a Content-Type header value.
    pub fn parse(header: &'a str) -> Self {
        Self {
            header,
            media_type: media_type(header),
            mime: header.parse().ok(),
        }
    }

    // The single place that classifies a media type into a body format.

    /// Lower-cased media type, without parameters.
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// `application/json`, or any `+json` structured syntax suffix.
    pub fn is_json(&self) -> bool {
        self.media_type == "application/json" || self.media_type.ends_with("+json")
    }

    /// `application/xml`, `text/xml`, or any `+xml` structured syntax suffix.
    pub fn is_xml(&self) -> bool {
        matches!(self.media_type.as_str(), "application/xml" | "text/xml")
            || self.media_type.ends_with("+xml")
    }

    /// `application/x-www-form-urlencoded`.
    pub fn is_form_url_encoded(&self) -> bool {
        self.media_type == "application/x-www-form-urlencoded"
    }

    /// Any `text/*` media type other than `text/xml`.
    pub fn is_text(&self) -> bool {
        self.media_type.starts_with("text/") && self.media_type != "text/xml"
    }

    /// `multipart/form-data`.
    pub fn is_multipart_form_data(&self) -> bool {
        self.media_type == "multipart/form-data"
    }
}

/// Whether a charset or XML encoding label names UTF-8.
fn is_utf8_label(label: &[u8]) -> bool {
    label.eq_ignore_ascii_case(b"utf-8") || label.eq_ignore_ascii_case(b"utf8")
}

/// Decode standard base64 transport data.
fn decode_base64(encoded: &str) -> Result<Vec<u8>, ProxyBodyError> {
    STANDARD
        .decode(encoded)
        .map_err(|_| ProxyBodyError::InvalidBase64)
}

/// Lower-cased media type of a Content-Type header value, without parameters.
fn media_type(header: &str) -> String {
    header
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

/// Evaluate `$call` with `$format` bound to the [`ProxyBodyFormat`] implementation of
/// `$body`'s variant. Each arm is a static call on a concrete type, with no allocation.
macro_rules! with_format {
    ($body:expr, |$format:ident| $call:expr) => {
        match $body {
            ProxyBody::Json(template) => {
                let $format = JsonBody(template);
                $call
            }
            ProxyBody::Xml(template) => {
                let $format = XmlBody(template);
                $call
            }
            ProxyBody::FormUrlEncoded(template) => {
                let $format = FormUrlEncodedBody(template);
                $call
            }
            ProxyBody::Text(template) => {
                let $format = TextBody(template);
                $call
            }
            ProxyBody::Multipart(parts) => {
                let $format = MultipartBody(parts);
                $call
            }
            ProxyBody::RawBytes(encoded) => {
                let $format = RawBytesBody(encoded);
                $call
            }
            ProxyBody::None => {
                let $format = NoBody;
                $call
            }
        }
    };
}

impl ProxyBody {
    /// Stable, bounded body format for structured logs and metric dimensions.
    pub fn as_metric_str(&self) -> &'static str {
        with_format!(self, |format| format.name())
    }

    /// Populate the old request field for callers during a staged rollout. Only
    /// JSON is sent correctly by servers predating the typed body, so other
    /// formats travel once, in `body`, keeping the envelope within size limits.
    pub fn legacy_value(&self) -> Value {
        with_format!(self, |format| format.legacy_value())
    }

    /// Interpret the original API contract without changing JSON callers.
    pub fn from_legacy(
        body: Value,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<Self, ProxyBodyError> {
        let text = || {
            body.as_str()
                .map(str::to_owned)
                .ok_or(ProxyBodyError::InvalidContentType)
        };
        match content_type {
            // JSON, which is also the legacy default without a Content-Type.
            _ if content_type.is_none_or(ConnectorContentType::is_json) => {
                serde_json::to_string(&body)
                    .map(Self::Json)
                    .map_err(|_| ProxyBodyError::Serialization)
            }
            Some(declared) if declared.is_form_url_encoded() => text().map(Self::FormUrlEncoded),
            Some(declared) if declared.is_xml() => text().map(Self::Xml),
            Some(declared) if declared.is_text() => text().map(Self::Text),
            _ => Err(ProxyBodyError::InvalidContentType),
        }
    }

    /// Start turning this body into connector bytes. See [`ProxyBodyBuilder`].
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
    fn default_content_type(&self) -> Option<&'static str>;

    /// Whether a declared Content-Type can carry this format.
    fn accepts(&self, content_type: &ConnectorContentType<'_>) -> bool;

    /// Templates are UTF-8, so their declared charset must agree. Opaque bytes and
    /// absent bodies carry no template.
    fn is_utf8_template(&self) -> bool {
        true
    }

    /// Value for the pre-typed `request_body` field; see [`ProxyBody::legacy_value`].
    fn legacy_value(&self) -> Value {
        Value::Null
    }

    /// The declared Content-Type header, or this format's default when none was declared.
    fn content_type_header(&self, declared: Option<&ConnectorContentType<'_>>) -> Option<String> {
        declared
            .map(|declared| declared.header)
            .or_else(|| self.default_content_type())
            .map(str::to_owned)
    }

    /// Resolve vault placeholders and produce the connector bytes.
    fn render(
        &self,
        vault: Vault<'_>,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<RenderedBody, ProxyBodyError>;
}

struct TextBody<'a>(&'a str);

impl ProxyBodyFormat for TextBody<'_> {
    fn name(&self) -> &'static str {
        "text"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("text/plain; charset=utf-8")
    }

    fn accepts(&self, content_type: &ConnectorContentType<'_>) -> bool {
        content_type.is_text()
    }

    fn render(
        &self,
        vault: Vault<'_>,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<RenderedBody, ProxyBodyError> {
        Ok(RenderedBody {
            bytes: Some(substitute(self.0, vault, str::to_owned)?.into_bytes()),
            content_type: self.content_type_header(content_type),
        })
    }
}

struct RawBytesBody<'a>(&'a str);

impl ProxyBodyFormat for RawBytesBody<'_> {
    fn name(&self) -> &'static str {
        "raw_bytes"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("application/octet-stream")
    }

    fn accepts(&self, _content_type: &ConnectorContentType<'_>) -> bool {
        true
    }

    fn is_utf8_template(&self) -> bool {
        false
    }

    fn render(
        &self,
        _vault: Vault<'_>,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<RenderedBody, ProxyBodyError> {
        Ok(RenderedBody {
            bytes: Some(decode_base64(self.0)?),
            content_type: self.content_type_header(content_type),
        })
    }
}

struct NoBody;

impl ProxyBodyFormat for NoBody {
    fn name(&self) -> &'static str {
        "none"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        None
    }

    fn accepts(&self, _content_type: &ConnectorContentType<'_>) -> bool {
        true
    }

    fn is_utf8_template(&self) -> bool {
        false
    }

    fn render(
        &self,
        _vault: Vault<'_>,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<RenderedBody, ProxyBodyError> {
        Ok(RenderedBody {
            bytes: None,
            content_type: self.content_type_header(content_type),
        })
    }
}

/// Type-state builder that turns a [`ProxyBody`] into a connector request:
/// `Init` → [`Validated`] → prepared (a [`crate::request::Request`] ready to send).
/// Validation checks the Content-Type and the whole body without vault data, so an invalid
/// request never reaches the vault; only a validated body can be built.
///
/// ```ignore
/// let validated = body.builder().validate(content_type)?;
/// // ... read the vault only after validation succeeds ...
/// let request = validated.build(&vault_data, url, method, headers)?;
/// ```
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

impl<S: std::fmt::Debug> std::fmt::Debug for ProxyBodyBuilder<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyBodyBuilder")
            .field("format", &self.body.as_metric_str())
            .field("state", &self.state)
            .finish()
    }
}

impl<'a> ProxyBodyBuilder<'a, Init> {
    /// Check the declared Content-Type and the body without vault data: the format
    /// and charset, the body's structure, and placeholder syntax. Templates are UTF-8;
    /// reject a conflicting charset rather than sending bytes that the connector will
    /// decode using another charset.
    pub fn validate(
        self,
        content_type: Option<&'a ConnectorContentType<'a>>,
    ) -> Result<ProxyBodyBuilder<'a, Validated<'a>>, ProxyBodyError> {
        if let Some(declared) = content_type {
            when(
                !with_format!(self.body, |format| format.accepts(declared)),
                || Err(ProxyBodyError::InvalidContentType),
            )?;
            if with_format!(self.body, |format| format.is_utf8_template()) {
                let parsed = declared
                    .mime
                    .as_ref()
                    .ok_or(ProxyBodyError::InvalidContentType)?;
                when(
                    parsed.params().any(|(name, value)| {
                        name == mime::CHARSET && !is_utf8_label(value.as_str().as_bytes())
                    }),
                    || Err(ProxyBodyError::InvalidContentType),
                )?;
            }
        }
        with_format!(self.body, |format| format
            .render(Vault::Validation, content_type))?;
        Ok(ProxyBodyBuilder {
            body: self.body,
            state: Validated { content_type },
        })
    }
}

impl ProxyBodyBuilder<'_, Validated<'_>> {
    /// Resolve placeholders from vault data and build the connector request, ready to be
    /// sent: the body bytes are never a JSON wrapper string, and the declared Content-Type,
    /// or the format's default, replaces any caller framing headers.
    pub fn build(
        self,
        vault: &Value,
        url: &str,
        method: crate::request::Method,
        headers: Vec<(String, hyperswitch_masking::Maskable<String>)>,
    ) -> Result<crate::request::Request, ProxyBodyError> {
        let content_type = self.state.content_type;
        let rendered = with_format!(self.body, |format| format
            .render(Vault::Data(vault), content_type))?;
        Ok(rendered.into_request(url, method, headers))
    }
}
