use std::{collections, str::FromStr};

use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, Secret};
use router_env::{instrument, tracing};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors::{self, CustomResult},
    types,
};

pub(crate) type Headers = collections::HashSet<(String, Maskable<String>)>;

///
/// An Enum that allows us to optionally mask data, based on which enum variant that data is stored
/// in.
///
#[derive(Clone, Eq, PartialEq)]
pub enum Maskable<T: Eq + PartialEq + Clone> {
    Masked(Secret<T>),
    Normal(T),
}

impl<T: std::fmt::Debug + Clone + Eq + PartialEq> std::fmt::Debug for Maskable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Masked(secret_value) => std::fmt::Debug::fmt(secret_value, f),
            Self::Normal(value) => std::fmt::Debug::fmt(value, f),
        }
    }
}

impl<T: Eq + PartialEq + Clone + std::hash::Hash> std::hash::Hash for Maskable<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Masked(value) => masking::PeekInterface::peek(value).hash(state),
            Self::Normal(value) => value.hash(state),
        }
    }
}

impl<T: Eq + PartialEq + Clone> Maskable<T> {
    pub fn into_inner(self) -> T {
        match self {
            Self::Masked(inner_secret) => inner_secret.expose(),
            Self::Normal(inner) => inner,
        }
    }

    pub fn new_masked(item: Secret<T>) -> Self {
        Self::Masked(item)
    }
    pub fn new_normal(item: T) -> Self {
        Self::Normal(item)
    }
}

pub trait Mask {
    type Output: Eq + Clone + PartialEq;
    fn into_masked(self) -> Maskable<Self::Output>;
}

impl Mask for String {
    type Output = Self;
    fn into_masked(self) -> Maskable<Self::Output> {
        Maskable::new_masked(self.into())
    }
}

impl Mask for Secret<String> {
    type Output = String;
    fn into_masked(self) -> Maskable<Self::Output> {
        Maskable::new_masked(self)
    }
}

impl<T: Eq + PartialEq + Clone> From<T> for Maskable<T> {
    fn from(value: T) -> Self {
        Self::new_normal(value)
    }
}

impl From<&str> for Maskable<String> {
    fn from(value: &str) -> Self {
        Self::new_normal(value.to_string())
    }
}

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
            headers: collections::HashSet::new(),
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

    pub fn body(mut self, option_body: Option<types::RequestBody>) -> Self {
        self.payload = option_body.map(types::RequestBody::get_inner_value);
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

pub(super) trait HeaderExt {
    fn construct_header_map(
        self,
    ) -> CustomResult<reqwest::header::HeaderMap, errors::ApiClientError>;
}

impl HeaderExt for Headers {
    fn construct_header_map(
        self,
    ) -> CustomResult<reqwest::header::HeaderMap, errors::ApiClientError> {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

        self.into_iter().fold(
            Ok(HeaderMap::new()),
            |mut header_map, (header_name, header_value)| {
                let header_name = HeaderName::from_str(&header_name)
                    .into_report()
                    .change_context(errors::ApiClientError::HeaderMapConstructionFailed)?;
                let header_value = header_value.into_inner();
                let header_value = HeaderValue::from_str(&header_value)
                    .into_report()
                    .change_context(errors::ApiClientError::HeaderMapConstructionFailed)?;
                if let Ok(map) = header_map.as_mut() {
                    map.append(header_name, header_value);
                }
                header_map
            },
        )
    }
}

pub(super) trait RequestBuilderExt {
    fn add_headers(self, headers: reqwest::header::HeaderMap) -> Self;
}

impl RequestBuilderExt for reqwest::RequestBuilder {
    #[instrument(skip_all)]
    fn add_headers(mut self, headers: reqwest::header::HeaderMap) -> Self {
        self = self.headers(headers);
        self
    }
}
