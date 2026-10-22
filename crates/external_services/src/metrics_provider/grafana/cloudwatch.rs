use std::collections::BTreeMap;

use error_stack::Report;
use hyperswitch_interfaces::types::Proxy;
use serde::Serialize;

use super::{GrafanaClient, GrafanaConfig, QUERY_REF_ID};
use crate::metrics_provider::{
    MetricRange, MetricSeries, MetricsError, MetricsProvider, MetricsResult,
};

#[derive(Debug, Clone)]
pub struct GrafanaCloudwatch {
    client: GrafanaClient,
}

impl GrafanaCloudwatch {
    pub fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self {
            client: GrafanaClient::new(config, proxy),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloudwatchMetricRequest {
    pub datasource_uid: String,
    pub region: String,
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: BTreeMap<String, String>,
    pub statistic: CloudwatchStatistic,
    pub period: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudwatchStatistic {
    Average,
    Maximum,
    Minimum,
    Sum,
}

impl CloudwatchStatistic {
    fn as_grafana_value(self) -> &'static str {
        match self {
            Self::Average => "Average",
            Self::Maximum => "Maximum",
            Self::Minimum => "Minimum",
            Self::Sum => "Sum",
        }
    }
}

#[async_trait::async_trait]
impl MetricsProvider for GrafanaCloudwatch {
    type Request = CloudwatchMetricRequest;
    type Response = Vec<MetricSeries>;
    type Error = MetricsError;

    fn validate_request(request: &Self::Request) -> MetricsResult<()> {
        validate_nonblank("datasource_uid", &request.datasource_uid)?;
        validate_nonblank("region", &request.region)?;
        validate_nonblank("namespace", &request.namespace)?;
        validate_nonblank("metric_name", &request.metric_name)?;
        if request.dimensions.len() > 30 {
            return Err(invalid_request(
                "dimensions",
                "must contain at most 30 entries",
            ));
        }
        if request
            .dimensions
            .iter()
            .any(|(name, value)| name.trim().is_empty() || value.trim().is_empty())
        {
            return Err(invalid_request(
                "dimensions",
                "names and values must not be blank",
            ));
        }
        let supported_period = matches!(request.period, 1 | 5 | 10 | 20 | 30)
            || ((60..=86_400).contains(&request.period) && request.period % 60 == 0);
        if !supported_period {
            return Err(invalid_request(
                "period",
                "must be 1, 5, 10, 20, 30, or a multiple of 60 up to 86400 seconds",
            ));
        }
        Ok(())
    }

    async fn fetch(
        &self,
        request: Self::Request,
        range: MetricRange,
    ) -> MetricsResult<Self::Response> {
        Self::validate_request(&request)?;
        self.client
            .fetch(
                GrafanaQueryRequest {
                    ref_id: QUERY_REF_ID,
                    datasource: GrafanaDatasource {
                        kind: "cloudwatch",
                        uid: request.datasource_uid,
                    },
                    query_mode: "Metrics",
                    metric_query_type: 0,
                    metric_editor_mode: 0,
                    region: request.region,
                    namespace: request.namespace,
                    metric_name: request.metric_name,
                    dimensions: request
                        .dimensions
                        .into_iter()
                        .map(|(name, value)| (name, vec![value]))
                        .collect(),
                    statistic: request.statistic.as_grafana_value(),
                    period: request.period.to_string(),
                    match_exact: true,
                    id: "",
                    expression: "",
                },
                range,
            )
            .await
    }
}

fn validate_nonblank(field: &'static str, value: &str) -> MetricsResult<()> {
    if value.trim().is_empty() {
        Err(invalid_request(field, "must not be blank"))
    } else {
        Ok(())
    }
}

fn invalid_request(field: &'static str, requirement: &'static str) -> Report<MetricsError> {
    Report::new(MetricsError::InvalidRequest { field, requirement })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaQueryRequest {
    ref_id: &'static str,
    datasource: GrafanaDatasource,
    query_mode: &'static str,
    metric_query_type: u8,
    metric_editor_mode: u8,
    region: String,
    namespace: String,
    metric_name: String,
    dimensions: BTreeMap<String, Vec<String>>,
    statistic: &'static str,
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::sync::OnceLock;

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

    fn request() -> CloudwatchMetricRequest {
        CloudwatchMetricRequest {
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
            from: timestamp(1_700_000_000),
            to: timestamp(1_700_000_300),
        }
    }

    fn timestamp(seconds: i64) -> time::PrimitiveDateTime {
        convert_to_pdt(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
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

    fn test_runtime() -> &'static tokio::runtime::Runtime {
        static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
        RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
        })
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
        let valid_periods = [1, 5, 10, 20, 30, 60, 300, 86_400];
        for period in valid_periods {
            let mut candidate = request();
            candidate.period = period;
            GrafanaCloudwatch::validate_request(&candidate).unwrap();
        }

        let invalid_cases = [
            ("datasource_uid", 0_u8),
            ("region", 1),
            ("namespace", 2),
            ("metric_name", 3),
            ("period", 4),
        ];
        for (expected_field, case) in invalid_cases {
            let mut candidate = request();
            match case {
                0 => candidate.datasource_uid = "  ".to_owned(),
                1 => candidate.region.clear(),
                2 => candidate.namespace.clear(),
                3 => candidate.metric_name.clear(),
                4 => candidate.period = 61,
                _ => unreachable!(),
            }
            let error = GrafanaCloudwatch::validate_request(&candidate).unwrap_err();
            assert!(matches!(
                error.current_context(),
                MetricsError::InvalidRequest { field, .. } if *field == expected_field
            ));
        }

        let mut blank_dimension = request();
        blank_dimension
            .dimensions
            .insert("blank".to_owned(), " ".to_owned());
        assert!(matches!(
            GrafanaCloudwatch::validate_request(&blank_dimension)
                .unwrap_err()
                .current_context(),
            MetricsError::InvalidRequest {
                field: "dimensions",
                ..
            }
        ));

        let mut too_many_dimensions = request();
        too_many_dimensions.dimensions = (0..31)
            .map(|index| (format!("name-{index}"), "value".to_owned()))
            .collect();
        assert!(GrafanaCloudwatch::validate_request(&too_many_dimensions).is_err());
    }

    #[test]
    fn fetch_posts_the_typed_grafana_cloudwatch_envelope() {
        test_runtime().block_on(async {
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

            let series = provider(&server).fetch(request(), range()).await.unwrap();

            assert_eq!(
                series,
                vec![MetricSeries {
                    points: vec![
                        MetricPoint {
                            timestamp: timestamp(1_700_000_000),
                            value: 1.5,
                        },
                        MetricPoint {
                            timestamp: timestamp(1_700_000_120),
                            value: 3.25,
                        },
                    ],
                }]
            );
        });
    }

    #[test]
    fn successful_no_frame_empty_and_all_null_responses_are_empty_data() {
        test_runtime().block_on(async {
            let bodies = [
            json!({"results": {"A": {}}}),
            json!({"results": {"A": {"frames": []}}}),
            json!({"results": {"A": {"frames": [{"data": {"values": []}}]}}}),
            json!({"results": {"A": {"frames": [{"data": {"values": [[1700000000000_i64], [null]]}}]}}}),
        ];
            for (index, body) in bodies.into_iter().enumerate() {
                let server = MockServer::start().await;
                mount(&server, 200, body).await;
                let series = provider(&server).fetch(request(), range()).await.unwrap();
                if index < 2 {
                    assert!(series.is_empty());
                } else {
                    assert_eq!(series, vec![MetricSeries { points: Vec::new() }]);
                }
            }
        });
    }

    #[test]
    fn malformed_frames_are_not_reported_as_empty_data() {
        test_runtime().block_on(async {
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
                let error = provider(&server)
                    .fetch(request(), range())
                    .await
                    .unwrap_err();
                assert!(
                    matches!(error.current_context(), MetricsError::MalformedResponse),
                    "malformed case `{case}` returned context `{}`; full report: {error:?}",
                    error.current_context()
                );
            }
        });
    }

    #[test]
    fn non_finite_json_number_is_a_malformed_response() {
        test_runtime().block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                r#"{"results":{"A":{"frames":[{"data":{"values":[[1700000000000],[1e400]]}}]}}}"#,
                "application/json",
            ))
            .mount(&server)
            .await;

            let error = provider(&server)
                .fetch(request(), range())
                .await
                .unwrap_err();
            assert!(matches!(
                error.current_context(),
                MetricsError::MalformedResponse
            ));
        });
    }

    #[test]
    fn query_error_wins_over_http_status_and_redacts_the_token() {
        test_runtime().block_on(async {
            let server = MockServer::start().await;
            mount(
                &server,
                400,
                json!({"results": {"A": {"error": format!("credential {TOKEN} rejected")}}}),
            )
            .await;

            let error = provider(&server)
                .fetch(request(), range())
                .await
                .unwrap_err();
            assert!(matches!(
                error.current_context(),
                MetricsError::QueryRefused { reason }
                    if reason == "credential [REDACTED] rejected" && !reason.contains(TOKEN)
            ));
            assert!(!format!("{error:?}").contains(TOKEN));
        });
    }

    #[test]
    fn authorization_and_other_non_success_statuses_are_typed_failures() {
        test_runtime().block_on(async {
            for (status, unauthorized) in [(401, true), (502, false)] {
                let server = MockServer::start().await;
                mount(&server, status, json!({"message": "failed"})).await;
                let error = provider(&server)
                    .fetch(request(), range())
                    .await
                    .unwrap_err();
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
        });
    }

    #[test]
    fn fetch_validates_before_sending() {
        test_runtime().block_on(async {
            let server = MockServer::start().await;
            let mut invalid = request();
            invalid.period = 90;

            let error = provider(&server).fetch(invalid, range()).await.unwrap_err();

            assert!(matches!(
                error.current_context(),
                MetricsError::InvalidRequest {
                    field: "period",
                    ..
                }
            ));
            assert!(server.received_requests().await.unwrap().is_empty());
        });
    }
}
