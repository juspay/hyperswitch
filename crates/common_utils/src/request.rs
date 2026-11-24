use hyperswitch_masking::{Maskable, Secret};
use reqwest::multipart::Form;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Default XML version
pub const DEFAULT_XML_VERSION: &str = "1.0";
/// Default XML encoding
pub const DEFAULT_XML_ENCODING: &str = "UTF-8";

/// Configuration for XML serialization
#[derive(Debug, Clone)]
pub struct XmlConfig {
    /// XML version (e.g., "1.0")
    pub xml_version: String,
    /// XML encoding (e.g., "UTF-8")
    pub xml_encoding: Option<String>,
    /// XML standalone declaration
    pub xml_standalone: Option<String>,
    /// XML document type declaration
    pub xml_doc_type: Option<String>,
}

impl Default for XmlConfig {
    fn default() -> Self {
        Self {
            xml_version: DEFAULT_XML_VERSION.to_string(),
            xml_encoding: Some(DEFAULT_XML_ENCODING.to_string()),
            xml_standalone: None,
            xml_doc_type: None,
        }
    }
}

pub type Headers = std::collections::HashSet<(String, Maskable<String>)>;

#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Deserialize,
    Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ContentType {
    Json,
    FormUrlEncoded,
    FormData,
    Xml,
}

/// A connector Content-Type header, parsed once and passed to every step that needs it.
#[derive(Debug)]
pub struct ConnectorContentType<'a> {
    header: &'a str,
    mime: mime::Mime,
}

impl<'a> ConnectorContentType<'a> {
    /// Parse a Content-Type header value; `None` when it is not a valid MIME type.
    pub fn parse(header: &'a str) -> Option<Self> {
        header
            .parse::<mime::Mime>()
            .ok()
            .map(|mime| Self { header, mime })
    }

    /// The body format this header declares, for the formats the proxy handles.
    pub fn kind(&self) -> Option<ContentType> {
        match self.mime.essence_str() {
            "application/json" => Some(ContentType::Json),
            "application/xml" | "text/xml" => Some(ContentType::Xml),
            kind if kind.ends_with("+json") => Some(ContentType::Json),
            kind if kind.ends_with("+xml") => Some(ContentType::Xml),
            _ => None,
        }
    }

    /// The header value as received.
    pub fn header(&self) -> &'a str {
        self.header
    }

    /// Whether the charset, if declared, is UTF-8. Templates are UTF-8, so a conflicting
    /// declaration would make the connector decode the bytes incorrectly.
    pub fn is_utf8(&self) -> bool {
        self.mime
            .params()
            .all(|(name, value)| name != mime::CHARSET || is_utf8_label(value.as_str().as_bytes()))
    }
}

/// Whether a charset or XML encoding label names UTF-8.
pub fn is_utf8_label(label: &[u8]) -> bool {
    label.eq_ignore_ascii_case(b"utf-8") || label.eq_ignore_ascii_case(b"utf8")
}

fn default_request_headers() -> [(String, Maskable<String>); 1] {
    use http::header;

    [(header::VIA.to_string(), "HyperSwitch".to_string().into())]
}

#[derive(Debug)]
pub struct Request {
    pub url: String,
    pub headers: Headers,
    pub method: Method,
    pub certificate: Option<Secret<String>>,
    pub certificate_key: Option<Secret<String>>,
    pub body: Option<RequestContent>,
    pub ca_certificate: Option<Secret<String>>,
    pub query_params: Option<serde_json::Value>,
}

impl std::fmt::Debug for RequestContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Json(_) => "JsonRequestBody",
            Self::FormUrlEncoded(_) => "FormUrlEncodedRequestBody",
            Self::FormData(_) => "FormDataRequestBody",
            Self::Xml(_, _) => "XmlRequestBody",
            Self::RawBytes(_) => "RawBytesRequestBody",
        })
    }
}

pub enum RequestContent {
    Json(Box<dyn hyperswitch_masking::ErasedMaskSerialize + Send>),
    FormUrlEncoded(Box<dyn hyperswitch_masking::ErasedMaskSerialize + Send>),
    FormData(
        (
            Form,
            Box<dyn hyperswitch_masking::ErasedMaskSerialize + Send>,
        ),
    ),
    Xml(
        Box<dyn hyperswitch_masking::ErasedMaskSerialize + Send>,
        Option<XmlConfig>,
    ),
    RawBytes(Vec<u8>),
}

impl RequestContent {
    pub fn get_inner_value(&self) -> Secret<String> {
        match self {
            Self::Json(i) => serde_json::to_string(&i).unwrap_or_default().into(),
            Self::FormUrlEncoded(i) => serde_urlencoded::to_string(i).unwrap_or_default().into(),
            Self::Xml(i, _) => quick_xml::se::to_string(&i).unwrap_or_default().into(),
            Self::FormData((_, i)) => serde_json::to_string(i).unwrap_or_default().into(),
            Self::RawBytes(_) => String::new().into(),
        }
    }
}

impl Request {
    pub fn new(method: Method, url: &str) -> Self {
        Self {
            method,
            url: String::from(url),
            headers: std::collections::HashSet::new(),
            certificate: None,
            certificate_key: None,
            body: None,
            ca_certificate: None,
            query_params: None,
        }
    }

    pub fn set_body<T: Into<RequestContent>>(&mut self, body: T) {
        self.body.replace(body.into());
    }

    pub fn add_default_headers(&mut self) {
        self.headers.extend(default_request_headers());
    }

    pub fn add_header(&mut self, header: &str, value: Maskable<String>) {
        self.headers.insert((String::from(header), value));
    }

    pub fn add_certificate(&mut self, certificate: Option<Secret<String>>) {
        self.certificate = certificate;
    }

    pub fn add_certificate_key(&mut self, certificate_key: Option<Secret<String>>) {
        self.certificate = certificate_key;
    }
}

#[derive(Debug)]
pub struct RequestBuilder {
    pub url: String,
    pub headers: Headers,
    pub method: Method,
    pub certificate: Option<Secret<String>>,
    pub certificate_key: Option<Secret<String>>,
    pub body: Option<RequestContent>,
    pub ca_certificate: Option<Secret<String>>,
    pub query_params: Option<serde_json::Value>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            method: Method::Get,
            url: String::with_capacity(1024),
            headers: std::collections::HashSet::new(),
            certificate: None,
            certificate_key: None,
            body: None,
            ca_certificate: None,
            query_params: None,
        }
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = url.into();
        self
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    pub fn attach_default_headers(mut self) -> Self {
        self.headers.extend(default_request_headers());
        self
    }

    pub fn header(mut self, header: &str, value: &str) -> Self {
        self.headers.insert((header.into(), value.into()));
        self
    }

    pub fn headers(mut self, headers: Vec<(String, Maskable<String>)>) -> Self {
        self.headers.extend(headers);
        self
    }

    pub fn set_optional_body<T: Into<RequestContent>>(mut self, body: Option<T>) -> Self {
        body.map(|body| self.body.replace(body.into()));
        self
    }

    pub fn set_body<T: Into<RequestContent>>(mut self, body: T) -> Self {
        self.body.replace(body.into());
        self
    }

    pub fn add_certificate(mut self, certificate: Option<Secret<String>>) -> Self {
        self.certificate = certificate;
        self
    }

    pub fn add_certificate_key(mut self, certificate_key: Option<Secret<String>>) -> Self {
        self.certificate_key = certificate_key;
        self
    }

    pub fn add_ca_certificate_pem(mut self, ca_certificate: Option<Secret<String>>) -> Self {
        self.ca_certificate = ca_certificate;
        self
    }

    pub fn set_query_params(mut self, query_params: Option<serde_json::Value>) -> Self {
        self.query_params = query_params;
        self
    }

    pub fn build(self) -> Request {
        Request {
            method: self.method,
            url: self.url,
            headers: self.headers,
            certificate: self.certificate,
            certificate_key: self.certificate_key,
            body: self.body,
            ca_certificate: self.ca_certificate,
            query_params: self.query_params,
        }
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
