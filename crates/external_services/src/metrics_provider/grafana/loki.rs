use std::collections::{BTreeMap, HashMap};

use common_utils::date_time::convert_to_pdt;
use error_stack::{Report, ResultExt};
use hyperswitch_interfaces::types::Proxy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use super::{GrafanaClient, GrafanaConfig};
use crate::metrics_provider::{
    MetricPoint, MetricRange, MetricSeries, MetricsError, MetricsProvider, MetricsResult,
};

const QUERY_REF_ID: &str = "A";

#[derive(Debug, Clone)]
pub struct GrafanaLoki {
    client: GrafanaClient,
}

impl GrafanaLoki {
    pub fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self {
            client: GrafanaClient { config, proxy },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LokiQueryMode {
    Instant,
    Range,
}

#[derive(Debug, Clone)]
pub struct LokiMetricsQuery {
    pub datasource_uid: String,
    pub expression: String,
    pub mode: LokiQueryMode,
    pub step_seconds: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct LokiMetricsRequest {
    from: String,
    to: String,
    queries: [LokiQuery; 1],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LokiQuery {
    ref_id: &'static str,
    datasource: LokiDatasource,
    expr: String,
    query_type: LokiQueryMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    step: Option<String>,
}

#[derive(Debug, Serialize)]
struct LokiDatasource {
    #[serde(rename = "type")]
    kind: &'static str,
    uid: String,
}

#[derive(Debug, Deserialize)]
pub struct GrafanaLokiResponse {
    results: Option<HashMap<String, LokiQueryResult>>,
}

#[derive(Debug, Deserialize)]
struct LokiQueryResult {
    status: Option<u16>,
    error: Option<String>,
    #[serde(default)]
    frames: Vec<LokiFrame>,
}

#[derive(Debug, Deserialize)]
struct LokiFrame {
    schema: LokiFrameSchema,
    data: LokiFrameData,
}

#[derive(Debug, Deserialize)]
struct LokiFrameSchema {
    fields: Vec<LokiField>,
    meta: Option<LokiFrameMeta>,
}

#[derive(Debug, Deserialize)]
struct LokiFrameMeta {
    #[serde(default)]
    notices: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct LokiField {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    labels: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct LokiFrameData {
    values: Vec<Vec<Value>>,
}

#[async_trait::async_trait]
impl MetricsProvider for GrafanaLoki {
    type MetricsQuery = LokiMetricsQuery;
    type MetricsRequest = LokiMetricsRequest;
    type MetricsResponse = GrafanaLokiResponse;
    type MetricsError = MetricsError;

    fn build_request(
        query: Self::MetricsQuery,
        range: MetricRange,
    ) -> MetricsResult<Self::MetricsRequest> {
        Ok(LokiMetricsRequest {
            from: (range.from.assume_utc().unix_timestamp_nanos() / 1_000_000).to_string(),
            to: (range.to.assume_utc().unix_timestamp_nanos() / 1_000_000).to_string(),
            queries: [LokiQuery {
                ref_id: QUERY_REF_ID,
                datasource: LokiDatasource {
                    kind: "loki",
                    uid: query.datasource_uid,
                },
                expr: query.expression,
                query_type: query.mode,
                step: query.step_seconds.map(|seconds| format!("{seconds}s")),
            }],
        })
    }

    fn handle_response(
        &self,
        response: Self::MetricsResponse,
        status: reqwest::StatusCode,
    ) -> MetricsResult<Vec<MetricSeries>> {
        if matches!(
            status,
            reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
        ) {
            return Err(Report::new(MetricsError::Unauthorized));
        }
        if status != reqwest::StatusCode::OK {
            return Err(Report::new(MetricsError::HttpStatus {
                status: status.as_u16(),
            }));
        }
        let mut results = response.results.ok_or(MetricsError::MalformedResponse)?;
        let result = results
            .remove(QUERY_REF_ID)
            .ok_or(MetricsError::MalformedResponse)?;
        if result.error.is_some() || result.status.is_some_and(|status| status != 200) {
            return Err(Report::new(MetricsError::QueryRefused {
                reason: "Loki query failed or returned an incomplete result".to_owned(),
            }));
        }
        result
            .frames
            .into_iter()
            .map(parse_loki_frame)
            .collect::<MetricsResult<Vec<_>>>()
            .map(|frames| frames.into_iter().flatten().collect())
    }

    async fn fetch_metrics(
        &self,
        request: Self::MetricsRequest,
    ) -> MetricsResult<Vec<MetricSeries>> {
        let (status, response) = self
            .client
            .execute::<Self::MetricsRequest, Self::MetricsResponse>(request)
            .await?;
        self.handle_response(response, status)
    }
}

fn parse_loki_frame(frame: LokiFrame) -> MetricsResult<Vec<MetricSeries>> {
    if frame
        .schema
        .meta
        .as_ref()
        .is_some_and(|meta| !meta.notices.is_empty())
    {
        return Err(Report::new(MetricsError::QueryRefused {
            reason: "Loki returned frame notices; result completeness requires verification"
                .to_owned(),
        }));
    }
    let fields = frame.schema.fields;
    let values = frame.data.values;
    if fields.len() != values.len() {
        return Err(Report::new(MetricsError::MalformedResponse));
    }
    let time_fields = fields
        .iter()
        .enumerate()
        .filter(|(_, f)| f.kind == "time")
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let [time_index] = time_fields.as_slice() else {
        return Err(Report::new(MetricsError::MalformedResponse));
    };
    let timestamps = values
        .get(*time_index)
        .ok_or(MetricsError::MalformedResponse)?;
    let numeric = fields
        .iter()
        .enumerate()
        .filter(|(_, f)| f.kind == "number")
        .collect::<Vec<_>>();
    if numeric.is_empty()
        || fields
            .iter()
            .any(|f| !matches!(f.kind.as_str(), "number" | "time"))
    {
        return Err(Report::new(MetricsError::MalformedResponse));
    }
    numeric
        .into_iter()
        .map(|(index, field)| {
            let numbers = values.get(index).ok_or(MetricsError::MalformedResponse)?;
            if numbers.len() != timestamps.len() {
                return Err(Report::new(MetricsError::MalformedResponse));
            }
            if numbers.iter().any(Value::is_null) && !numbers.iter().all(Value::is_null) {
                return Err(Report::new(MetricsError::QueryRefused {
                    reason:
                        "Loki series contains missing samples; strict reduction cannot ignore them"
                            .to_owned(),
                }));
            }
            let points = timestamps
                .iter()
                .zip(numbers)
                .map(|(timestamp, value)| {
                    let millis = timestamp.as_i64().ok_or(MetricsError::MalformedResponse)?;
                    let timestamp =
                        OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
                            .map(convert_to_pdt)
                            .change_context(MetricsError::MalformedResponse)?;
                    if value.is_null() {
                        return Ok(None);
                    }
                    let value = value
                        .as_f64()
                        .filter(|v| v.is_finite())
                        .ok_or(MetricsError::MalformedResponse)
                        .attach_printable("Loki numeric samples must be finite")?;
                    Ok(Some(MetricPoint { timestamp, value }))
                })
                .collect::<MetricsResult<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect();
            Ok(MetricSeries {
                labels: field.labels.clone(),
                points,
            })
        })
        .collect()
}
