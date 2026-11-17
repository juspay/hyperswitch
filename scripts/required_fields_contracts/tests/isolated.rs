//! Real generated SDK, scripted in-memory HTTP connector. NO server certification.
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use aws_smithy_runtime_api::client::{
    http::{HttpConnector, HttpConnectorFuture, SharedHttpConnector},
    orchestrator::HttpRequest,
};
use required_fields_api_contracts::{
    api::{self, Api},
    contract::{self, Observation, Registration},
    transport::{self, Exchange, Trace},
};
use serde_json::{json, Value};

#[derive(Clone, Debug)]
enum Fault {
    Reply(u16, Value),
    ConfigWithHeaders(Value),
    Truncated,
    CommitDrop(Arc<AtomicBool>),
    CommitError(Arc<AtomicBool>, u16),
    LateCommit(Arc<AtomicBool>, Arc<tokio::sync::Notify>),
}
#[derive(Clone, Debug)]
struct Script {
    steps: Arc<Mutex<VecDeque<Fault>>>,
    trace: Trace,
    headers: Arc<Mutex<Vec<Headers>>>,
}
type Headers = Vec<(String, String)>;
impl HttpConnector for Script {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let this = self.clone();
        HttpConnectorFuture::new(async move {
            this.headers.lock().unwrap().push(
                request
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            );
            let url = reqwest::Url::parse(request.uri()).unwrap();
            let input = request.body().bytes().unwrap();
            let request_body = if input.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(input).unwrap()
            };
            let index = {
                let mut trace = this.trace.lock().unwrap();
                trace.push(Exchange {
                    method: request.method().into(),
                    path: format!(
                        "{}{}",
                        url.path(),
                        url.query().map(|q| format!("?{q}")).unwrap_or_default()
                    ),
                    request: request_body,
                    status: None,
                    body: None,
                    version: None,
                    last_modified: None,
                });
                trace.len() - 1
            };
            let fault = this
                .steps
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected additional dispatch");
            let (status, body) = match fault {
                Fault::Reply(status, body) => (status, body),
                Fault::ConfigWithHeaders(body) => {
                    let mut response = transport::response(
                        200,
                        serde_json::to_vec(&body).unwrap(),
                        Some("fixture-version"),
                    );
                    response
                        .headers_mut()
                        .insert("last-modified", "2026-11-18T00:00:00Z");
                    let mut trace = this.trace.lock().unwrap();
                    trace[index].status = Some(200);
                    trace[index].body = Some(body);
                    trace[index].version = Some("fixture-version".into());
                    trace[index].last_modified = Some("2026-11-18T00:00:00Z".into());
                    return Ok(response);
                }
                Fault::Truncated => return Ok(transport::response(200, b"{".to_vec(), None)),
                Fault::CommitDrop(flag) => {
                    flag.store(true, Ordering::SeqCst);
                    return Err(transport::error(
                        "synthetic response dropped after simulated commit",
                    ));
                }
                Fault::CommitError(flag, status) => {
                    flag.store(true, Ordering::SeqCst);
                    (
                        status,
                        json!({"message":"synthetic error after simulated commit"}),
                    )
                }
                Fault::LateCommit(flag, release) => {
                    // A server task can outlive a cancelled client future.
                    tokio::spawn(async move {
                        release.notified().await;
                        flag.store(true, Ordering::SeqCst);
                    });
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    (200, json!({}))
                }
            };
            {
                let mut trace = this.trace.lock().unwrap();
                trace[index].status = Some(status);
                trace[index].body = Some(body.clone());
                trace[index].version = Some("fixture-version".into());
            }
            Ok(transport::response(
                status,
                serde_json::to_vec(&body).unwrap(),
                Some("fixture-version"),
            ))
        })
    }
}
fn setup(steps: Vec<Fault>, read_attempts: u32, timeout: Duration) -> (Api, Script) {
    let script = Script {
        steps: Arc::new(Mutex::new(steps.into())),
        trace: transport::trace(),
        headers: Arc::new(Mutex::new(vec![])),
    };
    let connector = SharedHttpConnector::new(script.clone());
    let api = Api {
        reader: api::client(
            connector.clone(),
            "http://127.0.0.1:1",
            read_attempts,
            timeout,
        ),
        writer: api::client(connector, "http://127.0.0.1:1", 1, timeout),
        trace: script.trace.clone(),
        org: "localorg".into(),
        workspace: "rf_contract_fixture".into(),
    };
    (api, script)
}
fn bad_request() -> Fault {
    Fault::Reply(400, json!({"message":"synthetic validation rejection"}))
}
fn empty_page() -> Value {
    json!({"total_pages":0,"total_items":0,"data":[]})
}
fn put() -> Value {
    json!({"context":{"rf_scope":"a"},"override":{"rf.options":["B","A","B"],"rf.manual":{"description":"preserve","function_name":"synthetic_fn"}},"description":"keep description"})
}

#[tokio::test]
async fn pinned_sdk_serializes_scopes_and_full_replacement_without_sorting_arrays() {
    let (api, script) = setup(
        vec![bad_request(), bad_request(), bad_request()],
        1,
        Duration::from_secs(3),
    );
    api.call("put", put()).await.unwrap();
    let full =
        json!({"id":"owned-id","override":put()["override"],"description":"keep description"});
    api.call("replace", full.clone()).await.unwrap();
    api.call(
        "bulk",
        json!([{"REPLACE":full},{"DELETE":"obsolete-owned-id"}]),
    )
    .await
    .unwrap();
    let rows = script.trace.lock().unwrap();
    assert_eq!(
        (rows[0].method.as_str(), rows[0].path.as_str()),
        ("PUT", "/context")
    );
    assert_eq!(
        (rows[1].method.as_str(), rows[1].path.as_str()),
        ("PATCH", "/context/overrides")
    );
    assert_eq!(
        (rows[2].method.as_str(), rows[2].path.as_str()),
        ("PUT", "/context/bulk-operations")
    );
    assert_eq!(rows[0].request["override"], put()["override"]);
    assert_eq!(rows[1].request["override"], put()["override"]);
    assert_eq!(rows[1].request["description"], "keep description");
    assert_eq!(
        rows[2].request["operations"][0]["REPLACE"]["override"],
        put()["override"]
    );
    for headers in script.headers.lock().unwrap().iter() {
        assert!(headers.contains(&("x-workspace".into(), "rf_contract_fixture".into())));
        assert!(headers.contains(&("x-org-id".into(), "localorg".into())));
        assert!(!headers.iter().any(|(k, _)| k == "x-tenant"));
    }
}

#[tokio::test]
async fn real_sdk_list_query_is_paginated_and_prefix_is_explicit_only() {
    let (api, script) = setup(
        vec![
            Fault::Reply(200, empty_page()),
            Fault::Reply(200, empty_page()),
        ],
        1,
        Duration::from_secs(3),
    );
    assert!(
        api.call("contexts", json!({"count":2,"page":1}))
            .await
            .unwrap()
            .sdk_ok
    );
    assert!(
        api.call(
            "contexts",
            json!({"count":2,"page":2,"prefix":"rf.required"})
        )
        .await
        .unwrap()
        .sdk_ok
    );
    let rows = script.trace.lock().unwrap();
    assert!(rows[0].path.contains("page=1") && rows[0].path.contains("count=2"));
    assert!(!rows[0].path.contains("prefix") && !rows[0].path.contains("all=true"));
    assert!(rows[1].path.contains("prefix=rf.required") && rows[1].path.contains("page=2"));
}

#[tokio::test]
async fn generated_client_read_retries_are_bounded_while_writer_dispatches_once() {
    let steps = vec![
        Fault::Reply(503, json!({"message":"synthetic"})),
        Fault::Reply(200, empty_page()),
        Fault::Reply(503, json!({"message":"synthetic"})),
    ];
    let (api, script) = setup(steps, 2, Duration::from_secs(5));
    let read = api.call("contexts", json!({})).await.unwrap();
    assert!(read.sdk_ok);
    assert_eq!(read.dispatches, 2);
    let write = api.call("put", put()).await.unwrap();
    assert!(!write.sdk_ok);
    assert_eq!(write.dispatches, 1);
    assert!(script.steps.lock().unwrap().is_empty());
}

#[tokio::test]
async fn malformed_success_body_never_becomes_compatible_server_evidence() {
    let (api, _) = setup(vec![Fault::Truncated], 1, Duration::from_secs(3));
    let reply = api.call("contexts", json!({})).await.unwrap();
    assert!(!reply.sdk_ok);
    assert!(reply.success().is_err());
}

#[tokio::test]
async fn post_commit_errors_and_dropped_responses_are_unknown_without_observation() {
    for status in [None, Some(512), Some(500)] {
        let committed = Arc::new(AtomicBool::new(false));
        let fault = status
            .map(|s| Fault::CommitError(committed.clone(), s))
            .unwrap_or_else(|| Fault::CommitDrop(committed.clone()));
        let (api, _) = setup(vec![fault], 1, Duration::from_secs(3));
        let reply = api.call("put", put()).await.unwrap();
        assert!(!reply.sdk_ok);
        assert_eq!(reply.dispatches, 1);
        assert!(committed.load(Ordering::SeqCst));
        assert_eq!(
            contract::outcome(true, false, false, Observation::Unavailable),
            "CommitUnknown"
        );
        assert_eq!(
            contract::outcome(true, false, false, Observation::After),
            "CommittedNeedsVerification"
        );
    }
}

#[tokio::test]
async fn caller_timeout_does_not_exclude_a_late_server_commit_or_authorize_retry() {
    let committed = Arc::new(AtomicBool::new(false));
    let release = Arc::new(tokio::sync::Notify::new());
    let (api, script) = setup(
        vec![Fault::LateCommit(committed.clone(), release.clone())],
        2,
        Duration::from_millis(20),
    );
    let reply = api.call("put", put()).await.unwrap();
    assert!(!reply.sdk_ok);
    assert!(!committed.load(Ordering::SeqCst));
    assert_eq!(reply.dispatches, 1);
    assert_eq!(
        contract::outcome(true, false, false, Observation::Before),
        "CommitUnknown"
    );
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(1), async {
        while !committed.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(committed.load(Ordering::SeqCst));
    assert_eq!(script.trace.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn bulk_action_and_byte_caps_refuse_before_any_sdk_dispatch_without_batching() {
    let (api, script) = setup(vec![], 1, Duration::from_secs(3));
    let too_many = Value::Array((0..17).map(|_| json!({"PUT":put()})).collect());
    assert!(api.call("bulk", too_many).await.is_err());
    let too_large = json!([{"PUT":{"context":{"rf_scope":"a"},"override":{"rf.manual":"x".repeat(contract::MAX_BODY)},"description":"synthetic"}}]);
    assert!(api.call("bulk", too_large).await.is_err());
    assert!(api.call("bulk", json!([])).await.is_err());
    assert!(script.trace.lock().unwrap().is_empty());
}

#[tokio::test]
async fn workspace_prerequisite_and_experiment_operations_use_actual_sdk_shapes() {
    let (api, script) = setup(vec![bad_request(); 7], 1, Duration::from_secs(3));
    api.call("create_workspace", json!({})).await.unwrap();
    api.call(
        "create_dimension",
        json!({"key":"rf_scope","schema":{"type":"string","enum":["a","b"],"minLength":1}}),
    )
    .await
    .unwrap();
    api.call(
        "update_dimension",
        json!({"key":"rf_scope","schema":{"type":"string","enum":["a","b","c"],"minLength":1}}),
    )
    .await
    .unwrap();
    api.call(
        "create_default",
        json!({"key":"rf.manual","value":"baseline","schema":{"type":"string","maxLength":128}}),
    )
    .await
    .unwrap();
    api.call(
        "create_experiment",
        json!({"context":{"rf_scope":"experiment"}}),
    )
    .await
    .unwrap();
    api.call("ramp_experiment", json!({"id":"owned-experiment"}))
        .await
        .unwrap();
    api.call("delete", json!({"id":"owned-context"}))
        .await
        .unwrap();
    let rows = script.trace.lock().unwrap();
    assert_eq!(rows[0].path, "/workspaces");
    assert_eq!(rows[0].request["workspace_name"], "rf_contract_fixture");
    assert_eq!(
        rows[0].request["workspace_admin_email"],
        "rf-fixture@example.invalid"
    );
    assert_eq!(rows[2].request["schema"]["enum"], json!(["a", "b", "c"]));
    assert_eq!(rows[2].request["schema"]["minLength"], 1);
    assert!(rows[2].request.get("position").is_none());
    assert_eq!(rows[3].request["value"], "baseline");
    assert_eq!(rows[4].request["variants"][0]["variant_type"], "CONTROL");
    assert_eq!(
        rows[4].request["variants"][1]["overrides"]["rf.required"],
        true
    );
    assert_eq!(rows[5].request["traffic_percentage"], 50);
    assert_eq!(rows[6].method, "DELETE");
}

#[test]
fn pagination_refuses_changed_totals_duplicate_missing_partial_and_permission_scoped_rows() {
    let first = json!({"total_pages":2,"total_items":3,"data":[{"id":"a","override":{"manual":1}},{"id":"b","override":{"manual":2}}]});
    let second =
        json!({"total_pages":2,"total_items":3,"data":[{"id":"c","override":{"manual":3}}]});
    let version = json!("v1");
    let rows = contract::pages(
        &[first.clone(), second.clone()],
        2,
        &version,
        &version,
        "id",
    )
    .unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2]["override"]["manual"], 3);
    assert!(contract::pages(std::slice::from_ref(&first), 2, &version, &version, "id").is_err());
    assert!(contract::pages(&[first.clone(), first.clone()], 2, &version, &version, "id").is_err());
    for bad in [
        json!({"total_pages":2,"total_items":4,"data":[{"id":"c"}]}),
        json!({"total_pages":2,"total_items":3,"data":[]}),
        json!({"total_pages":2,"total_items":3}),
        json!({"total_pages":2,"total_items":3,"data":[{"id":"b"}]}),
    ] {
        assert!(contract::pages(&[first.clone(), bad], 2, &version, &version, "id").is_err());
    }
    assert!(contract::pages(
        &[first.clone(), second.clone()],
        2,
        &version,
        &json!("v2"),
        "id"
    )
    .is_err());
    // Same totals cannot prove permission completeness: compare known fixture IDs.
    let scoped = json!({"total_pages":1,"total_items":1,"data":[{"id":"a"}]});
    let scoped = contract::pages(&[scoped], 2, &version, &version, "id").unwrap();
    assert_ne!(scoped, rows);
}

#[test]
fn page_byte_limit_and_missing_identity_are_not_complete_inventory() {
    let large = json!({"total_pages":1,"total_items":1,"data":[{"id":"a","value":"x".repeat(contract::MAX_BODY)}]});
    assert!(contract::pages(&[large], 2, &json!(1), &json!(1), "id").is_err());
    assert!(contract::pages(
        &[json!({"total_pages":1,"total_items":1,"data":[{}]})],
        2,
        &json!(1),
        &json!(1),
        "id"
    )
    .is_err());
}

#[test]
fn prerequisite_and_context_transaction_outcomes_do_not_hide_partial_or_unknown_commits() {
    assert_eq!(
        contract::outcome(false, false, false, Observation::Before),
        "Refused"
    );
    assert_eq!(
        contract::outcome(false, true, false, Observation::Before),
        "PartialApplied"
    );
    assert_eq!(
        contract::outcome(true, false, true, Observation::Before),
        "RejectedNoChange"
    );
    assert_eq!(
        contract::outcome(true, true, true, Observation::Before),
        "PartialApplied"
    );
    for observation in [
        Observation::Before,
        Observation::Diverged,
        Observation::Unavailable,
    ] {
        assert_eq!(
            contract::outcome(true, true, false, observation),
            "CommitUnknown"
        );
    }
}

fn registration() -> Registration {
    Registration {
        schema_version: 1,
        run_id: "a".repeat(32),
        docker_host: "unix:///fixture/docker.sock".into(),
        server_image: format!(
            "ghcr.io/juspay/superposition-demo@sha256:{}",
            "b".repeat(64)
        ),
        server_image_id: format!("sha256:{}", "c".repeat(64)),
        server_version: "0.115.5".into(),
        source_revision: "d".repeat(40),
        server_container: "e".repeat(64),
        redis_container: "f".repeat(64),
        network: "1".repeat(64),
        endpoint: "http://127.0.0.1:34567".into(),
        org: "localorg".into(),
        workspace: format!("rf_contract_{}", "a".repeat(32)),
    }
}
#[test]
fn registrations_reject_hosted_dns_credentials_floating_images_and_existing_workspaces() {
    registration().validate().unwrap();
    for endpoint in [
        "https://host.invalid",
        "http://localhost:34567",
        "http://127.0.0.1:34567/base",
        "http://127.0.0.1:34567/?x=1",
        "http://user:password@127.0.0.1:34567",
    ] {
        let mut r = registration();
        r.endpoint = endpoint.into();
        assert!(r.validate().is_err());
    }
    let mut r = registration();
    r.workspace = "dev".into();
    assert!(r.validate().is_err());
    let mut r = registration();
    r.server_image = "ghcr.io/juspay/superposition-demo:0.115.5".into();
    assert!(r.validate().is_err());
    let mut r = registration();
    r.docker_host = "tcp://host.invalid:2375".into();
    assert!(r.validate().is_err());
}

#[test]
fn reports_cannot_promote_mock_missing_or_duplicate_evidence_to_server_certification() {
    let mut results = contract::pending_cases();
    contract::validate_results(&results, false).unwrap();
    results[0].status = "passed".into();
    assert!(contract::validate_results(&results, false).is_err());
    contract::validate_results(&results, true).unwrap();
    results.pop();
    assert!(contract::validate_results(&results, true).is_err());
    let mut results = contract::pending_cases();
    results[1].id = results[0].id.clone();
    assert!(contract::validate_results(&results, true).is_err());
}

#[test]
fn ordered_array_identity_is_semantic_even_if_an_external_hash_collides() {
    let a = json!({"rf.options":["A","B","A"]});
    let b = json!({"rf.options":["B","A","A"]});
    assert_ne!(a, b);
    assert_ne!(api::object(&a).unwrap(), api::object(&b).unwrap());
}

#[test]
fn same_count_and_version_do_not_hide_changed_full_rows_or_export_order() {
    let a = vec![
        json!({"id":"a","description":"manual","override":{"x":["A","B"]}}),
        json!({"id":"b"}),
    ];
    assert!(contract::stable_rows(a.clone(), a.clone()).is_ok());
    let mut changed = a.clone();
    changed[0]["description"] = json!("concurrent edit");
    assert!(contract::stable_rows(a.clone(), changed).is_err());
    let mut reversed = a.clone();
    reversed.reverse();
    assert!(contract::stable_rows(a, reversed).is_err());
}

#[tokio::test]
async fn contract_refuses_missing_metadata_even_when_generated_config_parser_accepts_it() {
    let body = json!({"contexts":[],"overrides":{},"default_configs":{},"dimensions":{}});
    let (api, _) = setup(
        vec![
            Fault::Reply(200, body.clone()),
            Fault::ConfigWithHeaders(body),
        ],
        1,
        Duration::from_secs(3),
    );
    let missing = api.call("config", json!({})).await.unwrap();
    assert!(missing.sdk_ok); // actual SDK defaulting is not metadata evidence
    assert!(missing.success().is_err());
    let reply = api.call("config", json!({})).await.unwrap();
    assert!(reply.sdk_ok);
    assert_eq!(reply.version.as_deref(), Some("fixture-version"));
    reply.success().unwrap();
}

#[tokio::test]
async fn bootstrap_never_writes_on_existing_workspace_or_permission_failure() {
    for status in [200, 401, 403, 500] {
        let (api, script) = setup(
            vec![Fault::Reply(
                status,
                json!({"message":"synthetic existing/denied workspace"}),
            )],
            1,
            Duration::from_secs(3),
        );
        let results = required_fields_api_contracts::suite::run(&api).await;
        contract::validate_results(&results, false).unwrap();
        assert_eq!(results[0].status, "failed");
        assert!(results.iter().skip(1).all(|r| r.status == "pending"));
        let trace = script.trace.lock().unwrap();
        assert_eq!(trace.len(), 1);
        assert_eq!(trace[0].method, "GET");
    }
}

#[test]
fn command_is_opt_in_and_ignores_ambient_target_configuration() {
    let binary = env!("CARGO_BIN_EXE_required-fields-api-contracts");
    let help = std::process::Command::new(binary)
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let no_args = std::process::Command::new(binary)
        .env("SUPERPOSITION_URL", "https://do-not-contact.invalid")
        .env("SUPERPOSITION_TOKEN", "synthetic-not-a-credential")
        .output()
        .unwrap();
    assert_eq!(no_args.status.code(), Some(2));
    assert!(String::from_utf8(no_args.stderr)
        .unwrap()
        .contains("--registration"));
}
