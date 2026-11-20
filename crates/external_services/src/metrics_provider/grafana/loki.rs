use error_stack::ResultExt;
use hyperswitch_interfaces::types::Proxy;
use serde::Serialize;
use time::Duration;

use super::{
    epoch_millis, GrafanaClient, GrafanaConfig, GrafanaDatasource, GrafanaMetricsResponse,
    QUERY_REF_ID,
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
}

impl GrafanaLoki {
    pub fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self {
            client: GrafanaClient { config, proxy },
        }
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
        Ok(LokiMetricsRequest {
            from: epoch_millis(range.from),
            to: epoch_millis(range.to),
            queries: [GrafanaLokiQuery {
                ref_id: QUERY_REF_ID,
                datasource: GrafanaDatasource {
                    kind: "loki",
                    uid: query.datasource_uid,
                },
                expr: query.expr,
                query_type: QueryType::Instant,
                interval_ms: period * 1000,
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
        let [query] = &request.queries;
        let period = Duration::milliseconds(query.interval_ms);
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
    Instant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaLokiQuery {
    ref_id: &'static str,
    datasource: GrafanaDatasource,
    expr: String,
    query_type: QueryType,
    interval_ms: i64,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common_utils::date_time::convert_to_pdt;
    use hyperswitch_masking::Secret;
    use serde_json::json;
    use time::{OffsetDateTime, PrimitiveDateTime};
    use wiremock::{
        matchers::{body_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn at(seconds: i64) -> PrimitiveDateTime {
        convert_to_pdt(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
    }

    #[tokio::test]
    async fn fetch_posts_an_instant_query_and_stamps_the_point_at_the_minute_start() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .and(header("authorization", "Bearer loki-test-token"))
            .and(body_json(json!({
                "from": "1700000240000",
                "to": "1700000300000",
                "queries": [{
                    "refId": "A",
                    "datasource": {"type": "loki", "uid": "loki-main"},
                    "expr": "sum(count_over_time({app=\"router\"} |= \"error\" [1m]))",
                    "queryType": "instant",
                    "intervalMs": 60000
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": {"A": {"frames": [{
                    "schema": {"fields": [{"name": "Time"}, {"name": "Value", "labels": {"app": "router"}}]},
                    "data": {"values": [
                    [1_700_000_300_000_i64],
                    [5.0]
                ]}}]}}
            })))
            .mount(&server)
            .await;
        let provider = GrafanaLoki::new(
            GrafanaConfig {
                base_url: Some(server.uri().parse().unwrap()),
                ui_url: None,
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
                from: at(1_700_000_240),
                to: at(1_700_000_300),
            },
        )
        .unwrap();
        assert_eq!(
            provider.fetch_metrics(request).await.unwrap(),
            vec![MetricSeries {
                labels: BTreeMap::from([("app".to_owned(), "router".to_owned())]),
                points: vec![MetricPoint {
                    timestamp: at(1_700_000_240),
                    value: 5.0,
                }],
            }]
        );
    }
}
