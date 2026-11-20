pub mod cloudwatch;
pub mod loki;

use std::collections::{BTreeMap, HashMap};

use common_utils::{
    date_time::convert_to_pdt,
    ext_traits::BytesExt,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::{Report, ResultExt};
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use hyperswitch_masking::{Mask as _, PeekInterface, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};
use url::Url;

use super::{MetricPoint, MetricSeries, MetricsError, MetricsResult};
use crate::http_client;

const QUERY_TIMEOUT_SECONDS: u64 = 10;
const QUERY_REF_ID: &str = "A";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GrafanaConfig {
    pub base_url: Option<Url>,
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
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| Report::new(MetricsError::MissingGrafanaBaseUrl))?;
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&format!(
                "{}/api/ds/query",
                base_url.as_str().trim_end_matches('/')
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

    fn handle_response(
        &self,
        response: GrafanaMetricsResponse,
        status: reqwest::StatusCode,
    ) -> MetricsResult<Vec<MetricSeries>> {
        let mut results = response.results;
        if let Some(reason) = results
            .as_ref()
            .and_then(|results| results.values().find_map(|result| result.error.as_deref()))
        {
            let api_token = self.config.api_token.peek();
            return Err(Report::new(MetricsError::QueryRefused {
                reason: if api_token.is_empty() {
                    reason.to_owned()
                } else {
                    reason.replace(api_token, "[REDACTED]")
                },
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
            .ok_or(MetricsError::MalformedResponse)
            .attach_printable("response is missing results")?
            .remove(QUERY_REF_ID)
            .ok_or(MetricsError::MalformedResponse)
            .attach_printable_lazy(|| format!("response is missing query result {QUERY_REF_ID}"))?;
        result
            .frames
            .unwrap_or_default()
            .into_iter()
            .map(parse_frame)
            .collect()
    }
}

fn epoch_millis(timestamp: PrimitiveDateTime) -> String {
    (timestamp.assume_utc().unix_timestamp_nanos() / 1_000_000).to_string()
}

#[derive(Debug, Serialize)]
struct GrafanaDatasource {
    #[serde(rename = "type")]
    kind: &'static str,
    uid: String,
}

#[derive(Debug, Deserialize)]
pub struct GrafanaMetricsResponse {
    results: Option<HashMap<String, QueryResult>>,
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    frames: Option<Vec<Frame>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Frame {
    schema: Option<FrameSchema>,
    data: Option<FrameData>,
}

#[derive(Debug, Deserialize)]
struct FrameSchema {
    fields: Vec<FrameField>,
}

#[derive(Debug, Deserialize)]
struct FrameField {
    labels: Option<BTreeMap<String, String>>,
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

fn parse_frame(frame: Frame) -> MetricsResult<MetricSeries> {
    let labels = frame
        .schema
        .and_then(|schema| {
            schema
                .fields
                .into_iter()
                .find_map(|field| field.labels.filter(|labels| !labels.is_empty()))
        })
        .unwrap_or_default();
    let values = frame
        .data
        .ok_or(MetricsError::MalformedResponse)
        .attach_printable("frame is missing data")?
        .values
        .ok_or(MetricsError::MalformedResponse)
        .attach_printable("frame data is missing values")?;
    let (timestamps, metric_values) = match values {
        FrameValues::Empty([]) => {
            return Ok(MetricSeries {
                points: Vec::new(),
                labels,
            })
        }
        FrameValues::Series(series) => series,
    };
    if timestamps.len() != metric_values.len() {
        return Err(
            Report::new(MetricsError::MalformedResponse).attach_printable(format!(
                "frame contains {} timestamps and {} metric values",
                timestamps.len(),
                metric_values.len()
            )),
        );
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
                        return Err(Report::new(MetricsError::MalformedResponse)
                            .attach_printable("frame contains a non-finite metric value"));
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
    Ok(MetricSeries { points, labels })
}
