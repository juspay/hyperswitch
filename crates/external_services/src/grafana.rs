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

pub const QUERY_REF_ID: &str = "A";

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
    timeout_seconds: u64,
}

impl GrafanaClient {
    pub fn new(config: GrafanaConfig, proxy: Proxy) -> Self {
        Self {
            config,
            proxy,
            timeout_seconds: QUERY_TIMEOUT_SECONDS,
        }
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
        let response =
            match http_client::send_request(&self.proxy, request, Some(self.timeout_seconds)).await
            {
                Ok(response) => response,
                Err(error)
                    if error.current_context() == &HttpClientError::RequestTimeoutReceived =>
                {
                    return Err(error.change_context(GrafanaError::Timeout));
                }
                Err(error) => return Err(error.change_context(GrafanaError::Transport)),
            };
        let status = response.status();
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => {
                return Err(Report::from(error).change_context(GrafanaError::Timeout));
            }
            Err(error) => {
                return Err(Report::from(error).change_context(GrafanaError::Transport));
            }
        };
        parse_response(&body, status)
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

fn parse_response(body: &str, status: reqwest::StatusCode) -> GrafanaResult<Vec<MetricSeries>> {
    let response: QueryResponse = match serde_json::from_str(body) {
        Ok(response) => response,
        Err(error) if !status.is_success() => {
            return Err(
                Report::from(error).change_context(GrafanaError::HttpStatus {
                    status: status.as_u16(),
                }),
            );
        }
        Err(error) => {
            return Err(Report::from(error).change_context(GrafanaError::MalformedResponse));
        }
    };
    let results = response.results;
    if let Some(reason) = results
        .as_ref()
        .and_then(|results| results.values().find_map(|result| result.error.clone()))
    {
        return Err(Report::new(GrafanaError::QueryRefused { reason }));
    }
    if !status.is_success() {
        return Err(Report::new(GrafanaError::HttpStatus {
            status: status.as_u16(),
        }));
    }
    let result = results
        .ok_or_else(|| malformed_response("response is missing results"))?
        .into_iter()
        .find_map(|(ref_id, result)| (ref_id == QUERY_REF_ID).then_some(result))
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

fn parse_frame(frame: Frame) -> GrafanaResult<MetricSeries> {
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
                    .change_context(GrafanaError::MalformedResponse)
                    .attach_printable("frame contains an out-of-range timestamp")?;
            Ok(MetricPoint {
                timestamp: convert_to_pdt(timestamp),
                value,
            })
        })
        .collect::<GrafanaResult<Vec<_>>>()?;
    Ok(MetricSeries { points })
}

fn malformed_response(detail: impl Into<String>) -> Report<GrafanaError> {
    Report::new(GrafanaError::MalformedResponse).attach_printable(detail.into())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::time::Duration as StdDuration;

    use error_stack::{AttachmentKind, FrameKind};
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use wiremock::{
        matchers::{body_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    const TOKEN: &str = "grafana-test-token";

    fn query() -> serde_json::Value {
        json!({"refId": QUERY_REF_ID})
    }

    fn client(server: &MockServer) -> GrafanaClient {
        client_at(server.uri())
    }

    fn client_at(base_url: String) -> GrafanaClient {
        GrafanaClient::new(
            GrafanaConfig {
                base_url: Url::parse(&base_url).unwrap(),
                api_token: Secret::new(TOKEN.to_owned()),
            },
            Proxy::default(),
        )
    }

    fn timestamp(seconds: i64) -> PrimitiveDateTime {
        convert_to_pdt(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
    }

    async fn mount_response(server: &MockServer, status: u16, body: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(server)
            .await;
    }

    async fn fetch(server: &MockServer) -> GrafanaResult<Vec<MetricSeries>> {
        let from = convert_to_pdt(OffsetDateTime::UNIX_EPOCH);
        let to = from + time::Duration::seconds(60);
        client(server).fetch(query(), from, to).await
    }

    fn has_printable(error: &Report<GrafanaError>, expected: &str) -> bool {
        error.frames().any(|frame| {
            matches!(
                frame.kind(),
                FrameKind::Attachment(AttachmentKind::Printable(attachment))
                    if attachment.to_string().contains(expected)
            )
        })
    }

    #[tokio::test]
    async fn sends_one_query_in_the_common_envelope() {
        let server = MockServer::start().await;
        let from = timestamp(1_700_000_000);
        let to = timestamp(1_700_000_060);
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
            .and(header("content-type", "application/json"))
            .and(body_json(json!({
                "from": "1700000000000",
                "to": "1700000060000",
                "queries": [{"refId": QUERY_REF_ID}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": {QUERY_REF_ID: {"frames": []}}
            })))
            .mount(&server)
            .await;

        assert!(client(&server).fetch(query(), from, to).await.is_ok());
    }

    #[tokio::test]
    async fn refused_query_under_empty_key_keeps_the_reason() {
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

        let error = fetch(&server).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            GrafanaError::QueryRefused { reason } if reason == "metric request error: X"
        ));
    }

    #[tokio::test]
    async fn unauthorized_is_an_http_status_error() {
        let server = MockServer::start().await;
        mount_response(&server, 401, json!({"message": "Unauthorized"})).await;

        let error = fetch(&server).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            GrafanaError::HttpStatus { status: 401 }
        ));
    }

    #[tokio::test]
    async fn no_frames_returns_no_series() {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {QUERY_REF_ID: {"frames": []}}}),
        )
        .await;

        assert!(fetch(&server).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn absent_frames_returns_no_series() {
        let server = MockServer::start().await;
        mount_response(&server, 200, json!({"results": {QUERY_REF_ID: {}}})).await;

        assert!(fetch(&server).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn empty_frame_values_returns_no_points() {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {QUERY_REF_ID: {"frames": [{"data": {"values": []}}]}}}),
        )
        .await;

        let series = fetch(&server).await.unwrap();
        assert_eq!(series.len(), 1);
        assert!(series[0].points.is_empty());
    }

    #[tokio::test]
    async fn null_values_are_dropped() {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {QUERY_REF_ID: {"frames": [{"data": {"values": [
                [1700000000000_i64, 1700000060000_i64],
                [null, 42.5]
            ]}}]}}}),
        )
        .await;

        let series = fetch(&server).await.unwrap();
        assert_eq!(series[0].points.len(), 1);
        assert_eq!(series[0].points[0].value, 42.5);
        assert_eq!(series[0].points[0].timestamp, timestamp(1_700_000_060));
    }

    #[tokio::test]
    async fn two_frames_return_two_series() {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {QUERY_REF_ID: {"frames": [
                {"data": {"values": []}},
                {"data": {"values": []}}
            ]}}}),
        )
        .await;

        assert_eq!(fetch(&server).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn mismatched_point_counts_are_malformed() {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {QUERY_REF_ID: {"frames": [{"data": {"values": [
                [1700000000000_i64, 1700000060000_i64],
                [42.5]
            ]}}]}}}),
        )
        .await;

        let error = fetch(&server).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            GrafanaError::MalformedResponse
        ));
        assert!(has_printable(&error, "2 timestamps and 1 metric values"));
    }

    #[tokio::test]
    async fn non_json_body_is_malformed() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let error = fetch(&server).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            GrafanaError::MalformedResponse
        ));
    }

    #[tokio::test]
    async fn non_json_server_error_keeps_http_status_context() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(ResponseTemplate::new(503).set_body_string("not json"))
            .mount(&server)
            .await;

        let error = fetch(&server).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            GrafanaError::HttpStatus { status: 503 }
        ));
        assert!(error.contains::<serde_json::Error>());
    }

    #[tokio::test]
    async fn missing_response_fields_are_malformed() {
        let server = MockServer::start().await;
        mount_response(
            &server,
            200,
            json!({"results": {QUERY_REF_ID: {"frames": [{"data": {}}]}}}),
        )
        .await;

        let error = fetch(&server).await.unwrap_err();
        assert!(matches!(
            error.current_context(),
            GrafanaError::MalformedResponse
        ));
    }

    #[tokio::test]
    async fn body_read_failure_is_transport() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{}")
                .await
                .unwrap();
        });
        let from = convert_to_pdt(OffsetDateTime::UNIX_EPOCH);
        let result = client_at(format!("http://{address}"))
            .fetch(query(), from, from + time::Duration::seconds(60))
            .await;

        let error = result.unwrap_err();
        assert!(matches!(error.current_context(), GrafanaError::Transport));
    }

    #[tokio::test]
    async fn request_timeout_has_timeout_context() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/ds/query"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(StdDuration::from_secs(2))
                    .set_body_json(json!({"results": {QUERY_REF_ID: {"frames": []}}})),
            )
            .mount(&server)
            .await;

        let client = GrafanaClient {
            timeout_seconds: 1,
            ..client(&server)
        };
        let error = client
            .fetch(query(), timestamp(0), timestamp(60))
            .await
            .unwrap_err();
        assert!(matches!(error.current_context(), GrafanaError::Timeout));
    }
}
