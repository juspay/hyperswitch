//! Format-aware connector body interpolation for the vault proxy.

use common_utils::request::{
    ConnectorContentType, ContentType, Method, Request, RequestBuilder, RequestContent,
};
use serde_json::Value;

mod json;
mod placeholder;
mod xml;

/// The connector body of a proxy request.
#[derive(Debug)]
pub enum ProxyBody {
    /// A JSON body.
    Json(Value),
    /// A complete, UTF-8 XML/SOAP document.
    Xml(String),
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
    /// The Content-Type is neither JSON nor XML.
    #[error("Unsupported connector Content-Type; only JSON and XML are supported")]
    UnsupportedContentType,
    /// An XML Content-Type with a non-UTF-8 charset, or with a body that is not a string.
    #[error("Connector body does not match its Content-Type")]
    InvalidContentType,
    /// An XML document or its template placement is invalid.
    #[error("Invalid XML template")]
    InvalidXml,
    /// Serialization failed.
    #[error("Failed to serialize connector body")]
    Serialization,
}

impl ProxyBody {
    /// The connector body for the declared Content-Type: JSON, also when none is declared,
    /// or a UTF-8 XML document sent as a string. Any other Content-Type is rejected.
    pub fn new(
        request_body: Value,
        content_type: Option<&ConnectorContentType<'_>>,
    ) -> Result<Self, ProxyBodyError> {
        let kind = content_type.map_or(Some(ContentType::Json), ConnectorContentType::kind);
        match (kind, request_body) {
            (Some(ContentType::Json), body) => Ok(Self::Json(body)),
            (Some(ContentType::Xml), Value::String(xml))
                if content_type.is_some_and(ConnectorContentType::is_utf8) =>
            {
                Ok(Self::Xml(xml))
            }
            (Some(ContentType::Xml), _) => Err(ProxyBodyError::InvalidContentType),
            _ => Err(ProxyBodyError::UnsupportedContentType),
        }
    }

    /// Content-Type sent when the caller does not declare one.
    fn default_content_type(&self) -> &'static str {
        match self {
            Self::Json(_) => "application/json",
            Self::Xml(_) => "application/xml",
        }
    }

    /// Check the body's structure and placeholder syntax, without vault data.
    fn validate(&self) -> Result<(), ProxyBodyError> {
        match self {
            Self::Json(body) => json::validate(body),
            Self::Xml(template) => xml::validate(template),
        }
    }

    /// Replace placeholders with vault values and produce the connector bytes.
    fn render(&self, vault: &Value) -> Result<Vec<u8>, ProxyBodyError> {
        match self {
            Self::Json(body) => json::render(body, vault),
            Self::Xml(template) => xml::render(template, vault),
        }
    }

    /// Start turning this body into a connector request. See [`ProxyBodyBuilder`].
    pub fn builder(&self) -> ProxyBodyBuilder<'_, Init> {
        ProxyBodyBuilder {
            body: self,
            state: Init,
        }
    }
}

/// Type-state builder that turns a [`ProxyBody`] into a connector request:
/// [`Init`] → [`Validated`] → [`Prepared`]. Validation checks the whole body without vault
/// data, so an invalid request never reaches the vault.
#[derive(Debug)]
pub struct ProxyBodyBuilder<'a, S> {
    body: &'a ProxyBody,
    state: S,
}

/// Builder state before validation.
#[derive(Debug)]
pub struct Init;

/// Builder state after the body passed validation.
#[derive(Debug)]
pub struct Validated<'a> {
    content_type: Option<&'a ConnectorContentType<'a>>,
}

/// Builder state holding the prepared request, ready to be sent to the connector.
#[derive(Debug)]
pub struct Prepared(Request);

impl<'a> ProxyBodyBuilder<'a, Init> {
    /// Check the body without vault data: its structure and placeholder syntax.
    pub fn validate(
        self,
        content_type: Option<&'a ConnectorContentType<'a>>,
    ) -> Result<ProxyBodyBuilder<'a, Validated<'a>>, ProxyBodyError> {
        self.body.validate()?;
        Ok(ProxyBodyBuilder {
            body: self.body,
            state: Validated { content_type },
        })
    }
}

impl<'a> ProxyBodyBuilder<'a, Validated<'a>> {
    /// Resolve placeholders from vault data and build the connector request. The caller's
    /// Content-Type, Content-Length and Transfer-Encoding headers are dropped, as interpolation
    /// changes the body length; the declared Content-Type, or the body's default, is set again.
    pub fn build(
        self,
        vault: &Value,
        url: &str,
        method: Method,
        headers: Vec<(String, hyperswitch_masking::Maskable<String>)>,
    ) -> Result<ProxyBodyBuilder<'a, Prepared>, ProxyBodyError> {
        let bytes = self.body.render(vault)?;
        let content_type = self.state.content_type.map_or(
            self.body.default_content_type(),
            ConnectorContentType::header,
        );
        let headers = headers
            .into_iter()
            .filter(|(name, _)| {
                !["content-type", "content-length", "transfer-encoding"]
                    .iter()
                    .any(|blocked| name.eq_ignore_ascii_case(blocked))
            })
            .chain([("Content-Type".to_owned(), content_type.to_owned().into())])
            .collect();
        let request = RequestBuilder::new()
            .method(method)
            .url(url)
            .attach_default_headers()
            .headers(headers)
            .set_body(RequestContent::RawBytes(bytes))
            .build();
        Ok(ProxyBodyBuilder {
            body: self.body,
            state: Prepared(request),
        })
    }
}

impl ProxyBodyBuilder<'_, Prepared> {
    /// The prepared request, to be sent to the connector.
    pub fn into_inner(self) -> Request {
        self.state.0
    }
}
