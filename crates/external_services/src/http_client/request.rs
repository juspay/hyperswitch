use std::str::FromStr;

use common_utils::request::Headers;
pub use common_utils::{errors::CustomResult, request::ContentType};
use error_stack::ResultExt;
use hyperswitch_interfaces::errors::HttpClientError;
pub use masking::{Mask, Maskable};
use router_env::{instrument, tracing};

#[allow(missing_docs)]
pub trait HeaderExt {
    fn construct_header_map(self) -> CustomResult<reqwest::header::HeaderMap, HttpClientError>;
}

impl HeaderExt for Headers {
    fn construct_header_map(self) -> CustomResult<reqwest::header::HeaderMap, HttpClientError> {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

        self.into_iter().try_fold(
            HeaderMap::new(),
            |mut header_map, (header_name, header_value)| {
                let header_name = HeaderName::from_str(&header_name)
                    .change_context(HttpClientError::HeaderMapConstructionFailed)?;
                let header_value = header_value.into_inner();
                let header_value = HeaderValue::from_str(&header_value)
                    .change_context(HttpClientError::HeaderMapConstructionFailed)?;
                header_map.append(header_name, header_value);
                Ok(header_map)
            },
        )
    }
}

#[allow(missing_docs)]
pub trait RequestBuilderExt {
    fn add_headers(self, headers: reqwest::header::HeaderMap) -> Self;
}

impl RequestBuilderExt for reqwest::RequestBuilder {
    #[instrument(skip_all)]
    fn add_headers(mut self, headers: reqwest::header::HeaderMap) -> Self {
        self = self.headers(headers);
        self
    }
}
