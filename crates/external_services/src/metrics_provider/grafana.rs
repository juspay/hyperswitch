pub mod cloudwatch;

use std::collections::HashMap;

use common_utils::{
    date_time::convert_to_pdt,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::{Report, ResultExt};
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use hyperswitch_masking::{Mask as _, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};

use super::{MetricPoint, MetricRange, MetricSeries, MetricsError, MetricsResult};
use crate::http_client;

const QUERY_TIMEOUT_SECONDS: u64 = 10;
pub(crate) const QUERY_REF_ID: &str = "A";

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
    pub(crate) fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self { config, proxy }
    }

    async fn fetch<Q>(&self, query: Q, range: MetricRange) -> MetricsResult<Vec<MetricSeries>>
    where
        Q: Serialize + Send + 'static,
    {
        let payload = QueryEnvelope {
            from: epoch_milliseconds(range.from),
            to: epoch_milliseconds(range.to),
            queries: [query],
        };
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
        parse_response(&body, status, self.config.api_token.peek())
    }
}

#[derive(Debug, Serialize)]
struct QueryEnvelope<Q> {
    from: String,
    to: String,
    queries: [Q; 1],
}

#[derive(Debug, Deserialize)]
struct QueryResponse {
    results: Option<HashMap<String, QueryResult>>,
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    frames: Option<Vec<Frame>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Frame {
    data: Option<FrameData>,
}

#[derive(Debug, Deserialize)]
struct FrameData {
    values: Option<FrameValues>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FrameValues {
    Series((Vec<i64>, Vec<Option<f64>>)),
    Empty([(); 0]),
}

fn epoch_milliseconds(timestamp: PrimitiveDateTime) -> String {
    (timestamp.assume_utc().unix_timestamp_nanos() / 1_000_000).to_string()
}

fn parse_response(
    body: &str,
    status: reqwest::StatusCode,
    api_token: &str,
) -> MetricsResult<Vec<MetricSeries>> {
    let response: QueryResponse = match serde_json::from_str(body) {
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
    let mut results = response.results;
    if let Some(reason) = results
        .as_ref()
        .and_then(|results| results.values().find_map(|result| result.error.as_deref()))
    {
        return Err(Report::new(MetricsError::QueryRefused {
            reason: redact_token(reason, api_token),
        }));
    }
    if matches!(
        status,
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
    ) {
        return Err(Report::new(MetricsError::Unauthorized));
    }
    if !status.is_success() {
        return Err(Report::new(MetricsError::HttpStatus {
            status: status.as_u16(),
        }));
    }
    let result = results
        .as_mut()
        .ok_or_else(|| malformed_response("response is missing results"))?
        .remove(QUERY_REF_ID)
        .ok_or_else(|| {
            malformed_response(format!("response is missing query result {QUERY_REF_ID}"))
        })?;
    result
        .frames
        .unwrap_or_default()
        .into_iter()
        .map(parse_frame)
        .collect()
}

fn parse_frame(frame: Frame) -> MetricsResult<MetricSeries> {
    let values = frame
        .data
        .ok_or_else(|| malformed_response("frame is missing data"))?
        .values
        .ok_or_else(|| malformed_response("frame data is missing values"))?;
    let (timestamps, metric_values) = match values {
        FrameValues::Empty([]) => return Ok(MetricSeries { points: Vec::new() }),
        FrameValues::Series(series) => series,
    };
    if timestamps.len() != metric_values.len() {
        return Err(malformed_response(format!(
            "frame contains {} timestamps and {} metric values",
            timestamps.len(),
            metric_values.len()
        )));
    }
    let points = timestamps
        .into_iter()
        .zip(metric_values)
        .map(|(milliseconds, value)| {
            let timestamp =
                OffsetDateTime::from_unix_timestamp_nanos(i128::from(milliseconds) * 1_000_000)
                    .change_context(MetricsError::MalformedResponse)
                    .attach_printable("frame contains an out-of-range timestamp")?;
            value
                .map(|value| {
                    if !value.is_finite() {
                        return Err(malformed_response(
                            "frame contains a non-finite metric value",
                        ));
                    }
                    Ok(MetricPoint {
                        timestamp: convert_to_pdt(timestamp),
                        value,
                    })
                })
                .transpose()
        })
        .filter_map(Result::transpose)
        .collect::<MetricsResult<Vec<_>>>()?;
    Ok(MetricSeries { points })
}

fn redact_token(message: &str, api_token: &str) -> String {
    if api_token.is_empty() {
        message.to_owned()
    } else {
        message.replace(api_token, "[REDACTED]")
    }
}

fn malformed_response(detail: impl Into<String>) -> Report<MetricsError> {
    Report::new(MetricsError::MalformedResponse).attach_printable(detail.into())
}
