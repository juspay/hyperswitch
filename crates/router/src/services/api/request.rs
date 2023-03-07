use std::{
    collections,
    fmt::Debug,
    hash::{Hash, Hasher},
    str::FromStr,
};

use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use router_env::{instrument, tracing};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors::{self, CustomResult},
    logger,
};

pub(crate) type Headers = collections::HashSet<(String, Box<dyn HeaderValue>)>;

pub trait HeaderValue: ToString + Debug + erased_serde::Serialize + Send + Sync {}

pub mod header {
    use super::HeaderValue;
    pub type Value = Box<dyn HeaderValue>;
}

impl Hash for dyn HeaderValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string().hash(state)
    }
}

impl PartialEq for dyn HeaderValue {
    fn eq(&self, other: &Self) -> bool {
        // self.to_string() == other.to_string()
        let mut hasher1 = collections::hash_map::DefaultHasher::new();
        let mut hasher2 = collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher1);
        other.hash(&mut hasher2);
        hasher1.finish() == hasher2.finish()
    }
}

impl Eq for dyn HeaderValue {}

impl HeaderValue for String {}
impl HeaderValue for Secret<String, masking::ApiKey> {}

pub fn concat_headers(
    a: Vec<(String, impl HeaderValue + 'static)>,
    b: Vec<(String, impl HeaderValue + 'static)>,
) -> Vec<(String, Box<dyn HeaderValue>)> {
    a.into_iter()
        .map(|(key, value)| -> (String, Box<dyn HeaderValue>) { (key, Box::new(value)) })
        .chain(
            b.into_iter()
                .map(|(key, value)| -> (String, Box<dyn HeaderValue>) { (key, Box::new(value)) }),
        )
        .collect()
}

pub fn construct_headers(
    value: Vec<(String, impl HeaderValue + 'static)>,
) -> Vec<(String, header::Value)> {
    value
        .into_iter()
        .map(|(key, value)| -> (String, header::Value) { (key, Box::new(value)) })
        .collect()
}

erased_serde::serialize_trait_object!(HeaderValue);

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
}

#[derive(Debug, Serialize)]
pub struct Request {
    pub url: String,
    pub headers: Headers,
    pub payload: Option<Secret<String>>,
    pub method: Method,
    pub content_type: Option<ContentType>,
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
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
        }
    }

    pub fn set_body(&mut self, body: String) {
        self.payload = Some(body.into());
    }

    pub fn add_header(&mut self, header: &str, value: &str) {
        self.headers
            .insert((String::from(header), Box::new(String::from(value))));
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
}

pub struct RequestBuilder {
    pub url: String,
    pub headers: Headers,
    pub payload: Option<Secret<String>>,
    pub method: Method,
    pub content_type: Option<ContentType>,
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
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

    pub fn header(mut self, header: &str, value: &str) -> Self {
        self.headers
            .insert((header.into(), Box::new(value.to_string())));
        self
    }

    pub fn headers(mut self, headers: Vec<(String, Box<dyn HeaderValue>)>) -> Self {
        let mut h = headers
            .into_iter()
            .map(|(h, v)| -> (String, Box<dyn HeaderValue>) { (h, v) });
        self.headers.extend(&mut h);
        self
    }

    pub fn body(mut self, body: Option<String>) -> Self {
        self.payload = body.map(From::from);
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
        use reqwest::header::{self, HeaderMap, HeaderName};

        self.iter().fold(
            Ok(HeaderMap::new()),
            |mut header_map, (header_name, header_value)| {
                let header_name = HeaderName::from_str(header_name)
                    .into_report()
                    .change_context(errors::ApiClientError::HeaderMapConstructionFailed)?;
                let header_value = header::HeaderValue::from_str(&header_value.to_string())
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
        logger::debug!(request_builder=?self);
        self
    }
}
