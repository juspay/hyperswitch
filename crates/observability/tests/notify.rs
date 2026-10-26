//! The notify routes, exercised through actix as a caller would reach them.
//!
//! These go through the real route tree rather than calling handlers directly, because most of what
//! this ticket decided lives *between* the handler and the caller: the guard, the path extractor,
//! the body extractor's rejection shape, and which outcomes are a `200` versus an error.
//!
//! Delivery destinations are local log/recording notifiers. Metric tests use loopback HTTP mocks;
//! no sandbox or external notification service is contacted.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use common_types::alert_rules::AlertQuery;
use error_stack::report;
use external_services::{
    email::no_email::NoEmailClient,
    metrics_provider::grafana::{cloudwatch::GrafanaCloudwatch, GrafanaConfig},
};
use hyperswitch_masking::{PeekInterface, Secret};
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::Store,
    domain::notifier::{
        chat::{
            ChatFileOutcome, ChatFileUpload, ChatNotification, ChatNotifier, ChatOutcome,
            ChatReceipt, LogChatNotifier,
        },
        email::{EmailNotifier, EmailServiceNotifier},
        Registry,
    },
    domain_models::evaluator::{EvaluatorError, RuleDefinition},
    errors::ObservabilityError,
    routes::Alerts,
    settings::Database,
    state::AppState,
};
use serde_json::{json, Value};
use time::macros::datetime;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{mpsc, Semaphore},
    time::{timeout, Duration},
};

const API_KEY: &str = "test_internal_key";
const CHAT: &str = "sr_alerts";
const EMAIL: &str = "oncall";

async fn state() -> AppState {
    state_with_max(25 * 1024 * 1024).await
}

async fn state_with_max(max_upload_bytes: usize) -> AppState {
    let conf = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY },
        "chat": { "max_upload_bytes": max_upload_bytes }
    }))
    .expect("the test configuration should deserialize");

    let chat: Arc<dyn ChatNotifier> = Arc::new(LogChatNotifier::new(CHAT.to_owned()));
    // The real notifier over `NoEmailClient`, which accepts and logs. Exercises the composition
    // path rather than a stand-in, and needs no credentials.
    let email: Arc<dyn EmailNotifier> = Arc::new(EmailServiceNotifier::new(
        EMAIL.to_owned(),
        Arc::new(Box::new(NoEmailClient::create().await)),
        serde_json::from_value(json!("oncall@example.com")).expect("a valid recipient"),
        None,
    ));

    AppState {
        router_transport: None,
        conf: Arc::new(conf),
        chat: Arc::new(Registry::new(HashMap::from([(CHAT.to_owned(), chat)]))),
        email: Arc::new(Registry::new(HashMap::from([(EMAIL.to_owned(), email)]))),
        store: Arc::new(lazy_store().await),
    }
}

/// A store that never connects. No idle connections are opened at build, and the notify routes
/// never ask for one, so these tests need no database.
async fn lazy_store() -> Store {
    Store::new(&Database {
        username: "unused".to_owned(),
        host: "localhost".to_owned(),
        dbname: "unused".to_owned(),
        min_idle_pool_size: 0,
        ..Default::default()
    })
    .await
    .expect("a pool with no idle connections builds without a database")
}

#[derive(Clone, Copy)]
enum MetricReply {
    Value(f64),
    NoData,
    Failure,
}

#[derive(Debug)]
enum MetricEvent {
    Started(String, Value),
    Completed(String),
}

async fn metric_server(
    expected_requests: usize,
    replies: HashMap<String, MetricReply>,
    gated: bool,
) -> (
    String,
    mpsc::Receiver<MetricEvent>,
    HashMap<String, Arc<Semaphore>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (events_tx, events_rx) = mpsc::channel(expected_requests * 2);
    let gates = replies
        .keys()
        .map(|metric| (metric.clone(), Arc::new(Semaphore::new(0))))
        .collect::<HashMap<_, _>>();
    let server_gates = gates.clone();
    tokio::spawn(async move {
        for _ in 0..expected_requests {
            let (mut stream, _) = listener.accept().await.unwrap();
            let replies = replies.clone();
            let gates = server_gates.clone();
            let events = events_tx.clone();
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                let body_start = loop {
                    let read = stream.read(&mut buffer).await.unwrap();
                    assert!(read > 0, "metric request closed before headers completed");
                    request.extend_from_slice(&buffer[..read]);
                    if let Some(position) = request.windows(4).position(|part| part == b"\r\n\r\n")
                    {
                        break position + 4;
                    }
                };
                let headers = std::str::from_utf8(&request[..body_start]).unwrap();
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                while request.len() - body_start < content_length {
                    let read = stream.read(&mut buffer).await.unwrap();
                    assert!(read > 0, "metric request closed before body completed");
                    request.extend_from_slice(&buffer[..read]);
                }
                let body: Value =
                    serde_json::from_slice(&request[body_start..body_start + content_length])
                        .unwrap();
                let metric = body["queries"][0]["metricName"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                let _ = events
                    .send(MetricEvent::Started(metric.clone(), body.clone()))
                    .await;
                if gated {
                    gates[&metric].acquire().await.unwrap().forget();
                }
                let (status, response) = match replies[&metric] {
                    MetricReply::Value(value) => {
                        let to = body["to"].as_str().unwrap().parse::<i64>().unwrap();
                        let period = body["queries"][0]["period"]
                            .as_str()
                            .unwrap()
                            .parse::<i64>()
                            .unwrap();
                        let timestamp = to - period * 1000;
                        (
                            "200 OK",
                            json!({"results":{"A":{"frames":[{"data":{"values":[[timestamp],[value]]}}]}}}),
                        )
                    }
                    MetricReply::NoData => ("200 OK", json!({"results":{"A":{"frames":[]}}})),
                    MetricReply::Failure => (
                        "500 Internal Server Error",
                        json!({"message":"synthetic failure"}),
                    ),
                };
                let bytes = serde_json::to_vec(&response).unwrap();
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                            bytes.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
                stream.write_all(&bytes).await.unwrap();
                stream.shutdown().await.unwrap();
                let _ = events.send(MetricEvent::Completed(metric)).await;
            });
        }
    });
    (address, events_rx, gates)
}

#[derive(Clone, Copy, Debug)]
enum ChatReply {
    Delivered,
    Refused,
    Failed,
}

#[derive(Debug)]
struct RecordingChatNotifier {
    reply: ChatReply,
    calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl ChatNotifier for RecordingChatNotifier {
    async fn notify(
        &self,
        notification: ChatNotification,
    ) -> observability::errors::ObservabilityApiResult<ChatOutcome> {
        self.calls
            .lock()
            .unwrap()
            .push(notification.text.peek().clone());
        match self.reply {
            ChatReply::Delivered => Ok(observability::domain::notifier::Outcome::Delivered(
                ChatReceipt {
                    message_id: Some("synthetic-message".to_owned()),
                },
            )),
            ChatReply::Refused => Ok(observability::domain::notifier::Outcome::Refused(
                observability::domain::notifier::Refusal::new("synthetic_refusal"),
            )),
            ChatReply::Failed => Err(report!(ObservabilityError::ProviderUnavailable {
                destination: "synthetic".to_owned(),
            })),
        }
    }

    async fn upload_file(
        &self,
        _upload: ChatFileUpload,
    ) -> observability::errors::ObservabilityApiResult<ChatFileOutcome> {
        unreachable!("evaluator tests never upload files")
    }
}

fn alert_rule(id: &str, metric: &str, destination: &str, threshold: f64) -> Value {
    json!({
        "id": id,
        "name": id,
        "config": {
            "interval_seconds": 60,
            "query": {
                "datasource": "grafana_cloudwatch",
                "region": "us-east-1",
                "namespace": "Synthetic/Test",
                "metric_name": metric,
                "statistic": "Sum",
                "period": 60,
                "end_offset_seconds": 180
            },
            "thresholds": {"severities": {"critical": {
                "threshold": threshold,
                "comparison_operator": "GreaterThanOrEqualToThreshold",
                "description": "synthetic local test",
                "destination": destination
            }}}
        }
    })
}

async fn evaluator_state(
    base_url: &str,
    rules: Vec<Value>,
    destinations: &[(&str, ChatReply)],
) -> (AppState, HashMap<String, Arc<Mutex<Vec<String>>>>) {
    let conf: observability::Settings = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY },
        "alert": {
            "grafana": { "base_url": base_url, "api_token": "synthetic-local-token" },
            "datasources": { "grafana_cloudwatch": { "uid": "synthetic-cloudwatch" } },
            "rules": serde_json::to_string(&rules).unwrap()
        }
    }))
    .unwrap();
    let mut calls = HashMap::new();
    let chat = destinations
        .iter()
        .map(|(destination, reply)| {
            let destination_calls = Arc::new(Mutex::new(Vec::new()));
            calls.insert((*destination).to_owned(), Arc::clone(&destination_calls));
            let notifier: Arc<dyn ChatNotifier> = Arc::new(RecordingChatNotifier {
                reply: *reply,
                calls: destination_calls,
            });
            ((*destination).to_owned(), notifier)
        })
        .collect();
    (
        AppState {
            router_transport: None,
            conf: Arc::new(conf),
            chat: Arc::new(Registry::new(chat)),
            email: Arc::new(Registry::default()),
            store: Arc::new(lazy_store().await),
        },
        calls,
    )
}

async fn event(receiver: &mut mpsc::Receiver<MetricEvent>) -> MetricEvent {
    timeout(Duration::from_secs(2), receiver.recv())
        .await
        .expect("metric mock deadlock")
        .expect("metric mock ended early")
}

async fn call(request: TestRequest) -> (StatusCode, Value) {
    call_with_state(request, state().await).await
}

async fn call_with_state(request: TestRequest, state: AppState) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state))).await;
    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let body = test::read_body(response).await;

    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

fn post(uri: &str, body: Value) -> TestRequest {
    TestRequest::post()
        .uri(uri)
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .set_json(body)
}

fn upload(body: impl Into<Vec<u8>>) -> TestRequest {
    TestRequest::post()
        .uri(&format!("/alerts/chat/upload/{CHAT}"))
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .insert_header(("content-type", "multipart/form-data; boundary=BOUNDARY"))
        .set_payload(actix_web::web::Bytes::from(body.into()))
}

fn upload_body(file: &str) -> String {
    format!(
        "--BOUNDARY\r\nContent-Disposition: form-data; name=\"file\"; filename=\"report.pdf\"\r\nContent-Type: application/pdf\r\n\r\n{file}\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nDaily report\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"reply_to\"\r\n\r\n1.2\r\n--BOUNDARY--\r\n"
    )
}

#[actix_web::test]
async fn cloudwatch_evaluator_uses_the_new_authenticated_path_only() {
    let app = test::init_service(App::new().service(Alerts::server(state().await))).await;

    let authorized = TestRequest::get()
        .uri("/alerts/grafana/cloudwatch/evaluate")
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .to_request();
    assert_eq!(
        test::call_service(&app, authorized).await.status(),
        StatusCode::OK
    );

    let unauthorized = TestRequest::get()
        .uri("/alerts/grafana/cloudwatch/evaluate")
        .to_request();
    assert_eq!(
        test::call_service(&app, unauthorized).await.status(),
        StatusCode::UNAUTHORIZED
    );

    let legacy = TestRequest::get()
        .uri("/alerts/grafana/evaluate")
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .to_request();
    assert_eq!(
        test::call_service(&app, legacy).await.status(),
        StatusCode::NOT_FOUND
    );
}

// The dependency's cached HTTP client must not outlive the runtime that owns its dispatch tasks.
static EVALUATOR_RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("evaluator test runtime must start")
});

#[test]
fn evaluator_uses_complete_windows_and_rejects_invalid_time_ranges() {
    EVALUATOR_RUNTIME.block_on(async {
        let cases = [
            (
                60,
                180,
                datetime!(2026-03-18 10:08:00),
                datetime!(2026-03-18 10:01:00),
                datetime!(2026-03-18 10:06:00),
            ),
            (
                300,
                180,
                datetime!(2026-03-18 10:08:00),
                datetime!(2026-03-18 10:00:00),
                datetime!(2026-03-18 10:05:00),
            ),
            (
                60,
                0,
                datetime!(2026-03-18 10:08:00),
                datetime!(2026-03-18 10:03:00),
                datetime!(2026-03-18 10:08:00),
            ),
            (
                300,
                0,
                datetime!(2026-03-18 10:08:00),
                datetime!(2026-03-18 10:00:00),
                datetime!(2026-03-18 10:05:00),
            ),
            (
                60,
                0,
                datetime!(1969-12-31 23:59:59),
                datetime!(1969-12-31 23:54:00),
                datetime!(1969-12-31 23:59:00),
            ),
        ];
        let (base_url, mut events, _) = metric_server(
            cases.len(),
            HashMap::from([("window".to_owned(), MetricReply::Value(10.0))]),
            false,
        )
        .await;
        let provider = GrafanaCloudwatch::new(
            GrafanaConfig {
                base_url,
                api_token: Secret::new("test-token".to_owned()),
            },
            Default::default(),
        );
        for (period, offset, now, from, to) in cases {
            let mut config = alert_rule("window-rule", "window", "oncall", 1.0);
            config["config"]["query"]["period"] = json!(period);
            config["config"]["query"]["end_offset_seconds"] = json!(offset);
            let rule: RuleDefinition = serde_json::from_value(config).unwrap();
            let AlertQuery::GrafanaCloudwatch(query) = &rule.config.query;
            let result = observability::core::alert_evaluation::grafana_cloudwatch::evaluate(
                &provider,
                rule.metrics_query("cloudwatch"),
                query,
                &rule.config.thresholds,
                now,
            )
            .await
            .unwrap();
            assert_eq!(result.window.from, from);
            assert_eq!(result.window.to, to);
            assert_eq!(
                result.latest_point.as_ref().map(|point| point.value),
                Some(10.0)
            );
            assert_eq!(
                result.latest_point.as_ref().map(|point| point.timestamp),
                Some(to - time::Duration::seconds(i64::from(period)))
            );
            assert_eq!(result.datapoints.len(), 1);
            let request = loop {
                if let MetricEvent::Started(_, request) = event(&mut events).await {
                    break request;
                }
            };
            assert_eq!(
                request["from"],
                (from.assume_utc().unix_timestamp() * 1000).to_string()
            );
            assert_eq!(
                request["to"],
                (to.assume_utc().unix_timestamp() * 1000).to_string()
            );
        }
        for (period, offset, now) in [
            (0, 0, datetime!(2026-03-18 10:08:00)),
            (60, 0, time::PrimitiveDateTime::MIN),
            (60, u32::MAX, time::PrimitiveDateTime::MIN),
        ] {
            let mut config = alert_rule("invalid-window", "window", "oncall", 1.0);
            config["config"]["query"]["period"] = json!(period);
            config["config"]["query"]["end_offset_seconds"] = json!(offset);
            let rule: RuleDefinition = serde_json::from_value(config).unwrap();
            let AlertQuery::GrafanaCloudwatch(query) = &rule.config.query;
            let error = observability::core::alert_evaluation::grafana_cloudwatch::evaluate(
                &provider,
                rule.metrics_query("cloudwatch"),
                query,
                &rule.config.thresholds,
                now,
            )
            .await
            .unwrap_err();
            assert!(matches!(
                error.current_context(),
                EvaluatorError::InvalidTimeRange
            ));
        }
    });
}

#[test]
fn repeated_current_breach_sends_alarm_on_every_notify() {
    EVALUATOR_RUNTIME.block_on(async {
        let (base_url, _, _) = metric_server(
            2,
            HashMap::from([("repeated".to_owned(), MetricReply::Value(10.0))]),
            false,
        )
        .await;
        let (state, calls) = evaluator_state(
            &base_url,
            vec![alert_rule("repeated-rule", "repeated", "oncall", 1.0)],
            &[("oncall", ChatReply::Delivered)],
        )
        .await;
        for _ in 0..2 {
            let response = observability::core::alert_evaluation::notify(state.clone(), ())
                .await
                .unwrap();
            let body = serde_json::to_value(response).unwrap();
            assert_eq!(body["rules"][0]["status"], "evaluated");
            assert_eq!(body["rules"][0]["level"], "critical");
            assert_eq!(body["rules"][0]["delivered"], true);
            assert!(body["rules"][0]["message"]
                .as_str()
                .unwrap()
                .starts_with("ALARM: repeated-rule\n"));
        }
        assert_eq!(calls["oncall"].lock().unwrap().len(), 2);
    });
}

#[test]
fn evaluator_overlaps_requests_and_preserves_rule_order_after_reverse_completion() {
    EVALUATOR_RUNTIME.block_on(async {
        let (base_url, mut events, gates) = metric_server(
            2,
            HashMap::from([
                ("first".to_owned(), MetricReply::Value(10.0)),
                ("second".to_owned(), MetricReply::Value(20.0)),
            ]),
            true,
        )
        .await;
        let (state, _) = evaluator_state(
            &base_url,
            vec![
                alert_rule("rule-first", "first", "unused", 1.0),
                alert_rule("rule-second", "second", "unused", 1.0),
            ],
            &[],
        )
        .await;
        let evaluation = tokio::spawn(async move {
            observability::core::alert_evaluation::evaluate(state, ())
                .await
                .unwrap()
        });

        let mut started = vec![event(&mut events).await, event(&mut events).await]
            .into_iter()
            .map(|event| match event {
                MetricEvent::Started(metric, _) => metric,
                MetricEvent::Completed(metric) => panic!("{metric} completed before release"),
            })
            .collect::<Vec<_>>();
        started.sort();
        assert_eq!(started, ["first", "second"]);

        gates["second"].add_permits(1);
        assert!(
            matches!(event(&mut events).await, MetricEvent::Completed(metric) if metric == "second")
        );
        gates["first"].add_permits(1);
        assert!(
            matches!(event(&mut events).await, MetricEvent::Completed(metric) if metric == "first")
        );

        let response = timeout(Duration::from_secs(2), evaluation)
            .await
            .expect("concurrent evaluator deadlocked")
            .unwrap();
        assert_eq!(response.rules[0].id, "rule-first");
        assert_eq!(response.rules[1].id, "rule-second");
    });
}

#[test]
fn provider_failure_does_not_cancel_later_breach_and_both_notifications_are_attempted() {
    EVALUATOR_RUNTIME.block_on(async {
        let (base_url, mut events, _) = metric_server(
            2,
            HashMap::from([
                ("fails".to_owned(), MetricReply::Failure),
                ("breaches".to_owned(), MetricReply::Value(10.0)),
            ]),
            false,
        )
        .await;
        let mut failed_rule = alert_rule("failed-rule", "fails", "critical-errors", 1.0);
        failed_rule["name"] = json!("Failed `{raw}` <&>");
        let mut breached_rule = alert_rule("breach-rule", "breaches", "alarm", 1.0);
        breached_rule["name"] = json!("Breach `{raw}` <&>");
        failed_rule["config"]["thresholds"]["severities"]["info"] = json!({
            "threshold": 0.5,
            "comparison_operator": "GreaterThanOrEqualToThreshold",
            "description": "lower-priority synthetic threshold",
            "destination": "info-errors"
        });
        let (state, calls) = evaluator_state(
            &base_url,
            vec![failed_rule, breached_rule],
            &[
                ("critical-errors", ChatReply::Delivered),
                ("info-errors", ChatReply::Delivered),
                ("alarm", ChatReply::Delivered),
            ],
        )
        .await;

        let body = serde_json::to_value(
            observability::core::alert_evaluation::notify(state, ())
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["rules"][0]["id"], "failed-rule");
        assert_eq!(body["rules"][0]["status"], "failed");
        assert!(body["rules"][0]["error"]
            .as_str()
            .unwrap()
            .contains("data source request failed"));
        let error_message = body["rules"][0]["message"].as_str().unwrap();
        assert!(error_message.starts_with("RULE ERROR: Failed `{raw}` <&>\n"));
        assert!(error_message.contains("State: RULE ERROR\n"));
        assert!(error_message.contains("Severity: Not evaluated\n"));
        assert!(error_message.contains("Condition: Not evaluated\n"));
        assert!(error_message.contains("Evaluation: Failed\n"));
        assert!(error_message.contains("Latest value: Unavailable\n"));
        assert!(error_message.contains("Query range: Unavailable\n"));
        assert!(error_message.contains(
            "Datapoints — timestamp | value | evaluation:\nUnavailable: rule evaluation failed\n"
        ));
        assert!(!error_message.contains("SELECTED / BREACHING"));
        assert!(error_message.contains("Metric: fails\n"));
        assert!(error_message.contains("data source request failed"));
        assert!(error_message.ends_with("Definition: failed-rule\n"));
        assert_eq!(body["rules"][0]["delivered"], true);
        assert_eq!(body["rules"][1]["id"], "breach-rule");
        assert_eq!(body["rules"][1]["status"], "evaluated");
        assert_eq!(body["rules"][1]["level"], "critical");
        let alarm_message = body["rules"][1]["message"].as_str().unwrap();
        assert!(alarm_message.starts_with("ALARM: Breach `{raw}` <&>\n"));
        assert!(alarm_message.contains("Severity: CRITICAL\n"));
        assert!(alarm_message.contains("State: ALARM\n"));
        assert!(alarm_message.contains("Metric: breaches\n"));
        assert!(alarm_message.contains("Condition: Sum >= 1\n"));
        assert!(alarm_message.contains("Evaluation: 1 of 1 datapoints\n"));
        assert!(alarm_message.contains("Latest value: 10\n"));
        let request = loop {
            if let MetricEvent::Started(metric, request) = event(&mut events).await {
                if metric == "breaches" {
                    break request;
                }
            }
        };
        let from = common_utils::date_time::convert_to_pdt(
            time::OffsetDateTime::from_unix_timestamp(
                request["from"].as_str().unwrap().parse::<i64>().unwrap() / 1000,
            )
            .unwrap(),
        );
        let to = common_utils::date_time::convert_to_pdt(
            time::OffsetDateTime::from_unix_timestamp(
                request["to"].as_str().unwrap().parse::<i64>().unwrap() / 1000,
            )
            .unwrap(),
        );
        assert!(alarm_message.contains(&format!("Query range: {from} UTC to {to} UTC\n")));
        assert!(alarm_message.contains(&format!(
            "{} UTC | 10 | SELECTED / BREACHING\n",
            to - time::Duration::seconds(60),
        )));
        assert_eq!(alarm_message.matches("SELECTED / BREACHING").count(), 1);
        assert!(alarm_message.contains("Error:\nNone\n"));
        assert!(alarm_message.ends_with("Definition: breach-rule\n"));
        assert_eq!(calls["critical-errors"].lock().unwrap().len(), 1);
        assert!(calls["info-errors"].lock().unwrap().is_empty());
        assert_eq!(calls["alarm"].lock().unwrap().len(), 1);
    });
}

#[test]
fn dry_run_never_delivers_breaches_or_rule_errors() {
    EVALUATOR_RUNTIME.block_on(async {
        let (base_url, _, _) = metric_server(
            2,
            HashMap::from([
                ("fails".to_owned(), MetricReply::Failure),
                ("breaches".to_owned(), MetricReply::Value(10.0)),
            ]),
            false,
        )
        .await;
        let (state, calls) = evaluator_state(
            &base_url,
            vec![
                alert_rule("failed-rule", "fails", "errors", 1.0),
                alert_rule("breach-rule", "breaches", "alarm", 1.0),
            ],
            &[
                ("errors", ChatReply::Delivered),
                ("alarm", ChatReply::Delivered),
            ],
        )
        .await;

        let response = observability::core::alert_evaluation::evaluate(state, ())
            .await
            .unwrap();
        assert_eq!(response.rules.len(), 2);
        assert!(calls.values().all(|calls| calls.lock().unwrap().is_empty()));
    });
}

#[test]
fn no_data_and_normal_are_silent_and_keep_the_exact_wire_shape() {
    EVALUATOR_RUNTIME.block_on(async {
        let (base_url, _, _) = metric_server(
            2,
            HashMap::from([
                ("empty".to_owned(), MetricReply::NoData),
                ("normal".to_owned(), MetricReply::Value(1.0)),
            ]),
            false,
        )
        .await;
        let (state, calls) = evaluator_state(
            &base_url,
            vec![
                alert_rule("empty-rule", "empty", "oncall", 10.0),
                alert_rule("normal-rule", "normal", "oncall", 10.0),
            ],
            &[("oncall", ChatReply::Delivered)],
        )
        .await;

        let body = serde_json::to_value(
            observability::core::alert_evaluation::notify(state, ())
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["rules"][0]["level"], "nodata");
        assert!(body["rules"][0].get("message").is_none());
        assert!(body["rules"][0].get("delivered").is_none());
        assert!(body["rules"][0].get("delivery_error").is_none());
        assert_eq!(body["rules"][1]["level"], "normal");
        assert!(body["rules"][1].get("message").is_none());
        assert!(body["rules"][1].get("delivered").is_none());
        assert!(body["rules"][1].get("delivery_error").is_none());
        assert!(calls["oncall"].lock().unwrap().is_empty());
    });
}

#[test]
fn refused_failed_and_successful_deliveries_are_isolated_and_ordered() {
    EVALUATOR_RUNTIME.block_on(async {
        let (base_url, _, _) = metric_server(
            3,
            HashMap::from([
                ("refused".to_owned(), MetricReply::Value(10.0)),
                ("failed".to_owned(), MetricReply::Value(10.0)),
                ("delivered".to_owned(), MetricReply::Value(10.0)),
            ]),
            false,
        )
        .await;
        let (state, calls) = evaluator_state(
            &base_url,
            vec![
                alert_rule("refused-rule", "refused", "refused-chat", 1.0),
                alert_rule("failed-rule", "failed", "failed-chat", 1.0),
                alert_rule("delivered-rule", "delivered", "delivered-chat", 1.0),
            ],
            &[
                ("refused-chat", ChatReply::Refused),
                ("failed-chat", ChatReply::Failed),
                ("delivered-chat", ChatReply::Delivered),
            ],
        )
        .await;

        let body = serde_json::to_value(
            observability::core::alert_evaluation::notify(state, ())
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["rules"][0]["id"], "refused-rule");
        assert_eq!(body["rules"][0]["status"], "evaluated");
        assert_eq!(body["rules"][0]["delivered"], false);
        assert!(body["rules"][0]["delivery_error"]
            .as_str()
            .unwrap()
            .contains("refused"));
        assert_eq!(body["rules"][1]["id"], "failed-rule");
        assert_eq!(body["rules"][1]["status"], "evaluated");
        assert_eq!(body["rules"][1]["delivered"], false);
        assert!(body["rules"][1]["delivery_error"]
            .as_str()
            .unwrap()
            .contains("could not be reached"));
        assert_eq!(body["rules"][2]["id"], "delivered-rule");
        assert_eq!(body["rules"][2]["status"], "evaluated");
        assert_eq!(body["rules"][2]["delivered"], true);
        assert!(body["rules"][2].get("delivery_error").is_none());
        assert!(calls.values().all(|calls| calls.lock().unwrap().len() == 1));
    });
}

#[test]
fn cloudwatch_notify_route_enforces_auth_verbs_and_legacy_removal_with_a_real_rule() {
    EVALUATOR_RUNTIME.block_on(async {
        let (base_url, _, _) = metric_server(
            1,
            HashMap::from([("breach".to_owned(), MetricReply::Value(10.0))]),
            false,
        )
        .await;
        let (state, calls) = evaluator_state(
            &base_url,
            vec![alert_rule("breach-rule", "breach", "oncall", 1.0)],
            &[("oncall", ChatReply::Delivered)],
        )
        .await;
        let app = test::init_service(App::new().service(Alerts::server(state))).await;

        let unauthorized = TestRequest::post()
            .uri("/alerts/grafana/cloudwatch/notify")
            .to_request();
        assert_eq!(
            test::call_service(&app, unauthorized).await.status(),
            StatusCode::UNAUTHORIZED
        );
        let wrong_verb = TestRequest::get()
            .uri("/alerts/grafana/cloudwatch/notify")
            .insert_header((X_INTERNAL_API_KEY, API_KEY))
            .to_request();
        assert_eq!(
            test::call_service(&app, wrong_verb).await.status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        let wrong_evaluate_verb = TestRequest::post()
            .uri("/alerts/grafana/cloudwatch/evaluate")
            .insert_header((X_INTERNAL_API_KEY, API_KEY))
            .to_request();
        assert_eq!(
            test::call_service(&app, wrong_evaluate_verb).await.status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        let old_path = TestRequest::post()
            .uri("/alerts/grafana/notify")
            .insert_header((X_INTERNAL_API_KEY, API_KEY))
            .to_request();
        assert_eq!(
            test::call_service(&app, old_path).await.status(),
            StatusCode::NOT_FOUND
        );
        let authorized = TestRequest::post()
            .uri("/alerts/grafana/cloudwatch/notify")
            .insert_header((X_INTERNAL_API_KEY, API_KEY))
            .to_request();
        let response = test::call_service(&app, authorized).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value = test::read_body_json(response).await;
        assert_eq!(body["rules"][0]["status"], "evaluated");
        assert_eq!(body["rules"][0]["delivered"], true);
        assert_eq!(calls["oncall"].lock().unwrap().len(), 1);
    });
}

#[actix_web::test]
async fn a_chat_notification_reports_delivery_and_a_message_id() {
    let (status, body) = call(post(
        &format!("/alerts/chat/notify/{CHAT}"),
        json!({ "text": "*3 merchants not converting*" }),
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "delivered");
    assert!(body["message_id"].as_str().is_some_and(|id| !id.is_empty()));
    assert!(body.get("error_code").is_none());
}

/// The threading round trip a recovery notice needs: post, keep the id, reply under it.
#[actix_web::test]
async fn a_chat_notification_can_reply_under_an_earlier_message() {
    let (_, first) = call(post(
        &format!("/alerts/chat/notify/{CHAT}"),
        json!({ "text": "alerts" }),
    ))
    .await;

    let (status, second) = call(post(
        &format!("/alerts/chat/notify/{CHAT}"),
        json!({ "text": "recovered", "reply_to": first["message_id"] }),
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["status"], "delivered");
}

#[actix_web::test]
async fn a_chat_file_can_be_uploaded_into_a_thread() {
    let (status, body) = call(upload(upload_body("%PDF-test"))).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "delivered");
    assert!(body["file_id"].as_str().is_some_and(|id| !id.is_empty()));
    assert!(body.get("message_id").is_none());
}

#[actix_web::test]
async fn an_upload_is_refused_while_streaming_past_the_configured_cap() {
    let (status, body) =
        call_with_state(upload(upload_body("four")), state_with_max(3).await).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_unparseable_upload_uses_the_standard_bad_request_shape() {
    let request = TestRequest::post()
        .uri(&format!("/alerts/chat/upload/{CHAT}"))
        .insert_header(("content-type", "multipart/form-data; boundary=BOUNDARY"))
        .set_payload("not multipart");
    let (status, body) = call(request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_upload_rejects_unknown_fields() {
    let body = upload_body("%PDF").replace("name=\"title\"", "name=\"titel\"");
    let (status, body) = call(upload(body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_email_notification_reports_delivery() {
    let (status, body) = call(post(
        &format!("/alerts/email/notify/{EMAIL}"),
        json!({ "subject": "3 merchants not converting", "body": "<pre>...</pre>" }),
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "delivered");
}

/// The load-bearing property of the response shape: a caller cannot read a `200` and assume the
/// message arrived, because `status` is always there to be checked.
#[actix_web::test]
async fn every_success_carries_a_status() {
    for (uri, body) in [
        (
            format!("/alerts/chat/notify/{CHAT}"),
            json!({ "text": "x" }),
        ),
        (
            format!("/alerts/email/notify/{EMAIL}"),
            json!({ "subject": "s", "body": "b" }),
        ),
    ] {
        let (status, body) = call(post(&uri, body)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.get("status").is_some(), "{uri} returned no status");
    }
}

/// Threading against a mailing list is a caller bug, and `deny_unknown_fields` makes it loud rather
/// than a field that quietly goes nowhere.
#[actix_web::test]
async fn threading_against_an_email_destination_is_rejected() {
    let (status, body) = call(post(
        &format!("/alerts/email/notify/{EMAIL}"),
        json!({ "subject": "s", "body": "b", "reply_to": "cmtk931s1" }),
    ))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_unknown_destination_is_a_404() {
    let (status, body) = call(post("/alerts/chat/notify/typo", json!({ "text": "x" }))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "IR_02");
}

/// The id a caller guessed must not come back, and nor must the ones that exist.
#[actix_web::test]
async fn an_unknown_destination_does_not_echo_or_enumerate() {
    let (_, body) = call(post("/alerts/chat/notify/typo", json!({ "text": "x" }))).await;

    let rendered = body.to_string();
    assert!(!rendered.contains("typo"));
    assert!(!rendered.contains(CHAT));
}

/// A malformed body must render like every other error, not as actix's own plain-text 400.
#[actix_web::test]
async fn a_missing_field_renders_in_our_error_shape() {
    let (status, body) = call(post(&format!("/alerts/chat/notify/{CHAT}"), json!({}))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
    assert_eq!(body["error"]["type"], "invalid_request");
}

/// Serde's message quotes the body, and a body carries merchant ids and payment volumes. It goes to
/// the log; the caller gets the code.
#[actix_web::test]
async fn a_parse_failure_does_not_echo_the_body_back() {
    let (_, body) = call(post(
        &format!("/alerts/chat/notify/{CHAT}"),
        json!({ "text": 12345 }),
    ))
    .await;

    assert!(!body.to_string().contains("12345"));
}

#[actix_web::test]
async fn both_routes_are_behind_the_guard() {
    for (uri, body) in [
        (
            format!("/alerts/chat/notify/{CHAT}"),
            json!({ "text": "x" }),
        ),
        (
            format!("/alerts/email/notify/{EMAIL}"),
            json!({ "subject": "s", "body": "b" }),
        ),
    ] {
        let (status, body) = call(TestRequest::post().uri(&uri).set_json(body)).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} must be guarded");
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

/// The guard runs before the destination is resolved, so an unauthenticated caller cannot probe
/// which ids exist by watching the status change.
#[actix_web::test]
async fn a_bad_key_is_rejected_before_the_destination_is_resolved() {
    let (known, _) = call(
        TestRequest::post()
            .uri(&format!("/alerts/chat/notify/{CHAT}"))
            .insert_header((X_INTERNAL_API_KEY, "wrong"))
            .set_json(json!({ "text": "x" })),
    )
    .await;

    let (unknown, _) = call(
        TestRequest::post()
            .uri("/alerts/chat/notify/typo")
            .insert_header((X_INTERNAL_API_KEY, "wrong"))
            .set_json(json!({ "text": "x" })),
    )
    .await;

    assert_eq!(known, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown, StatusCode::UNAUTHORIZED);
}

/// **Known and accepted:** actix runs the body extractor before the handler, and the guard runs
/// inside the handler via `server_wrap`, so an unparseable body is answered 400 without its key
/// being checked. Restoring "guard strictly first" means extracting raw bytes and deserializing by
/// hand, trading typed extraction for the concealment of a documented schema. Kept as a test so the
/// ordering is a recorded property rather than something a reviewer rediscovers.
#[actix_web::test]
async fn an_unparseable_body_is_rejected_before_the_key_is_checked() {
    let (status, body) = call(
        TestRequest::post()
            .uri(&format!("/alerts/chat/notify/{CHAT}"))
            .set_json(json!({})),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}
