pub mod cloudwatch;

use common_utils::request::{Method, RequestBuilder, RequestContent};
use error_stack::Report;
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use hyperswitch_masking::{Mask as _, PeekInterface, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{MetricsError, MetricsResult};
use crate::http_client;

const QUERY_TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GrafanaConfig {
    pub base_url: String,
    pub api_token: Secret<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct GrafanaClient {
    config: GrafanaConfig,
    proxy: Proxy,
}

impl GrafanaClient {
    async fn execute<MetricsRequest, MetricsResponse>(
        &self,
        payload: MetricsRequest,
    ) -> MetricsResult<(reqwest::StatusCode, MetricsResponse)>
    where
        MetricsRequest: Serialize + Send + 'static,
        MetricsResponse: DeserializeOwned,
    {
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&format!(
                "{}/api/ds/query",
                self.config.base_url.as_str().trim_end_matches('/')
            ))
            .attach_default_headers()
            .headers(vec![
                (
                    http::header::AUTHORIZATION.to_string(),
                    format!("Bearer {}", self.config.api_token.peek()).into_masked(),
                ),
                (
                    http::header::CONTENT_TYPE.to_string(),
                    mime::APPLICATION_JSON.essence_str().to_owned().into(),
                ),
            ])
            .set_body(RequestContent::Json(Box::new(payload)))
            .build();
        let response = match http_client::send_request(
            &self.proxy,
            request,
            Some(QUERY_TIMEOUT_SECONDS),
        )
        .await
        {
            Ok(response) => response,
            Err(error) if error.current_context() == &HttpClientError::RequestTimeoutReceived => {
                return Err(error.change_context(MetricsError::Timeout));
            }
            Err(error) => return Err(error.change_context(MetricsError::Transport)),
        };
        let status = response.status();
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => {
                return Err(Report::from(error).change_context(MetricsError::Timeout));
            }
            Err(error) => return Err(Report::from(error).change_context(MetricsError::Transport)),
        };
        let response = match serde_json::from_str::<MetricsResponse>(&body) {
            Ok(response) => response,
            Err(error)
                if matches!(
                    status,
                    reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
                ) =>
            {
                return Err(Report::from(error).change_context(MetricsError::Unauthorized));
            }
            Err(error) if !status.is_success() => {
                return Err(
                    Report::from(error).change_context(MetricsError::HttpStatus {
                        status: status.as_u16(),
                    }),
                );
            }
            Err(error) => {
                return Err(Report::from(error).change_context(MetricsError::MalformedResponse));
            }
        };
        Ok((status, response))
    }
}
