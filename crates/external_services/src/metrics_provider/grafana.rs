pub mod cloudwatch;
pub mod loki;

use common_utils::{
    ext_traits::BytesExt,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::Report;
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use hyperswitch_masking::{Mask as _, PeekInterface, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{MetricsError, MetricsResult};
use crate::http_client;

const QUERY_TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrafanaAuthType {
    #[default]
    Bearer,
    SessionCookie,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GrafanaConfig {
    pub base_url: String,
    pub api_token: Secret<String>,
    #[serde(default)]
    pub auth_type: GrafanaAuthType,
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
        let (auth_header, credential) = match self.config.auth_type {
            GrafanaAuthType::Bearer => (
                http::header::AUTHORIZATION,
                format!("Bearer {}", self.config.api_token.peek()),
            ),
            GrafanaAuthType::SessionCookie => (
                http::header::COOKIE,
                format!("grafana_session={}", self.config.api_token.peek()),
            ),
        };
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&format!(
                "{}/api/ds/query",
                self.config.base_url.as_str().trim_end_matches('/')
            ))
            .attach_default_headers()
            .headers(vec![
                (auth_header.to_string(), credential.into_masked()),
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
        let body = match response.bytes().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => {
                return Err(Report::from(error).change_context(MetricsError::Timeout));
            }
            Err(error) => return Err(Report::from(error).change_context(MetricsError::Transport)),
        };
        let response = body
            .parse_struct::<MetricsResponse>("Grafana metrics response")
            .map_err(|_| {
                let context = match status {
                    reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
                        MetricsError::Unauthorized
                    }
                    status if !status.is_success() => MetricsError::HttpStatus {
                        status: status.as_u16(),
                    },
                    _ => MetricsError::MalformedResponse,
                };
                // Parsing attachments can contain response content, including echoed credentials.
                Report::new(context)
            })?;
        Ok((status, response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_mode_is_explicit_and_defaults_to_bearer() {
        let mut value =
            serde_json::json!({"base_url": "http://localhost", "api_token": "test-secret"});
        let config: GrafanaConfig = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(config.auth_type, GrafanaAuthType::Bearer));
        value["auth_type"] = serde_json::json!("session_cookie");
        let config: GrafanaConfig = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(config.auth_type, GrafanaAuthType::SessionCookie));
        value["auth_type"] = serde_json::json!("unknown");
        assert!(serde_json::from_value::<GrafanaConfig>(value).is_err());
    }
}
