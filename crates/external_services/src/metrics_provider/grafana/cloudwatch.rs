use std::collections::{BTreeMap, HashMap};

use common_utils::date_time::convert_to_pdt;
use error_stack::{Report, ResultExt};
use hyperswitch_interfaces::types::Proxy;
use hyperswitch_masking::PeekInterface;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};

use super::{GrafanaClient, GrafanaConfig};
use crate::metrics_provider::{MetricsError, MetricsProvider, MetricsResult};

const QUERY_REF_ID: &str = "A";

#[derive(Debug, Clone)]
pub struct GrafanaCloudwatch {
    client: GrafanaClient,
}

impl GrafanaCloudwatch {
    pub fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self {
            client: GrafanaClient { config, proxy },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloudwatchMetricsQuery {
    pub datasource_uid: String,
    pub region: String,
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: BTreeMap<String, String>,
    pub statistic: CloudwatchStatistic,
    pub period: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CloudwatchStatistic {
    Average,
    Maximum,
    Minimum,
    Sum,
}

#[derive(Debug, Serialize)]
pub struct CloudwatchMetricsRequest {
    from: String,
    to: String,
    queries: [GrafanaCloudwatchQuery; 1],
}

#[async_trait::async_trait]
impl MetricsProvider for GrafanaCloudwatch {
    type MetricsQuery = CloudwatchMetricsQuery;
    type MetricsRequest = CloudwatchMetricsRequest;
    type MetricsResponse = GrafanaMetricsResponse;
    type MetricsError = MetricsError;

    fn validate_query(query: &Self::MetricsQuery) -> MetricsResult<()> {
        for (field, value) in [
            ("datasource_uid", query.datasource_uid.as_str()),
            ("region", query.region.as_str()),
            ("namespace", query.namespace.as_str()),
            ("metric_name", query.metric_name.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(Report::new(MetricsError::InvalidQuery {
                    field,
                    requirement: "must not be blank",
                }));
            }
        }
        if query.dimensions.len() > 30 {
            return Err(Report::new(MetricsError::InvalidQuery {
                field: "dimensions",
                requirement: "must contain at most 30 entries",
            }));
        }
        if query
            .dimensions
            .iter()
            .any(|(name, value)| name.trim().is_empty() || value.trim().is_empty())
        {
            return Err(Report::new(MetricsError::InvalidQuery {
                field: "dimensions",
                requirement: "names and values must not be blank",
            }));
        }
        let supported_period = matches!(query.period, 1 | 5 | 10 | 20 | 30)
            || ((60..=86_400).contains(&query.period) && query.period % 60 == 0);
        if !supported_period {
            return Err(Report::new(MetricsError::InvalidQuery {
                field: "period",
                requirement: "must be 1, 5, 10, 20, 30, or a multiple of 60 up to 86400 seconds",
            }));
        }
        Ok(())
    }

    fn build_request(
        query: Self::MetricsQuery,
        range: MetricRange,
    ) -> MetricsResult<Self::MetricsRequest> {
        Self::validate_query(&query)?;
        Ok(CloudwatchMetricsRequest {
            from: (range.from.assume_utc().unix_timestamp_nanos() / 1_000_000).to_string(),
            to: (range.to.assume_utc().unix_timestamp_nanos() / 1_000_000).to_string(),
            queries: [GrafanaCloudwatchQuery {
                ref_id: QUERY_REF_ID,
                datasource: GrafanaDatasource {
                    kind: "cloudwatch",
                    uid: query.datasource_uid,
                },
                query_mode: "Metrics",
                metric_query_type: 0,
                metric_editor_mode: 0,
                region: query.region,
                namespace: query.namespace,
                metric_name: query.metric_name,
                dimensions: query
                    .dimensions
                    .into_iter()
                    .map(|(name, value)| (name, vec![value]))
                    .collect(),
                statistic: query.statistic,
                period: query.period.to_string(),
                match_exact: true,
                id: "",
                expression: "",
            }],
        })
    }

    fn handle_response(
        &self,
        response: Self::MetricsResponse,
        status: reqwest::StatusCode,
    ) -> MetricsResult<Vec<MetricSeries>> {
        let mut results = response.results;
        if let Some(reason) = results
            .as_ref()
            .and_then(|results| results.values().find_map(|result| result.error.as_deref()))
        {
            let api_token = self.client.config.api_token.peek();
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaCloudwatchQuery {
    ref_id: &'static str,
    datasource: GrafanaDatasource,
    query_mode: &'static str,
    metric_query_type: u8,
    metric_editor_mode: u8,
    region: String,
    namespace: String,
    metric_name: String,
    dimensions: BTreeMap<String, Vec<String>>,
    statistic: CloudwatchStatistic,
    period: String,
    match_exact: bool,
    id: &'static str,
    expression: &'static str,
}

#[derive(Debug, Serialize)]
struct GrafanaDatasource {
    #[serde(rename = "type")]
    kind: &'static str,
    uid: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricRange {
    pub from: PrimitiveDateTime,
    pub to: PrimitiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricSeries {
    pub points: Vec<MetricPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricPoint {
    pub timestamp: PrimitiveDateTime,
    pub value: f64,
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

fn parse_frame(frame: Frame) -> MetricsResult<MetricSeries> {
    let values = frame
        .data
        .ok_or(MetricsError::MalformedResponse)
        .attach_printable("frame is missing data")?
        .values
        .ok_or(MetricsError::MalformedResponse)
        .attach_printable("frame data is missing values")?;
    let (timestamps, metric_values) = match values {
        FrameValues::Empty([]) => return Ok(MetricSeries { points: Vec::new() }),
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
    Ok(MetricSeries { points })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use common_utils::date_time::convert_to_pdt;
    use hyperswitch_masking::Secret;
    use serde_json::json;
    use time::OffsetDateTime;
    use wiremock::{
        matchers::{body_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;
    use crate::metrics_provider::MetricPoint;

    const TOKEN: &str = "provider-test-token";

    fn query() -> CloudwatchMetricsQuery {
        CloudwatchMetricsQuery {
            datasource_uid: "cloudwatch-main".to_owned(),
            region: "us-east-1".to_owned(),
            namespace: "AWS/ApplicationELB".to_owned(),
            metric_name: "HTTPCode_Target_5XX_Count".to_owned(),
            dimensions: BTreeMap::from([("LoadBalancer".to_owned(), "app/test/123".to_owned())]),
            statistic: CloudwatchStatistic::Sum,
            period: 60,
        }
    }

    fn range() -> MetricRange {
        MetricRange {
            from: convert_to_pdt(OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()),
            to: convert_to_pdt(OffsetDateTime::from_unix_timestamp(1_700_000_300).unwrap()),
        }
    }

    fn provider(server: &MockServer) -> GrafanaCloudwatch {
        GrafanaCloudwatch::new(
            GrafanaConfig {
                base_url: server.uri(),
                api_token: Secret::new(TOKEN.to_owned()),
            },
            Proxy::default(),
        )
    }

    async fn mount(server: &MockServer, status: u16, body: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(server)
            .await;
    }

    #[test]
    fn validates_required_fields_dimensions_and_periods() {
        for period in [1, 5, 10, 20, 30, 60, 300, 86_400] {
            let mut candidate = query();
            candidate.period = period;
            GrafanaCloudwatch::validate_query(&candidate).unwrap();
        }
        for (expected_field, case) in [
            ("datasource_uid", 0_u8),
            ("region", 1),
            ("namespace", 2),
            ("metric_name", 3),
            ("period", 4),
        ] {
            let mut candidate = query();
            match case {
                0 => candidate.datasource_uid = "  ".to_owned(),
                1 => candidate.region.clear(),
                2 => candidate.namespace.clear(),
                3 => candidate.metric_name.clear(),
                4 => candidate.period = 61,
                _ => unreachable!(),
            }
            let error = GrafanaCloudwatch::validate_query(&candidate).unwrap_err();
            assert!(matches!(
                error.current_context(),
                MetricsError::InvalidQuery { field, .. } if *field == expected_field
            ));
        }
        let mut blank_dimension = query();
        blank_dimension
            .dimensions
            .insert("blank".to_owned(), " ".to_owned());
        assert!(matches!(
            GrafanaCloudwatch::validate_query(&blank_dimension)
                .unwrap_err()
                .current_context(),
            MetricsError::InvalidQuery {
                field: "dimensions",
                ..
            }
        ));
        let mut too_many_dimensions = query();
        too_many_dimensions.dimensions = (0..31)
            .map(|index| (format!("name-{index}"), "value".to_owned()))
            .collect();
        assert!(GrafanaCloudwatch::validate_query(&too_many_dimensions).is_err());
    }

    #[tokio::test]
    async fn fetch_posts_the_typed_grafana_cloudwatch_envelope() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
            .and(header("content-type", "application/json"))
            .and(body_json(json!({
                "from": "1700000000000",
                "to": "1700000300000",
                "queries": [{
                    "refId": "A",
                    "datasource": {"type": "cloudwatch", "uid": "cloudwatch-main"},
                    "queryMode": "Metrics",
                    "metricQueryType": 0,
                    "metricEditorMode": 0,
                    "region": "us-east-1",
                    "namespace": "AWS/ApplicationELB",
                    "metricName": "HTTPCode_Target_5XX_Count",
                    "dimensions": {"LoadBalancer": ["app/test/123"]},
                    "statistic": "Sum",
                    "period": "60",
                    "matchExact": true,
                    "id": "",
                    "expression": ""
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": {"A": {"frames": [{"data": {"values": [
                    [1700000000000_i64, 1700000060000_i64, 1700000120000_i64],
                    [1.5, null, 3.25]
                ]}}]}}
            })))
            .mount(&server)
            .await;
        let request = GrafanaCloudwatch::build_request(query(), range()).unwrap();
        let series = provider(&server).fetch_metrics(request).await.unwrap();
        assert_eq!(
            series,
            vec![MetricSeries {
                points: vec![
                    MetricPoint {
                        timestamp: convert_to_pdt(
                            OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
                        ),
                        value: 1.5
                    },
                    MetricPoint {
                        timestamp: convert_to_pdt(
                            OffsetDateTime::from_unix_timestamp(1_700_000_120).unwrap()
                        ),
                        value: 3.25
                    },
                ],
            }]
        );
    }

    #[tokio::test]
    async fn successful_no_frame_empty_and_all_null_responses_are_empty_data() {
        let bodies = [
            json!({"results": {"A": {}}}),
            json!({"results": {"A": {"frames": []}}}),
            json!({"results": {"A": {"frames": [{"data": {"values": []}}]}}}),
            json!({"results": {"A": {"frames": [{"data": {"values": [[1700000000000_i64], [null]]}}]}}}),
        ];
        for (index, body) in bodies.into_iter().enumerate() {
            let server = MockServer::start().await;
            mount(&server, 200, body).await;
            let request = GrafanaCloudwatch::build_request(query(), range()).unwrap();
            let series = provider(&server).fetch_metrics(request).await.unwrap();
            if index < 2 {
                assert!(series.is_empty());
            } else {
                assert_eq!(series, vec![MetricSeries { points: Vec::new() }]);
            }
        }
    }

    #[tokio::test]
    async fn malformed_frames_are_not_reported_as_empty_data() {
        let malformed_bodies = [
            (
                "unaligned values",
                json!({"results": {"A": {"frames": [{"data": {"values": [[1700000000000_i64], [1.0, 2.0]]}}]}}}),
            ),
            (
                "out-of-range null timestamp",
                json!({"results": {"A": {"frames": [{"data": {"values": [[i64::MAX], [null]]}}]}}}),
            ),
            (
                "missing result A",
                json!({"results": {"B": {"frames": []}}}),
            ),
            (
                "missing values",
                json!({"results": {"A": {"frames": [{"data": {}}]}}}),
            ),
        ];
        for (case, body) in malformed_bodies {
            let server = MockServer::start().await;
            mount(&server, 200, body).await;
            let request = GrafanaCloudwatch::build_request(query(), range()).unwrap();
            let error = provider(&server).fetch_metrics(request).await.unwrap_err();
            assert!(
                matches!(error.current_context(), MetricsError::MalformedResponse),
                "malformed case `{case}` returned context `{}`; full report: {error:?}",
                error.current_context()
            );
        }
    }

    #[tokio::test]
    async fn non_finite_json_number_is_a_malformed_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"results":{"A":{"frames":[{"data":{"values":[[1700000000000],[1e400]]}}]}}}"#,
                "application/json",
            ))
            .mount(&server)
            .await;
        let request = GrafanaCloudwatch::build_request(query(), range()).unwrap();
        let error = provider(&server).fetch_metrics(request).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            MetricsError::MalformedResponse
        ));
    }

    #[tokio::test]
    async fn query_error_wins_over_http_status_and_redacts_the_token() {
        let server = MockServer::start().await;
        mount(
            &server,
            400,
            json!({"results": {"A": {"error": format!("credential {TOKEN} rejected")}}}),
        )
        .await;
        let request = GrafanaCloudwatch::build_request(query(), range()).unwrap();
        let error = provider(&server).fetch_metrics(request).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            MetricsError::QueryRefused { reason }
                if reason == "credential [REDACTED] rejected" && !reason.contains(TOKEN)
        ));
        assert!(!format!("{error:?}").contains(TOKEN));
    }

    #[tokio::test]
    async fn authorization_and_other_non_success_statuses_are_typed_failures() {
        for (status, unauthorized) in [(401, true), (502, false)] {
            let server = MockServer::start().await;
            mount(&server, status, json!({"message": "failed"})).await;
            let request = GrafanaCloudwatch::build_request(query(), range()).unwrap();
            let error = provider(&server).fetch_metrics(request).await.unwrap_err();
            if unauthorized {
                assert!(matches!(
                    error.current_context(),
                    MetricsError::Unauthorized
                ));
            } else {
                assert!(matches!(
                    error.current_context(),
                    MetricsError::HttpStatus { status: 502 }
                ));
            }
        }
    }

    #[tokio::test]
    async fn build_request_validates_before_sending() {
        let server = MockServer::start().await;
        let mut invalid = query();
        invalid.period = 90;
        let error = GrafanaCloudwatch::build_request(invalid, range()).unwrap_err();
        assert!(matches!(
            error.current_context(),
            MetricsError::InvalidQuery {
                field: "period",
                ..
            }
        ));
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
