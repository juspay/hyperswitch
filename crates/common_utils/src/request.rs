use masking::{Maskable, Secret};
#[cfg(feature = "logs")]
use router_env::logger;
use serde::{Deserialize, Serialize};

use crate::errors;

pub type Headers = std::collections::HashSet<(String, Maskable<String>)>;

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ContentType {
    Json,
    FormUrlEncoded,
    FormData,
    Xml,
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
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
    pub body: Option<RequestContent>,
}

impl std::fmt::Debug for RequestContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Json(_) => "JsonRequestBody",
            Self::FormUrlEncoded(_) => "FormUrlEncodedRequestBody",
            Self::FormData(_) => "FormDataRequestBody",
            Self::Xml(_) => "XmlRequestBody",
        })
    }
}

// #[derive(Debug)]
pub enum RequestContent {
    Json(Box<dyn masking::ErasedMaskSerialize>),
    FormUrlEncoded(Box<dyn masking::ErasedMaskSerialize>),
    FormData(reqwest::multipart::Form),
    Xml(Box<dyn masking::ErasedMaskSerialize>),
}

// #[derive(Debug)]
pub struct JsonRequestBody(pub Box<dyn masking::ErasedMaskSerialize>);

// #[derive(Debug)]
pub struct FormRequestBody(pub reqwest::multipart::Form);

// #[derive(Debug)]
pub struct FormUrlEncodedRequestBody(pub Box<dyn masking::ErasedMaskSerialize>);

pub struct XmlRequestBody(pub Box<dyn masking::ErasedMaskSerialize>);

impl From<JsonRequestBody> for RequestContent {
    fn from(value: JsonRequestBody) -> Self {
        Self::Json(value.0)
    }
}

impl From<FormRequestBody> for RequestContent {
    fn from(value: FormRequestBody) -> Self {
        Self::FormData(value.0)
    }
}

impl From<XmlRequestBody> for RequestContent {
    fn from(value: XmlRequestBody) -> Self {
        Self::Xml(value.0)
    }
}

impl From<FormUrlEncodedRequestBody> for RequestContent {
    fn from(value: FormUrlEncodedRequestBody) -> Self {
        Self::FormUrlEncoded(value.0)
    }
}

pub trait HttpRequestBody {
    fn get_content_type(&self) -> ContentType;
}

impl HttpRequestBody for JsonRequestBody {
    fn get_content_type(&self) -> ContentType {
        ContentType::Json
    }
}

impl HttpRequestBody for FormRequestBody {
    fn get_content_type(&self) -> ContentType {
        ContentType::FormData
    }
}

impl HttpRequestBody for FormUrlEncodedRequestBody {
    fn get_content_type(&self) -> ContentType {
        ContentType::FormUrlEncoded
    }
}

impl HttpRequestBody for XmlRequestBody {
    fn get_content_type(&self) -> ContentType {
        ContentType::Xml
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

    pub fn add_certificate(&mut self, certificate: Option<String>) {
        self.certificate = certificate;
    }

    pub fn add_certificate_key(&mut self, certificate_key: Option<String>) {
        self.certificate = certificate_key;
    }
}

#[derive(Debug)]
pub struct RequestBuilder {
    pub url: String,
    pub headers: Headers,
    pub method: Method,
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
    pub body: Option<RequestContent>,
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
        let mut h = headers.into_iter().map(|(h, v)| (h, v));
        self.headers.extend(&mut h);
        self
    }

    pub fn set_body<T: Into<RequestContent>>(mut self, body: T) -> Self {
        self.body.replace(body.into());
        self
    }

    pub fn add_certificate(mut self, certificate: Option<String>) -> Self {
        self.certificate = certificate;
        self
    }

    pub fn add_certificate_key(mut self, certificate_key: Option<String>) -> Self {
        self.certificate_key = certificate_key;
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
        }
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct RequestBody(Secret<String>);

impl RequestBody {
    pub fn log_and_get_request_body<T, F>(
        body: T,
        encoder: F,
    ) -> errors::CustomResult<Self, errors::ParsingError>
    where
        F: FnOnce(T) -> errors::CustomResult<String, errors::ParsingError>,
        T: std::fmt::Debug,
    {
        #[cfg(feature = "logs")]
        logger::info!(connector_request_body=?body);
        Ok(Self(Secret::new(encoder(body)?)))
    }
    pub fn get_inner_value(request_body: Self) -> Secret<String> {
        request_body.0
    }
}
