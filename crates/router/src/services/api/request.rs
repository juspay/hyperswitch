use std::str::FromStr;

pub use common_utils::request::ContentType;
use common_utils::request::Headers;
use error_stack::{IntoReport, ResultExt};
pub use masking::{Mask, Maskable};
use router_env::{instrument, tracing};

use crate::core::errors::{self, CustomResult};

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

        self.into_iter().try_fold(
            HeaderMap::new(),
            |mut header_map, (header_name, header_value)| {
                let header_name = HeaderName::from_str(&header_name)
                    .into_report()
                    .change_context(errors::ApiClientError::HeaderMapConstructionFailed)?;
                let header_value = header_value.into_inner();
                let header_value = HeaderValue::from_str(&header_value)
                    .into_report()
                    .change_context(errors::ApiClientError::HeaderMapConstructionFailed)?;
                header_map.append(header_name, header_value);
                Ok(header_map)
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
