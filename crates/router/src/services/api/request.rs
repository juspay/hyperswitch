use std::{collections, str::FromStr};

use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use router_env::{instrument, tracing};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors::{self, CustomResult},
    logger,
};

pub(crate) type Headers = collections::HashSet<(String, String)>;

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

fn default_request_headers() -> [(String, String); 1] {
    use http::header;

    [(header::VIA.to_string(), "HyperSwitch".into())]
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub url: String,
    pub headers: Headers,
    pub payload: Option<Secret<String>>,
    pub method: Method,
    pub content_type: Option<ContentType>,
    pub certificate: Option<String>,
    pub certificate_key: Option<String>,
    pub file_data: Option<Vec<u8>>,
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
            file_data: None,
        }
    }

    pub fn set_body(&mut self, body: String) {
        self.payload = Some(body.into());
    }

    pub fn add_default_headers(&mut self) {
        self.headers.extend(default_request_headers());
    }

    pub fn add_header(&mut self, header: &str, value: &str) {
        self.headers
            .insert((String::from(header), String::from(value)));
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

    pub fn set_file_data(&mut self, file_data: Vec<u8>) {
        self.file_data = Some(file_data);
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
    pub file_data: Option<Vec<u8>>,
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
            file_data: None,
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

    pub fn headers(mut self, headers: Vec<(String, String)>) -> Self {
        let mut h = headers.into_iter().map(|(h, v)| (h, v));
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
            file_data: self.file_data,
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

        self.iter().fold(
            Ok(HeaderMap::new()),
            |mut header_map, (header_name, header_value)| {
                let header_name = HeaderName::from_str(header_name)
                    .into_report()
                    .change_context(errors::ApiClientError::HeaderMapConstructionFailed)?;
                let header_value = HeaderValue::from_str(header_value)
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
