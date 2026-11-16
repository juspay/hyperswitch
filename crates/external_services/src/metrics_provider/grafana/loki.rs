use std::{cmp::Reverse, collections::HashMap};

use common_utils::date_time::convert_to_pdt;
use error_stack::{Report, ResultExt};
use hyperswitch_interfaces::types::Proxy;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use super::{
    epoch_millis, GrafanaClient, GrafanaConfig, GrafanaDatasource, GrafanaMetricsResponse,
    QueryResult, QUERY_REF_ID,
};
use crate::metrics_provider::{
    MetricPoint, MetricRange, MetricSeries, MetricsError, MetricsProvider, MetricsResult,
};

#[derive(Debug, Clone)]
pub struct GrafanaLoki {
    client: GrafanaClient,
}

#[derive(Debug, Clone)]
pub struct LokiMetricsQuery {
    pub datasource_uid: String,
    pub expr: String,
    pub period: u32,
}

#[derive(Debug, Serialize)]
pub struct LokiMetricsRequest {
    from: String,
    to: String,
    queries: [GrafanaLokiQuery; 1],
    #[serde(skip)]
    period: u32,
}

#[derive(Debug, Clone)]
pub struct LokiLogsQuery {
    pub datasource_uid: String,
    pub expr: String,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub timestamp: PrimitiveDateTime,
    pub line: String,
}

impl GrafanaLoki {
    pub fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self {
            client: GrafanaClient { config, proxy },
        }
    }

    pub async fn fetch_logs(
        &self,
        query: LokiLogsQuery,
        range: MetricRange,
    ) -> MetricsResult<Vec<LogLine>> {
        let (status, response) = self
            .client
            .execute::<LokiLogsRequest, GrafanaLogsResponse>(LokiLogsRequest {
                from: epoch_millis(range.from),
                to: epoch_millis(range.to),
                queries: [GrafanaLokiLogsQuery {
                    ref_id: QUERY_REF_ID,
                    datasource: GrafanaDatasource {
                        kind: "loki",
                        uid: query.datasource_uid,
                    },
                    expr: query.expr,
                    query_type: QueryType::Range,
                    max_lines: query.limit,
                    direction: Direction::Backward,
                }],
            })
            .await?;
        let mut lines = self
            .client
            .take_frames(response.results, status)?
            .into_iter()
            .map(parse_log_frame)
            .collect::<MetricsResult<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        lines.sort_by_key(|line| Reverse(line.timestamp));
        lines.truncate(usize::try_from(query.limit).unwrap_or(usize::MAX));
        Ok(lines)
    }
}

#[async_trait::async_trait]
impl MetricsProvider for GrafanaLoki {
    type MetricsQuery = LokiMetricsQuery;
    type MetricsRequest = LokiMetricsRequest;
    type MetricsResponse = GrafanaMetricsResponse;
    type MetricsError = MetricsError;

    fn build_request(
        query: Self::MetricsQuery,
        range: MetricRange,
    ) -> MetricsResult<Self::MetricsRequest> {
        let period = i64::from(query.period);
        let steps = (range.to - range.from)
            .whole_seconds()
            .checked_div(period)
            .ok_or(MetricsError::MalformedResponse)
            .attach_printable("query period must be greater than zero")?;
        Ok(LokiMetricsRequest {
            from: epoch_millis(range.to - Duration::seconds(period * steps)),
            to: epoch_millis(range.to),
            queries: [GrafanaLokiQuery {
                ref_id: QUERY_REF_ID,
                datasource: GrafanaDatasource {
                    kind: "loki",
                    uid: query.datasource_uid,
                },
                expr: query.expr,
                query_type: QueryType::Range,
                step: format!("{}s", query.period),
                interval_ms: u64::from(query.period) * 1000,
            }],
            period: query.period,
        })
    }

    fn handle_response(
        &self,
        response: Self::MetricsResponse,
        status: reqwest::StatusCode,
    ) -> MetricsResult<Vec<MetricSeries>> {
        self.client.handle_response(response, status)
    }

    async fn fetch_metrics(
        &self,
        request: Self::MetricsRequest,
    ) -> MetricsResult<Vec<MetricSeries>> {
        let period = Duration::seconds(i64::from(request.period));
        let (status, response) = self
            .client
            .execute::<Self::MetricsRequest, Self::MetricsResponse>(request)
            .await?;
        self.handle_response(response, status)?
            .into_iter()
            .map(|series| {
                series
                    .points
                    .into_iter()
                    .map(|point| {
                        point
                            .timestamp
                            .checked_sub(period)
                            .map(|timestamp| MetricPoint { timestamp, ..point })
                            .ok_or(MetricsError::MalformedResponse)
                            .attach_printable("restamped timestamp is out of range")
                    })
                    .collect::<MetricsResult<_>>()
                    .map(|points| MetricSeries {
                        points,
                        labels: series.labels,
                    })
            })
            .collect()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum QueryType {
    Range,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum Direction {
    Backward,
}

#[derive(Debug, Serialize)]
struct LokiLogsRequest {
    from: String,
    to: String,
    queries: [GrafanaLokiLogsQuery; 1],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaLokiLogsQuery {
    ref_id: &'static str,
    datasource: GrafanaDatasource,
    expr: String,
    query_type: QueryType,
    max_lines: u32,
    direction: Direction,
}

#[derive(Debug, Deserialize)]
struct GrafanaLogsResponse {
    results: Option<HashMap<String, QueryResult<LogFrame>>>,
}

#[derive(Debug, Deserialize)]
struct LogFrame {
    schema: Option<LogFrameSchema>,
    data: Option<LogFrameData>,
}

#[derive(Debug, Deserialize)]
struct LogFrameSchema {
    fields: Vec<LogFrameField>,
}

#[derive(Debug, Deserialize)]
struct LogFrameField {
    name: String,
}

#[derive(Debug, Deserialize)]
struct LogFrameData {
    values: Option<Vec<Vec<serde_json::Value>>>,
}

fn parse_log_frame(frame: LogFrame) -> MetricsResult<Vec<LogLine>> {
    let column = |name: &str| {
        frame
            .schema
            .as_ref()
            .and_then(|schema| schema.fields.iter().position(|field| field.name == name))
            .ok_or(MetricsError::MalformedResponse)
            .attach_printable_lazy(|| format!("log frame is missing the {name} field"))
    };
    let (time, line) = (column("Time")?, column("Line")?);
    let values = frame.data.and_then(|data| data.values).unwrap_or_default();
    let (Some(times), Some(lines)) = (values.get(time), values.get(line)) else {
        return Ok(Vec::new());
    };
    times
        .iter()
        .zip(lines)
        .map(|(time, line)| {
            let milliseconds = time
                .as_i64()
                .ok_or(MetricsError::MalformedResponse)
                .attach_printable("log frame time is not an integer")?;
            let timestamp =
                OffsetDateTime::from_unix_timestamp_nanos(i128::from(milliseconds) * 1_000_000)
                    .change_context(MetricsError::MalformedResponse)
                    .attach_printable("log frame contains an out-of-range timestamp")?;
            Ok(LogLine {
                timestamp: convert_to_pdt(timestamp),
                line: line
                    .as_str()
                    .ok_or_else(|| Report::new(MetricsError::MalformedResponse))
                    .attach_printable("log frame line is not a string")?
                    .to_owned(),
            })
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaLokiQuery {
    ref_id: &'static str,
    datasource: GrafanaDatasource,
    expr: String,
    query_type: QueryType,
    step: String,
    interval_ms: u64,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use hyperswitch_masking::Secret;
    use serde_json::json;
    use wiremock::{
        matchers::{body_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn at(seconds: i64) -> PrimitiveDateTime {
        convert_to_pdt(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
    }

    #[tokio::test]
    async fn fetch_posts_a_range_query_and_stamps_points_at_bucket_start() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .and(header("authorization", "Bearer loki-test-token"))
            .and(body_json(json!({
                "from": "1700000000000",
                "to": "1700000300000",
                "queries": [{
                    "refId": "A",
                    "datasource": {"type": "loki", "uid": "loki-main"},
                    "expr": "sum(count_over_time({app=\"router\"} |= \"error\" [1m]))",
                    "queryType": "range",
                    "step": "60s",
                    "intervalMs": 60000
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": {"A": {"frames": [{
                    "schema": {"fields": [{"name": "Time"}, {"name": "Value", "labels": {"app": "router"}}]},
                    "data": {"values": [
                    [1_700_000_240_000_i64, 1_700_000_300_000_i64],
                    [3.0, 5.0]
                ]}}]}}
            })))
            .mount(&server)
            .await;
        let provider = GrafanaLoki::new(
            GrafanaConfig {
                base_url: Some(server.uri().parse().unwrap()),
                api_token: Secret::new("loki-test-token".to_owned()),
            },
            Proxy::default(),
        );
        let request = GrafanaLoki::build_request(
            LokiMetricsQuery {
                datasource_uid: "loki-main".to_owned(),
                expr: "sum(count_over_time({app=\"router\"} |= \"error\" [1m]))".to_owned(),
                period: 60,
            },
            MetricRange {
                from: at(1_700_000_000),
                to: at(1_700_000_300),
            },
        )
        .unwrap();
        assert_eq!(
            provider.fetch_metrics(request).await.unwrap(),
            vec![MetricSeries {
                labels: BTreeMap::from([("app".to_owned(), "router".to_owned())]),
                points: vec![
                    MetricPoint {
                        timestamp: at(1_700_000_180),
                        value: 3.0,
                    },
                    MetricPoint {
                        timestamp: at(1_700_000_240),
                        value: 5.0,
                    },
                ],
            }]
        );
    }

    #[tokio::test]
    async fn fetch_logs_posts_a_backward_log_query_and_returns_the_newest_lines() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .and(body_json(json!({
                "from": "1700000000000",
                "to": "1700000060000",
                "queries": [{
                    "refId": "A",
                    "datasource": {"type": "loki", "uid": "loki-main"},
                    "expr": "{app=\"router\"} |= \"error\"",
                    "queryType": "range",
                    "maxLines": 2,
                    "direction": "backward"
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": {"A": {"frames": [{
                    "schema": {"fields": [
                        {"name": "labels"}, {"name": "Time"}, {"name": "Line"}, {"name": "tsNs"}
                    ]},
                    "data": {"values": [
                        [{"app": "router"}, {"app": "router"}, {"app": "router"}],
                        [1_700_000_010_000_i64, 1_700_000_050_000_i64, 1_700_000_030_000_i64],
                        ["first", "newest", "middle"],
                        ["1700000010000000000", "1700000050000000000", "1700000030000000000"]
                    ]}
                }]}}
            })))
            .mount(&server)
            .await;
        let provider = GrafanaLoki::new(
            GrafanaConfig {
                base_url: Some(server.uri().parse().unwrap()),
                api_token: Secret::new("loki-test-token".to_owned()),
            },
            Proxy::default(),
        );
        assert_eq!(
            provider
                .fetch_logs(
                    LokiLogsQuery {
                        datasource_uid: "loki-main".to_owned(),
                        expr: "{app=\"router\"} |= \"error\"".to_owned(),
                        limit: 2,
                    },
                    MetricRange {
                        from: at(1_700_000_000),
                        to: at(1_700_000_060),
                    },
                )
                .await
                .unwrap(),
            vec![
                LogLine {
                    timestamp: at(1_700_000_050),
                    line: "newest".to_owned(),
                },
                LogLine {
                    timestamp: at(1_700_000_030),
                    line: "middle".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn query_start_is_snapped_onto_the_period_grid() {
        let request = GrafanaLoki::build_request(
            LokiMetricsQuery {
                datasource_uid: "loki-main".to_owned(),
                expr: "sum(count_over_time({app=\"router\"}[2m]))".to_owned(),
                period: 120,
            },
            MetricRange {
                from: at(1_700_000_040),
                to: at(1_700_000_340),
            },
        )
        .unwrap();
        assert_eq!(request.from, "1700000100000");
        assert_eq!(request.to, "1700000340000");
    }
}
