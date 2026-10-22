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
use url::Url;

use super::{MetricPoint, MetricRange, MetricSeries, MetricsError, MetricsResult};
use crate::http_client;

const QUERY_TIMEOUT_SECONDS: u64 = 10;
pub(crate) const QUERY_REF_ID: &str = "A";

#[derive(Debug, Clone, Deserialize)]
pub struct GrafanaConfig {
    pub base_url: Url,
    pub api_token: Secret<String>,
}

#[derive(Debug, Clone)]
pub struct GrafanaClient {
    config: GrafanaConfig,
    proxy: Proxy,
}

impl GrafanaClient {
    pub fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self { config, proxy }
    }

    async fn fetch<Q>(&self, query: Q, range: MetricRange) -> MetricsResult<Vec<MetricSeries>>
    where
        Q: Serialize + Send + Sync,
    {
        let payload = serde_json::to_value(QueryEnvelope {
            from: epoch_milliseconds(range.from),
            to: epoch_milliseconds(range.to),
            queries: [query],
        })
        .change_context(MetricsError::RequestSerialization)?;
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
                return Err(Report::from(error).change_context(MetricsError::Timeout))
            }
            Err(error) => return Err(Report::from(error).change_context(MetricsError::Transport)),
        };
        parse_response(&body, status)
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
    values: Option<Vec<serde_json::Value>>,
}

fn epoch_milliseconds(timestamp: PrimitiveDateTime) -> String {
    (timestamp.assume_utc().unix_timestamp_nanos() / 1_000_000).to_string()
}

fn parse_response(body: &str, status: reqwest::StatusCode) -> MetricsResult<Vec<MetricSeries>> {
    let response: QueryResponse = match serde_json::from_str(body) {
        Ok(response) => response,
        Err(error) if !status.is_success() => {
            return Err(
                Report::from(error).change_context(MetricsError::HttpStatus {
                    status: status.as_u16(),
                }),
            );
        }
        Err(error) => {
            return Err(Report::from(error).change_context(MetricsError::MalformedResponse))
        }
    };
    let results = response.results;
    if let Some(reason) = results
        .as_ref()
        .and_then(|results| results.values().find_map(|result| result.error.clone()))
    {
        return Err(Report::new(MetricsError::QueryRefused { reason }));
    }
    if !status.is_success() {
        return Err(Report::new(MetricsError::HttpStatus {
            status: status.as_u16(),
        }));
    }
    let result = results
        .ok_or_else(|| malformed_response("response is missing results"))?
        .remove(QUERY_REF_ID)
        .ok_or_else(|| {
            malformed_response(format!("response is missing query result {QUERY_REF_ID}"))
        })?;
    result
        .frames
        .into_iter()
        .flatten()
        .map(parse_frame)
        .collect()
}

fn parse_frame(frame: Frame) -> MetricsResult<MetricSeries> {
    let values = frame
        .data
        .ok_or_else(|| malformed_response("frame is missing data"))?
        .values
        .ok_or_else(|| malformed_response("frame data is missing values"))?;
    if values.is_empty() {
        return Ok(MetricSeries { points: Vec::new() });
    }
    let [timestamps, metric_values]: [serde_json::Value; 2] =
        values
            .try_into()
            .map_err(|values: Vec<serde_json::Value>| {
                malformed_response(format!(
                    "frame contains {} value arrays; expected 0 or 2",
                    values.len()
                ))
            })?;
    let serde_json::Value::Array(timestamps) = timestamps else {
        return Err(malformed_response("timestamps are not an array"));
    };
    let serde_json::Value::Array(metric_values) = metric_values else {
        return Err(malformed_response("metric values are not an array"));
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
        .filter(|(_, value)| !value.is_null())
        .map(|(timestamp, value)| {
            let value = value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| malformed_response("frame contains an invalid metric value"))?;
            let milliseconds = timestamp
                .as_i64()
                .ok_or_else(|| malformed_response("frame contains an invalid timestamp"))?;
            let timestamp =
                OffsetDateTime::from_unix_timestamp_nanos(i128::from(milliseconds) * 1_000_000)
                    .change_context(MetricsError::MalformedResponse)
                    .attach_printable("frame contains an out-of-range timestamp")?;
            Ok(MetricPoint {
                timestamp: convert_to_pdt(timestamp),
                value,
            })
        })
        .collect::<MetricsResult<Vec<_>>>()?;
    Ok(MetricSeries { points })
}

fn malformed_response(detail: impl Into<String>) -> Report<MetricsError> {
    Report::new(MetricsError::MalformedResponse).attach_printable(detail.into())
}
