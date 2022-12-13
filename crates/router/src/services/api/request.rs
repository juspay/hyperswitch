use std::str::FromStr;

use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use router_env::{tracing, tracing::instrument};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors::{self, CustomResult},
    logger,
};

pub(crate) type Headers = Vec<(String, String)>;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub url: String,
    pub headers: Headers,
    pub payload: Option<Secret<String>>,
    pub method: Method,
    pub content_type: Option<ContentType>,
}

impl Request {
    pub fn new(method: Method, url: &str) -> Request {
        Request {
            method,
            url: String::from(url),
            headers: Vec::new(),
            payload: None,
            content_type: None,
        }
    }

    pub fn set_body(&mut self, body: String) {
        self.payload = Some(body.into());
    }

    pub fn add_header(&mut self, header: &str, value: &str) {
        self.headers
            .push((String::from(header), String::from(value)));
    }

    pub fn add_content_type(&mut self, content_type: ContentType) {
        self.content_type = Some(content_type);
    }
}

pub struct RequestBuilder {
    pub url: String,
    pub headers: Headers,
    pub payload: Option<Secret<String>>,
    pub method: Method,
    pub content_type: Option<ContentType>,
}

impl RequestBuilder {
    pub fn new() -> RequestBuilder {
        RequestBuilder {
            method: Method::Get,
            url: String::with_capacity(1024),
            headers: Vec::new(),
            payload: None,
            content_type: None,
        }
    }

    pub fn url(mut self, url: &str) -> RequestBuilder {
        self.url = url.into();
        self
    }

    pub fn method(mut self, method: Method) -> RequestBuilder {
        self.method = method;
        self
    }

    pub fn header(mut self, header: &str, value: &str) -> RequestBuilder {
        self.headers.push((header.into(), value.into()));
        self
    }

    pub fn headers(mut self, headers: Vec<(String, String)>) -> RequestBuilder {
        // Fixme add union property
        let mut h = headers.into_iter().map(|(h, v)| (h, v)).collect();
        self.headers.append(&mut h);
        self
    }

    pub fn body(mut self, body: Option<String>) -> RequestBuilder {
        self.payload = body.map(From::from);
        self
    }

    pub fn content_type(mut self, content_type: ContentType) -> RequestBuilder {
        self.content_type = Some(content_type);
        self
    }

    pub fn build(self) -> Request {
        Request {
            method: self.method,
            url: self.url,
            headers: self.headers,
            payload: self.payload,
            content_type: self.content_type,
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
