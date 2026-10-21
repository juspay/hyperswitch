use std::collections::HashMap;

use common_utils::{
    date_time::convert_to_pdt,
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::{Report, ResultExt};
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use hyperswitch_masking::{Mask as _, Maskable, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};
use url::Url;

use crate::http_client;

const QUERY_TIMEOUT_SECONDS: u64 = 10;

pub type GrafanaResult<T> = CustomResult<T, GrafanaError>;

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

    pub async fn fetch<Q>(
        &self,
        query: Q,
        from: PrimitiveDateTime,
        to: PrimitiveDateTime,
    ) -> GrafanaResult<Vec<MetricSeries>>
    where
        Q: Serialize + Send + Sync,
    {
        let payload = serde_json::to_value(QueryEnvelope {
            from: epoch_milliseconds(from),
            to: epoch_milliseconds(to),
            queries: [query],
        })
        .change_context(GrafanaError::RequestSerialization)?;
        let request = RequestBuilder::new()
            .method(Method::Post)
            .url(&self.query_url())
            .attach_default_headers()
            .headers(self.headers())
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
                return Err(error.change_context(GrafanaError::Timeout));
            }
            Err(error) => return Err(error.change_context(GrafanaError::Transport)),
        };
        let status = response.status();
        let body = response
            .text()
            .await
            .change_context(GrafanaError::MalformedResponse)?;
        parse_response(&body, status.as_u16(), status.is_success())
    }

    fn query_url(&self) -> String {
        format!(
            "{}/api/ds/query",
            self.config.base_url.as_str().trim_end_matches('/')
        )
    }

    fn headers(&self) -> Vec<(String, Maskable<String>)> {
        vec![
            (
                http::header::AUTHORIZATION.to_string(),
                format!("Bearer {}", self.config.api_token.peek()).into_masked(),
            ),
            (
                http::header::CONTENT_TYPE.to_string(),
                mime::APPLICATION_JSON.essence_str().to_owned().into(),
            ),
        ]
    }
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

#[derive(Debug, thiserror::Error)]
pub enum GrafanaError {
    #[error("failed to serialize the Grafana query")]
    RequestSerialization,
    #[error("failed to send the request to Grafana")]
    Transport,
    #[error("the Grafana request timed out")]
    Timeout,
    #[error("Grafana responded with HTTP status {status}")]
    HttpStatus { status: u16 },
    #[error("Grafana refused the query: {reason}")]
    QueryRefused { reason: String },
    #[error("could not interpret the Grafana response")]
    MalformedResponse,
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

fn parse_response(
    body: &str,
    status: u16,
    status_is_success: bool,
) -> GrafanaResult<Vec<MetricSeries>> {
    let response: QueryResponse = match serde_json::from_str(body) {
        Ok(response) => response,
        Err(error) if !status_is_success => {
            return Err(Report::from(error).change_context(GrafanaError::HttpStatus { status }));
        }
        Err(error) => {
            return Err(Report::from(error).change_context(GrafanaError::MalformedResponse));
        }
    };
    let mut results = response.results;
    if let Some(reason) = results
        .as_mut()
        .and_then(|results| results.values_mut().find_map(|result| result.error.take()))
    {
        return Err(Report::new(GrafanaError::QueryRefused { reason }));
    }
    if !status_is_success {
        return Err(Report::new(GrafanaError::HttpStatus { status }));
    }
    let mut results = results.ok_or_else(|| malformed_response("response is missing results"))?;
    let result = results
        .remove("A")
        .ok_or_else(|| malformed_response("response is missing query result A"))?;
    result
        .frames
        .into_iter()
        .flatten()
        .map(parse_frame)
        .collect()
}

fn parse_frame(frame: Frame) -> GrafanaResult<MetricSeries> {
    let values = frame
        .data
        .ok_or_else(|| malformed_response("frame is missing data"))?
        .values
        .ok_or_else(|| malformed_response("frame data is missing values"))?;
    if values.is_empty() {
        return Ok(MetricSeries { points: Vec::new() });
    }
    if values.len() != 2 {
        return Err(malformed_response(format!(
            "frame contains {} value arrays; expected 0 or 2",
            values.len()
        )));
    }
    let mut values = values.into_iter();
    let timestamps = match values.next() {
        Some(serde_json::Value::Array(values)) => values,
        _ => return Err(malformed_response("timestamps are not an array")),
    };
    let metric_values = match values.next() {
        Some(serde_json::Value::Array(values)) => values,
        _ => return Err(malformed_response("metric values are not an array")),
    };
    if timestamps.len() != metric_values.len() {
        return Err(malformed_response(format!(
            "frame contains {} timestamps and {} metric values",
            timestamps.len(),
            metric_values.len()
        )));
    }
    let mut points = Vec::new();
    for (timestamp, value) in timestamps.into_iter().zip(metric_values) {
        if value.is_null() {
            continue;
        }
        let value = value
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| malformed_response("frame contains an invalid metric value"))?;
        let milliseconds = timestamp
            .as_i64()
            .ok_or_else(|| malformed_response("frame contains an invalid timestamp"))?;
        let timestamp =
            OffsetDateTime::from_unix_timestamp_nanos(i128::from(milliseconds) * 1_000_000)
                .change_context(GrafanaError::MalformedResponse)
                .attach_printable("frame contains an out-of-range timestamp")?;
        points.push(MetricPoint {
            timestamp: convert_to_pdt(timestamp),
            value,
        });
    }
    Ok(MetricSeries { points })
}

fn malformed_response(detail: impl Into<String>) -> Report<GrafanaError> {
    Report::new(GrafanaError::MalformedResponse).attach_printable(detail.into())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Serialize;
    use serde_json::json;
    use wiremock::{
        matchers::{body_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    const TOKEN: &str = "grafana-test-token";

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TestQuery {
        ref_id: &'static str,
        datasource: TestDatasource,
        query_mode: &'static str,
        metric_query_type: u8,
        metric_editor_mode: u8,
        region: &'static str,
        namespace: &'static str,
        metric_name: &'static str,
        dimensions: BTreeMap<&'static str, Vec<&'static str>>,
        statistic: &'static str,
        period: &'static str,
        match_exact: bool,
        id: &'static str,
        expression: &'static str,
    }

    #[derive(Debug, Serialize)]
    struct TestDatasource {
        #[serde(rename = "type")]
        kind: &'static str,
        uid: &'static str,
    }

    fn query() -> TestQuery {
        TestQuery {
            ref_id: "A",
            datasource: TestDatasource {
                kind: "cloudwatch",
                uid: "cloudwatch",
            },
            query_mode: "Metrics",
            metric_query_type: 0,
            metric_editor_mode: 0,
            region: "us-east-1",
            namespace: "AWS/EC2",
            metric_name: "CPUUtilization",
            dimensions: BTreeMap::from([("InstanceId", vec!["i-0123"])]),
            statistic: "Average",
            period: "60",
            match_exact: true,
            id: "",
            expression: "",
        }
    }

    fn client(server: &MockServer) -> TestResult<GrafanaClient> {
        Ok(GrafanaClient::new(
            GrafanaConfig {
                base_url: Url::parse(&server.uri())?,
                api_token: Secret::new(TOKEN.to_owned()),
            },
            Proxy::default(),
        ))
    }

    fn timestamp(seconds: i64) -> TestResult<PrimitiveDateTime> {
        Ok(convert_to_pdt(OffsetDateTime::from_unix_timestamp(
            seconds,
        )?))
    }

    async fn mount_response(server: &MockServer, status: u16, body: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(server)
            .await;
    }

    async fn fetch(server: &MockServer) -> TestResult<GrafanaResult<Vec<MetricSeries>>> {
        let client = client(server)?;
        let from = convert_to_pdt(OffsetDateTime::UNIX_EPOCH);
        let to = from + time::Duration::seconds(60);
        Ok(client.fetch(query(), from, to).await)
    }

    #[tokio::test]
    async fn sends_one_query_in_the_common_envelope() -> TestResult {
        let server = MockServer::start().await;
        let from = timestamp(1_700_000_000)?;
        let to = timestamp(1_700_000_060)?;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
            .and(header("content-type", "application/json"))
            .and(body_json(json!({
                "from": "1700000000000",
                "to": "1700000060000",
                "queries": [{
                    "refId": "A",
                    "datasource": {"type": "cloudwatch", "uid": "cloudwatch"},
                    "queryMode": "Metrics",
                    "metricQueryType": 0,
                    "metricEditorMode": 0,
                    "region": "us-east-1",
                    "namespace": "AWS/EC2",
                    "metricName": "CPUUtilization",
                    "dimensions": {"InstanceId": ["i-0123"]},
                    "statistic": "Average",
                    "period": "60",
                    "matchExact": true,
                    "id": "",
                    "expression": ""
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": {"A": {"frames": []}}
            })))
            .mount(&server)
            .await;

        let result = client(&server)?.fetch(query(), from, to).await;
        assert!(result.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn refused_query_under_empty_key_keeps_the_reason() -> TestResult {
        let server = MockServer::start().await;
        mount_response(
            &server,
            400,
            json!({"results": {"": {
                "error": "metric request error: X",
                "errorSource": "downstream",
                "status": 500
            }}}),
        )
        .await;

        let result = fetch(&server).await?;
        let Err(error) = result else {
            return Err("expected a refused query error".into());
        };
        assert!(matches!(
            error.current_context(),
            GrafanaError::QueryRefused { reason } if reason.contains("metric request error: X")
        ));
        Ok(())
    }

    #[tokio::test]
    async fn unauthorized_is_an_http_status_error() -> TestResult {
        let server = MockServer::start().await;
        mount_response(&server, 401, json!({"message": "Unauthorized"})).await;

        let result = fetch(&server).await?;
        let Err(error) = result else {
            return Err("expected an HTTP status error".into());
        };
        assert!(matches!(
            error.current_context(),
            GrafanaError::HttpStatus { status: 401 }
        ));
        Ok(())
    }

    #[tokio::test]
    async fn no_frames_returns_no_series() -> TestResult {
        let server = MockServer::start().await;
        mount_response(&server, 200, json!({"results": {"A": {"frames": []}}})).await;

        let result = fetch(&server).await?;
        let Ok(series) = result else {
            return Err("expected an empty series result".into());
        };
        assert!(series.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn absent_frames_returns_no_series() -> TestResult {
        let server = MockServer::start().await;
        mount_response(&server, 200, json!({"results": {"A": {}}})).await;

        let result = fetch(&server).await?;
        let Ok(series) = result else {
            return Err("expected an empty series result".into());
        };
        assert!(series.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn empty_frame_values_returns_no_points() -> TestResult {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {"A": {"frames": [{"data": {"values": []}}]}}}),
        )
        .await;

        let result = fetch(&server).await?;
        let Ok(series) = result else {
            return Err("expected one empty series".into());
        };
        let Some(series) = series.first() else {
            return Err("expected one series".into());
        };
        assert!(series.points.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn null_values_are_dropped() -> TestResult {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {"A": {"frames": [{"data": {"values": [
                [1700000000000_i64, 1700000060000_i64],
                [null, 42.5]
            ]}}]}}}),
        )
        .await;

        let result = fetch(&server).await?;
        let Ok(series) = result else {
            return Err("expected one parsed series".into());
        };
        let Some(series) = series.first() else {
            return Err("expected one series".into());
        };
        assert_eq!(series.points.len(), 1);
        let Some(point) = series.points.first() else {
            return Err("expected one point".into());
        };
        assert_eq!(point.value, 42.5);
        assert_eq!(point.timestamp, timestamp(1_700_000_060)?);
        Ok(())
    }

    #[tokio::test]
    async fn two_frames_return_two_series() -> TestResult {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {"A": {"frames": [
                {"data": {"values": []}},
                {"data": {"values": []}}
            ]}}}),
        )
        .await;

        let result = fetch(&server).await?;
        let Ok(series) = result else {
            return Err("expected two series".into());
        };
        assert_eq!(series.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn mismatched_point_counts_are_malformed() -> TestResult {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {"A": {"frames": [{"data": {"values": [
                [1700000000000_i64, 1700000060000_i64],
                [42.5]
            ]}}]}}}),
        )
        .await;

        let result = fetch(&server).await?;
        let Err(error) = result else {
            return Err("expected mismatched point counts".into());
        };
        assert!(matches!(
            error.current_context(),
            GrafanaError::MalformedResponse
        ));
        assert!(format!("{error:?}").contains("2 timestamps and 1 metric values"));
        Ok(())
    }

    #[tokio::test]
    async fn non_json_body_is_malformed() -> TestResult {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let result = fetch(&server).await?;
        let Err(error) = result else {
            return Err("expected a malformed response".into());
        };
        assert!(matches!(
            error.current_context(),
            GrafanaError::MalformedResponse
        ));
        Ok(())
    }

    #[tokio::test]
    async fn non_json_server_error_keeps_http_status_context() -> TestResult {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(503).set_body_string("not json"))
            .mount(&server)
            .await;

        let result = fetch(&server).await?;
        let Err(error) = result else {
            return Err("expected an HTTP status error".into());
        };
        assert!(matches!(
            error.current_context(),
            GrafanaError::HttpStatus { status: 503 }
        ));
        assert!(error.contains::<serde_json::Error>());
        Ok(())
    }

    #[tokio::test]
    async fn missing_response_fields_are_malformed() -> TestResult {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {"A": {"frames": [{"data": {}}]}}}),
        )
        .await;

        let result = fetch(&server).await?;
        let Err(error) = result else {
            return Err("expected a missing values error".into());
        };
        assert!(matches!(
            error.current_context(),
            GrafanaError::MalformedResponse
        ));
        Ok(())
    }
}
