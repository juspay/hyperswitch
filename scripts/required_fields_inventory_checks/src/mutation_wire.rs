//! Actual locked SDK serializers/retry middleware, intercepted entirely in memory.
//! The direct adapter keeps frozen raw bytes; semantic SDK equality is not a server certificate.
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, Mutex},
    time::Duration,
};

use aws_smithy_runtime_api::client::{
    http::{http_client_fn, HttpConnector, HttpConnectorFuture, SharedHttpConnector},
    orchestrator::{HttpRequest, HttpResponse},
};
use aws_smithy_types::{
    body::SdkBody, retry::RetryConfig, timeout::TimeoutConfig, Document, Number,
};
use serde_json::{json, Value};

use super::{
    apply_contract::CertifiedMutationTarget,
    coordination::tests::{d, n, registration},
    journal::{Method, SecretGuard},
    mutation::{compile, CHANGE_REASON},
    planner_types::{ActionKind, PlannedAction},
    raw_json::Json,
    types::EvidenceScope,
};

fn document(v: &Value) -> Document {
    match v {
        Value::Null => Document::Null,
        Value::Bool(v) => Document::Bool(*v),
        Value::String(v) => Document::String(v.clone()),
        Value::Array(v) => Document::Array(v.iter().map(document).collect()),
        Value::Object(v) => {
            Document::Object(v.iter().map(|(k, v)| (k.clone(), document(v))).collect())
        }
        Value::Number(v) => Document::Number(if let Some(n) = v.as_u64() {
            Number::PosInt(n)
        } else if let Some(n) = v.as_i64() {
            Number::NegInt(n)
        } else {
            Number::Float(v.as_f64().unwrap())
        }),
    }
}
fn object(v: Value) -> HashMap<String, Document> {
    match document(&v) {
        Document::Object(m) => m,
        _ => panic!("object"),
    }
}
fn actions() -> Vec<PlannedAction> {
    let raw = |s: &str| Json::parse(s.as_bytes()).unwrap();
    let action =
        |kind, id: Option<&str>, before: Option<Json>, after: Option<Json>| PlannedAction {
            kind,
            server_id: id.map(n),
            condition: None,
            before,
            after,
            preservation_hash: d("preserved"),
            ownership_intent: None,
        };
    vec![
        action(
            ActionKind::ExtendDimension,
            Some("connector"),
            Some(raw(
                r#"{"dimension":"connector","schema":{"type":"string","enum":["a"]},"description":"dimension description"}"#,
            )),
            Some(raw(
                r#"{"dimension":"connector","schema":{"type":"string","enum":["a","b"]},"description":"dimension description"}"#,
            )),
        ),
        action(
            ActionKind::CreateDefault,
            None,
            None,
            Some(raw(
                r#"{"key":"dynamic_fields.choice._is_required","value":false,"schema":{"type":"boolean"},"description":"default description"}"#,
            )),
        ),
        action(
            ActionKind::Create,
            None,
            None,
            Some(raw(
                r#"{"value":{"connector":"b"},"override":{"dynamic_fields.choice._is_required":true},"description":"created description"}"#,
            )),
        ),
        action(
            ActionKind::Replace,
            Some("server-A"),
            Some(raw(
                r#"{"value":{"connector":"a"},"override":{"dynamic_fields.choice._is_required":true},"description":"preserved description"}"#,
            )),
            Some(raw(
                r#"{"value":{"connector":"a"},"override":{"dynamic_fields.choice._is_required":false,"unrelated.ordered":["B","A","A"],"unrelated.decimal":1.2300},"description":"preserved description"}"#,
            )),
        ),
        action(
            ActionKind::Delete,
            Some("server-retired"),
            Some(raw(
                r#"{"value":{"connector":"old"},"override":{"dynamic_fields.choice._is_required":true}}"#,
            )),
            None,
        ),
    ]
}
fn target() -> CertifiedMutationTarget {
    let r = registration();
    CertifiedMutationTarget {
        id: r.server_contract.clone(),
        registration: r,
        read_contract: n("synthetic-read"),
        server_image: d("synthetic-server"),
        server_source: "4".repeat(40).try_into().unwrap(),
        scope: EvidenceScope::SyntheticFixture,
        max_prerequisites: 32,
        max_context_actions: 100,
        max_body: 512 * 1024,
        request_ms: 30,
        rollback_statuses: BTreeSet::from([400]),
        context_audit: BTreeSet::new(),
        default_audit: BTreeSet::new(),
        dimension_audit: BTreeSet::new(),
    }
}
#[derive(Debug)]
struct Seen {
    method: String,
    path: String,
    body: Vec<u8>,
    org: Option<String>,
    workspace: Option<String>,
}
#[derive(Clone, Debug)]
struct Capture {
    status: u16,
    hang: bool,
    seen: Arc<Mutex<Vec<Seen>>>,
}
impl HttpConnector for Capture {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let this = self.clone();
        HttpConnectorFuture::new(async move {
            let url = reqwest::Url::parse(request.uri()).unwrap();
            this.seen.lock().unwrap().push(Seen {
                method: request.method().into(),
                path: url.path().into(),
                body: request.body().bytes().unwrap().to_vec(),
                org: request.headers().get("x-org-id").map(str::to_owned),
                workspace: request.headers().get("x-workspace").map(str::to_owned),
            });
            if this.hang {
                std::future::pending::<()>().await;
            }
            let mut response =
                HttpResponse::new(this.status.try_into().unwrap(), SdkBody::from("{}"));
            response
                .headers_mut()
                .insert("content-type", "application/json");
            Ok(response)
        })
    }
}
async fn invoke(capture: Capture) {
    use superposition_sdk::types::{
        ContextAction, ContextIdentifier, ContextPut, UpdateContextOverrideRequest,
    };
    let connector = SharedHttpConnector::new(capture);
    let client = superposition_sdk::Client::from_conf(
        superposition_sdk::Config::builder()
            .behavior_version(superposition_sdk::config::BehaviorVersion::latest())
            .endpoint_url("https://synthetic.invalid/sp")
            .bearer_token(superposition_sdk::config::Token::new(
                "synthetic-fixture",
                None,
            ))
            .http_client(http_client_fn(move |_, _| connector.clone()))
            .retry_config(RetryConfig::standard().with_max_attempts(1))
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_millis(30))
                    .operation_attempt_timeout(Duration::from_millis(30))
                    .build(),
            )
            .sleep_impl(aws_smithy_async::rt::sleep::TokioSleep::new())
            .build(),
    );
    let _ = client
        .update_dimension()
        .org_id("synthetic-org")
        .workspace_id("synthetic-workspace")
        .dimension("connector")
        .set_schema(Some(object(json!({"type":"string","enum":["a","b"]}))))
        .description("dimension description")
        .change_reason(CHANGE_REASON)
        .send()
        .await;
    let _ = client
        .create_default_config()
        .org_id("synthetic-org")
        .workspace_id("synthetic-workspace")
        .key("dynamic_fields.choice._is_required")
        .value(Document::Bool(false))
        .set_schema(Some(object(json!({"type":"boolean"}))))
        .description("default description")
        .change_reason(CHANGE_REASON)
        .send()
        .await;
    let operations=vec![
        ContextAction::Put(ContextPut::builder().set_context(Some(object(json!({"connector":"b"})))).set_override(Some(object(json!({"dynamic_fields.choice._is_required":true})))).description("created description").change_reason(CHANGE_REASON).build().unwrap()),
        ContextAction::Replace(UpdateContextOverrideRequest::builder().context(ContextIdentifier::Id("server-A".into()))
            .set_override(Some(object(json!({"dynamic_fields.choice._is_required":false,"unrelated.ordered":["B","A","A"],"unrelated.decimal":1.23})))).description("preserved description").change_reason(CHANGE_REASON).build().unwrap()),
        ContextAction::Delete("server-retired".into()),
    ];
    let _ = client
        .bulk_operation()
        .org_id("synthetic-org")
        .workspace_id("synthetic-workspace")
        .set_operations(Some(operations))
        .send()
        .await;
}
#[tokio::test]
async fn frozen_operations_match_locked_sdk_and_keep_decimal_lexemes_and_order() {
    let requests = compile(&actions(), &target(), &SecretGuard::default()).unwrap();
    assert_eq!(requests.len(), 3);
    let capture = Capture {
        status: 200,
        hang: false,
        seen: Arc::default(),
    };
    invoke(capture.clone()).await;
    let seen = capture.seen.lock().unwrap();
    assert_eq!(seen.len(), 3);
    for (request, wire) in requests.iter().zip(seen.iter()) {
        let method = match request.method() {
            Method::Post => "POST",
            Method::Patch => "PATCH",
            Method::Put => "PUT",
            Method::Delete => panic!("no standalone context delete"),
        };
        assert_eq!(wire.method, method);
        assert_eq!(wire.path, format!("/sp{}", request.path()));
        assert_eq!(wire.org.as_deref(), Some("synthetic-org"));
        assert_eq!(wire.workspace.as_deref(), Some("synthetic-workspace"));
        assert_eq!(
            serde_json::from_slice::<Value>(&wire.body).unwrap(),
            serde_json::from_slice::<Value>(request.body()).unwrap()
        );
        assert_eq!(
            request.headers(),
            &BTreeMap::from([("content-type".into(), "application/json".into())])
        );
    }
    assert!(std::str::from_utf8(requests[2].body())
        .unwrap()
        .contains("1.2300"));
    assert!(!std::str::from_utf8(&seen[2].body)
        .unwrap()
        .contains("1.2300"));
    assert!(std::str::from_utf8(requests[2].body())
        .unwrap()
        .contains(r#"["B","A","A"]"#));
}
#[tokio::test]
async fn locked_sdk_mutations_attempt_once_on_rejection_retryable_errors_postcommit_error_and_timeout(
) {
    for status in [400, 409, 429, 500, 503, 512] {
        let capture = Capture {
            status,
            hang: false,
            seen: Arc::default(),
        };
        invoke(capture.clone()).await;
        assert_eq!(
            capture.seen.lock().unwrap().len(),
            3,
            "one call per operation for status {status}"
        );
    }
    let capture = Capture {
        status: 200,
        hang: true,
        seen: Arc::default(),
    };
    invoke(capture.clone()).await;
    assert_eq!(
        capture.seen.lock().unwrap().len(),
        3,
        "timeouts do not retry"
    );
}
