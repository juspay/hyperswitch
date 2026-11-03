//! Lossless body transport and format-aware interpolation for the vault proxy.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use self::{
    form::FormUrlEncodedBody, json::JsonBody, multipart::MultipartBody, placeholder::substitute,
    xml::XmlBody,
};

mod form;
mod json;
mod multipart;
pub mod placeholder;
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
        f.debug_tuple("ProxyMultipartPart")
            .field(&self.format().kind())
            .finish()
    }
}

/// Prepared bytes and the effective connector Content-Type.
pub struct PreparedProxyBody {
    /// `None` means there must be no HTTP body.
    pub bytes: Option<Vec<u8>>,
    /// Original Content-Type, or a default derived from the explicit body type.
    pub content_type: Option<String>,
}

impl std::fmt::Debug for PreparedProxyBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedProxyBody")
            .field("body_length", &self.bytes.as_ref().map(Vec::len))
            .field("content_type", &self.content_type)
            .finish()
    }
}

impl PreparedProxyBody {
    /// Build the connector request from resolved bytes. Recompute framing headers
    /// because interpolation changes the body length and multipart boundary.
    pub fn into_request(
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

/// Lower-cased media type of a Content-Type header value, without parameters.
pub fn media_type(header: &str) -> String {
    header
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

impl ProxyBody {
    /// Stable, bounded body format for structured logs and metric dimensions.
    pub fn as_metric_str(&self) -> &'static str {
        self.format().name()
    }

    /// Populate the old request field for callers during a staged rollout. Only
    /// JSON is sent correctly by servers predating the typed body, so other
    /// formats travel once, in `body`, keeping the envelope within size limits.
    pub fn legacy_value(&self) -> Value {
        self.format().legacy_value()
    }

    /// Interpret the original API contract without changing JSON callers.
    pub fn from_legacy(body: Value, content_type: Option<&str>) -> Result<Self, ProxyBodyError> {
        let kind = content_type
            .map(media_type)
            .unwrap_or_else(|| "application/json".into());
        if kind == "application/json" || kind.ends_with("+json") {
            return serde_json::to_string(&body)
                .map(Self::Json)
                .map_err(|_| ProxyBodyError::Serialization);
        }
        let text = body
            .as_str()
            .ok_or(ProxyBodyError::InvalidContentType)?
            .to_owned();
        match kind.as_str() {
            "application/x-www-form-urlencoded" => Ok(Self::FormUrlEncoded(text)),
            "text/xml" | "application/xml" => Ok(Self::Xml(text)),
            _ if kind.ends_with("+xml") => Ok(Self::Xml(text)),
            _ if kind.starts_with("text/") => Ok(Self::Text(text)),
            _ => Err(ProxyBodyError::InvalidContentType),
        }
    }

    /// Start turning this body into connector bytes. See [`ProxyBodyBuilder`].
    pub fn builder(&self) -> ProxyBodyBuilder<'_, NeedsContentType> {
        ProxyBodyBuilder {
            format: self.format(),
            state: NeedsContentType,
        }
    }

    /// The only place that maps a variant to its format implementation.
    fn format(&self) -> Box<dyn ProxyBodyFormat + '_> {
        match self {
            Self::Json(template) => Box::new(JsonBody(template)),
            Self::Xml(template) => Box::new(XmlBody(template)),
            Self::FormUrlEncoded(template) => Box::new(FormUrlEncodedBody(template)),
            Self::Text(template) => Box::new(TextBody(template)),
            Self::Multipart(parts) => Box::new(MultipartBody(parts)),
            Self::RawBytes(encoded) => Box::new(RawBytesBody(encoded)),
            Self::None => Box::new(NoBody),
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

    /// Whether a declared, lower-cased media type can carry this format.
    fn accepts(&self, media_type: &str) -> bool;

    /// Templates are UTF-8, so their declared charset must agree. Opaque bytes and
    /// absent bodies carry no template.
    fn is_utf8_template(&self) -> bool {
        true
    }

    /// Value for the pre-typed `request_body` field; see [`ProxyBody::legacy_value`].
    fn legacy_value(&self) -> Value {
        Value::Null
    }

    /// Resolve vault placeholders and produce the connector bytes.
    fn render(
        &self,
        vault: &Value,
        content_type: &CheckedContentType<'_>,
    ) -> Result<PreparedProxyBody, ProxyBodyError>;
}

/// A declared Content-Type that is compatible with the body format.
#[derive(Debug)]
struct CheckedContentType<'a> {
    header: Option<&'a str>,
    /// Parsed only for UTF-8 templates, whose parameters are significant.
    parsed: Option<mime::Mime>,
}

impl CheckedContentType<'_> {
    /// The declared header, or the format's default when none was declared.
    fn or_default(&self, default: Option<&str>) -> Option<String> {
        self.header.or(default).map(str::to_owned)
    }
}

struct TextBody<'a>(&'a str);

impl ProxyBodyFormat for TextBody<'_> {
    fn name(&self) -> &'static str {
        "text"
    }

    fn default_content_type(&self) -> Option<&'static str> {
        Some("text/plain; charset=utf-8")
    }

    fn accepts(&self, media_type: &str) -> bool {
        media_type.starts_with("text/") && media_type != "text/xml"
    }

    fn render(
        &self,
        vault: &Value,
        content_type: &CheckedContentType<'_>,
    ) -> Result<PreparedProxyBody, ProxyBodyError> {
        Ok(PreparedProxyBody {
            bytes: Some(substitute(self.0, vault, str::to_owned)?.into_bytes()),
            content_type: content_type.or_default(self.default_content_type()),
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

    fn accepts(&self, _media_type: &str) -> bool {
        true
    }

    fn is_utf8_template(&self) -> bool {
        false
    }

    fn render(
        &self,
        _vault: &Value,
        content_type: &CheckedContentType<'_>,
    ) -> Result<PreparedProxyBody, ProxyBodyError> {
        Ok(PreparedProxyBody {
            bytes: Some(
                STANDARD
                    .decode(self.0)
                    .map_err(|_| ProxyBodyError::InvalidBase64)?,
            ),
            content_type: content_type.or_default(self.default_content_type()),
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

    fn accepts(&self, _media_type: &str) -> bool {
        true
    }

    fn is_utf8_template(&self) -> bool {
        false
    }

    fn render(
        &self,
        _vault: &Value,
        content_type: &CheckedContentType<'_>,
    ) -> Result<PreparedProxyBody, ProxyBodyError> {
        Ok(PreparedProxyBody {
            bytes: None,
            content_type: content_type.or_default(self.default_content_type()),
        })
    }
}

/// Type-state builder that turns a [`ProxyBody`] into connector bytes. The declared
/// Content-Type is checked first, vault data is supplied next, and only then can the
/// body be built; each step returns a new state, so a skipped step does not compile.
///
/// ```ignore
/// let prepared = body
///     .builder()
///     .content_type(content_type)?
///     .vault_data(&vault_data)
///     .build()?;
/// ```
pub struct ProxyBodyBuilder<'a, S> {
    format: Box<dyn ProxyBodyFormat + 'a>,
    state: S,
}

/// Builder state before the declared Content-Type is checked.
#[derive(Debug)]
pub struct NeedsContentType;

/// Builder state after the Content-Type check, waiting for vault data.
#[derive(Debug)]
pub struct NeedsVaultData<'a> {
    content_type: CheckedContentType<'a>,
}

/// Builder state with every input in place.
pub struct ReadyToBuild<'a, 'v> {
    content_type: CheckedContentType<'a>,
    vault: &'v Value,
}

impl std::fmt::Debug for ReadyToBuild<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Vault data is never printed.
        f.debug_struct("ReadyToBuild")
            .field("content_type", &self.content_type)
            .finish_non_exhaustive()
    }
}

impl<S: std::fmt::Debug> std::fmt::Debug for ProxyBodyBuilder<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyBodyBuilder")
            .field("format", &self.format.name())
            .field("state", &self.state)
            .finish()
    }
}

impl<'a> ProxyBodyBuilder<'a, NeedsContentType> {
    /// Check the declared Content-Type against the body format. Templates are
    /// UTF-8; reject a conflicting charset rather than sending bytes that the
    /// connector will decode using another charset.
    pub fn content_type(
        self,
        header: Option<&'a str>,
    ) -> Result<ProxyBodyBuilder<'a, NeedsVaultData<'a>>, ProxyBodyError> {
        if header
            .map(media_type)
            .is_some_and(|kind| !self.format.accepts(&kind))
        {
            return Err(ProxyBodyError::InvalidContentType);
        }
        let parsed = header
            .filter(|_| self.format.is_utf8_template())
            .map(str::parse::<mime::Mime>)
            .transpose()
            .map_err(|_| ProxyBodyError::InvalidContentType)?;
        if parsed.as_ref().is_some_and(|parsed| {
            parsed.params().any(|(name, value)| {
                name == mime::CHARSET
                    && !value.as_str().eq_ignore_ascii_case("utf-8")
                    && !value.as_str().eq_ignore_ascii_case("utf8")
            })
        }) {
            return Err(ProxyBodyError::InvalidContentType);
        }
        Ok(ProxyBodyBuilder {
            format: self.format,
            state: NeedsVaultData {
                content_type: CheckedContentType { header, parsed },
            },
        })
    }
}

impl<'a> ProxyBodyBuilder<'a, NeedsVaultData<'a>> {
    /// Supply the vault data that placeholders resolve against.
    pub fn vault_data(self, vault: &Value) -> ProxyBodyBuilder<'a, ReadyToBuild<'a, '_>> {
        ProxyBodyBuilder {
            format: self.format,
            state: ReadyToBuild {
                content_type: self.state.content_type,
                vault,
            },
        }
    }
}

impl ProxyBodyBuilder<'_, ReadyToBuild<'_, '_>> {
    /// Resolve the template and produce connector bytes, never a JSON wrapper string.
    pub fn build(self) -> Result<PreparedProxyBody, ProxyBodyError> {
        self.format
            .render(self.state.vault, &self.state.content_type)
    }
}
