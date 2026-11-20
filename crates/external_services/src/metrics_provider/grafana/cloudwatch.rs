use std::collections::BTreeMap;

use hyperswitch_interfaces::types::Proxy;
use serde::Serialize;

use super::{
    epoch_millis, GrafanaClient, GrafanaConfig, GrafanaDatasource, GrafanaMetricsResponse,
    QUERY_REF_ID,
};
use crate::metrics_provider::{
    MetricRange, MetricSeries, MetricsError, MetricsProvider, MetricsResult,
};

#[derive(Debug, Clone)]
pub struct GrafanaCloudwatch {
    client: GrafanaClient,
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

impl GrafanaCloudwatch {
    pub fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self {
            client: GrafanaClient { config, proxy },
        }
    }
}

#[async_trait::async_trait]
impl MetricsProvider for GrafanaCloudwatch {
    type MetricsQuery = CloudwatchMetricsQuery;
    type MetricsRequest = CloudwatchMetricsRequest;
    type MetricsResponse = GrafanaMetricsResponse;
    type MetricsError = MetricsError;

    fn build_request(
        query: Self::MetricsQuery,
        range: MetricRange,
    ) -> MetricsResult<Self::MetricsRequest> {
        Ok(CloudwatchMetricsRequest {
            from: epoch_millis(range.from),
            to: epoch_millis(range.to),
            queries: [GrafanaCloudwatchQuery {
                ref_id: QUERY_REF_ID,
                datasource: GrafanaDatasource {
                    kind: "cloudwatch",
                    uid: query.datasource_uid,
                },
                query_mode: QueryMode::Metrics,
                metric_query_type: MetricQueryType::Search,
                metric_editor_mode: MetricEditorMode::Builder,
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
            }],
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
        let (status, response) = self
            .client
            .execute::<Self::MetricsRequest, Self::MetricsResponse>(request)
            .await?;
        self.handle_response(response, status)
    }
}

#[derive(Debug, Serialize)]
enum QueryMode {
    Metrics,
}

#[derive(Debug)]
enum MetricQueryType {
    Search,
}

impl Serialize for MetricQueryType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Search => serializer.serialize_u8(0),
        }
    }
}

#[derive(Debug)]
enum MetricEditorMode {
    Builder,
}

impl Serialize for MetricEditorMode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Builder => serializer.serialize_u8(0),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaCloudwatchQuery {
    ref_id: &'static str,
    datasource: GrafanaDatasource,
    query_mode: QueryMode,
    metric_query_type: MetricQueryType,
    metric_editor_mode: MetricEditorMode,
    region: String,
    namespace: String,
    metric_name: String,
    dimensions: BTreeMap<String, Vec<String>>,
    statistic: CloudwatchStatistic,
    period: String,
    match_exact: bool,
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
                base_url: Some(server.uri().parse().unwrap()),
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
                    "matchExact": true
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
                labels: Default::default(),
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
    async fn missing_grafana_url_is_rejected() {
        let provider = GrafanaCloudwatch::new(GrafanaConfig::default(), Proxy::default());
        let request = GrafanaCloudwatch::build_request(query(), range()).unwrap();
        let error = provider.fetch_metrics(request).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            MetricsError::MissingGrafanaBaseUrl
        ));
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
                assert_eq!(
                    series,
                    vec![MetricSeries {
                        points: Vec::new(),
                        labels: Default::default()
                    }]
                );
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
            (
                "malformed response echoing a credential",
                json!({"results": {"A": {"error": format!("credential {TOKEN}"), "frames": "invalid"}}}),
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
            assert!(!format!("{error:?}").contains(TOKEN));
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
        let mut unsupported = query();
        unsupported.period = 90;
        let request = GrafanaCloudwatch::build_request(unsupported, range()).unwrap();
        let error = provider(&server).fetch_metrics(request).await.unwrap_err();
        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["queries"][0]["period"], "90");
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
}
