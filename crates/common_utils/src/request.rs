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
}

fn default_request_headers() -> [(String, Maskable<String>); 1] {
    use http::header;

    [(header::VIA.to_string(), "HyperSwitch".to_string().into())]
}

#[derive(Debug)]
pub struct Request {
    pub url: String,
    pub headers: Headers,
    pub payload: Option<Secret<String>>,
    pub method: Method,
    pub content_type: Option<ContentType>,
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
    pub form_data: Option<reqwest::multipart::Form>,
}

impl Request {
    pub fn new(method: Method, url: &str) -> Self {
        Self {
            method,
            url: String::from(url),
            headers: std::collections::HashSet::new(),
            payload: None,
            content_type: None,
            certificate: None,
            certificate_key: None,
            form_data: None,
        }
    }

    pub fn set_body(&mut self, body: String) {
        self.payload = Some(body.into());
    }

    pub fn add_default_headers(&mut self) {
        self.headers.extend(default_request_headers());
    }

    pub fn add_header(&mut self, header: &str, value: Maskable<String>) {
        self.headers.insert((String::from(header), value));
    }

    pub fn add_content_type(&mut self, content_type: ContentType) {
        self.content_type = Some(content_type);
    }

    pub fn add_certificate(&mut self, certificate: Option<String>) {
        self.certificate = certificate;
    }

    pub fn add_certificate_key(&mut self, certificate_key: Option<String>) {
        self.certificate = certificate_key;
    }

    pub fn set_form_data(&mut self, form_data: reqwest::multipart::Form) {
        self.form_data = Some(form_data);
    }
}

#[derive(Debug)]
pub struct RequestBuilder {
    pub url: String,
    pub headers: Headers,
    pub payload: Option<Secret<String>>,
    pub method: Method,
    pub content_type: Option<ContentType>,
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
    pub form_data: Option<reqwest::multipart::Form>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            method: Method::Get,
            url: String::with_capacity(1024),
            headers: std::collections::HashSet::new(),
            payload: None,
            content_type: None,
            certificate: None,
            certificate_key: None,
            form_data: None,
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

    pub fn form_data(mut self, form_data: Option<reqwest::multipart::Form>) -> Self {
        self.form_data = form_data;
        self
    }

    pub fn body(mut self, option_body: Option<RequestBody>) -> Self {
        self.payload = option_body.map(RequestBody::get_inner_value);
        self
    }

    pub fn content_type(mut self, content_type: ContentType) -> Self {
        self.content_type = Some(content_type);
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
            payload: self.payload,
            content_type: self.content_type,
            certificate: self.certificate,
            certificate_key: self.certificate_key,
            form_data: self.form_data,
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
