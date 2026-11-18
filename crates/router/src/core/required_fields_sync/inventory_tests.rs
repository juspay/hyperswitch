#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::{collections::VecDeque, sync::Mutex};

use serde_json::{json, Value};

use super::*;

pub(crate) fn n(value: &str) -> Name {
    value.to_owned().try_into().unwrap()
}
pub(crate) fn target() -> CertifiedReadTarget {
    CertifiedReadTarget {
        identity: TargetIdentity {
            target_id: n("synthetic-inventory"),
            organisation: n("synthetic_org"),
            workspace: n("synthetic_workspace"),
            instance_generation: 1,
        },
        endpoint: "https://synthetic.invalid/sp".parse().unwrap(),
        evidence: ReadEvidence {
            contract_id: n("synthetic-only-counted-pages"),
            client_version: "0.119.0".to_owned(),
            server_image: format!("sha256:{}", "1".repeat(64)).try_into().unwrap(),
            server_source: "2".repeat(40).try_into().unwrap(),
            reader_principal: n("synthetic-reader"),
            permission_receipt: format!("sha256:{}", "3".repeat(64)).try_into().unwrap(),
            scope: EvidenceScope::SyntheticFixture,
        },
        authorization_fingerprint: fingerprint("reader-authorization", b"Bearer synthetic-fixture")
            .unwrap(),
        pages: PAGED
            .into_iter()
            .map(|resource| {
                (
                    resource,
                    PageContract {
                        empty_total_pages: 0,
                    },
                )
            })
            .collect(),
        full_read_scope: true,
        all_experiment_history: true,
        latest_version_descending: true,
        empty_version_total_pages: 0,
    }
}
fn limits() -> InventoryLimits {
    InventoryLimits {
        page_size: 2,
        ..InventoryLimits::default()
    }
}

fn audit(mut value: Value, modified: &str) -> Value {
    for (key, v) in [
        ("description", "synthetic original description"),
        ("change_reason", "synthetic original reason"),
        ("created_by", "fixture@example.invalid"),
        ("last_modified_by", "fixture@example.invalid"),
        ("created_at", "2026-11-18T00:00:00Z"),
        (modified, "2026-11-18T00:00:00Z"),
    ] {
        value[key] = json!(v);
    }
    value
}
fn context_row(id: &str, connector: &str) -> Value {
    audit(
        json!({"id":id,"override_id":format!("override-{id}"),"weight":"47",
        "value":{"connector":connector,"payment_method":"Card","payment_method_type":"Debit"},
        "override":{"dynamic_fields.card.number._is_required":true,"unrelated.owner_config":{"keep":["B","A","A"],"enabled":false}},
        "unknown_metadata":{"a":[1,null,false],"preserve":"é"}}),
        "last_modified_at",
    )
}

fn rows(resource: Resource) -> Vec<Value> {
    match resource {
        Resource::Contexts => vec![context_row("server-A","adyen"),context_row("server-B","stripe"),context_row("server-C","worldpay")],
        Resource::Defaults => ["dynamic_fields.card.number._is_required","presentation.future","unrelated.threshold"].into_iter().map(|key| audit(json!({"key":key,"value":true,"schema":{"type":"boolean","unknown_constraint":{"keep":true}},"value_validation_function_name":"existing-validator","value_compute_function_name":null}),"last_modified_at")).collect(),
        Resource::Dimensions => ["connector","payment_method"].into_iter().enumerate().map(|(position, dimension)| audit(json!({"dimension":dimension,"position":position,"mandatory":false,"schema":{"type":"string","enum":["B","A"]},"dimension_type":{"REGULAR":{}},"dependency_graph":{},"value_compute_function_name":null,"value_validation_function_name":null}),"last_modified_at")).collect(),
        Resource::Experiments => [("exp-active","INPROGRESS",Some("server-C")),("exp-retained","DISCARDED",Some("historical-removed"))].into_iter().map(|(id,status,reference)| audit(json!({"id":id,"name":id,"experiment_type":"DEFAULT","override_keys":["dynamic_fields.card.number._is_required"],"status":status,"traffic_percentage":10,"context":{"connector":"worldpay"},"variants":[{"id":"variant-control","variant_type":"CONTROL","context_id":reference,"override_id":"existing-variant-override","overrides":{"dynamic_fields.card.number._is_required":false}}],"experiment_group_id":"group-one","metrics":{"unknown":["keep"]}}),"last_modified")).collect(),
        Resource::ExperimentGroups => vec![audit(json!({"id":"group-one","context_hash":"server-returned-group-hash","name":"synthetic-group","context":{"connector":"worldpay"},"traffic_percentage":20,"member_experiment_ids":["exp-active","exp-retained"],"buckets":[null,{"experiment_id":"exp-active","variant_id":"variant-control","future":"retained"}],"group_type":"USER_CREATED"}),"last_modified_at")],
        _ => panic!("not a collection"),
    }
}

fn workspace_row(version: Option<&str>) -> Value {
    let mut value = audit(
        json!({"workspace_name":"synthetic_workspace","organisation_id":"synthetic_org","organisation_name":"Synthetic","workspace_schema_name":"synthetic_schema","workspace_status":"ENABLED","workspace_admin_email":"fixture@example.invalid","config_version":version,"mandatory_dimensions":["connector"],"metrics":{},"allow_experiment_self_approval":false,"auto_populate_control":false,"enable_context_validation":true,"enable_change_reason_validation":true,"workspace_lock":null,"future_workspace_flags":{"keep":true}}),
        "last_modified_at",
    );
    value.as_object_mut().unwrap().remove("description");
    value.as_object_mut().unwrap().remove("change_reason");
    value
}
fn reply(value: Value) -> RawResponse {
    RawResponse {
        status: 200,
        content_type: Some("application/json; charset=utf-8".to_owned()),
        content_length: None,
        content_encoding: None,
        content_range: false,
        age: None,
        body: serde_json::to_vec(&value).unwrap(),
    }
}
fn version_reply(empty: bool) -> RawResponse {
    if empty {
        reply(json!({"total_items":0,"total_pages":0,"data":[]}))
    } else {
        reply(
            json!({"total_items":10,"total_pages":10,"data":[{"id":"server-version-10","config":{"contexts":[],"overrides":{},"default_configs":{}},"created_at":"2026-11-18T00:00:00Z","description":"full immutable version row","tags":["synthetic"],"unknown_version_metadata":{"keep":true}}]}),
        )
    }
}

#[derive(Clone)]
struct Step {
    request: ReadRequest,
    response: RawResponse,
    delay: Option<Duration>,
}
struct Script {
    steps: Mutex<VecDeque<Step>>,
    calls: Mutex<Vec<ReadRequest>>,
}
impl Script {
    fn new(steps: Vec<Step>) -> Self {
        Self {
            steps: Mutex::new(steps.into()),
            calls: Mutex::new(Vec::new()),
        }
    }
    fn calls(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}
impl ReadSource for Script {
    async fn read(
        &self,
        request: ReadRequest,
        _: Duration,
        _: usize,
    ) -> InventoryResult<RawResponse> {
        self.calls.lock().unwrap().push(request);
        let step = self
            .steps
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| failure(InventoryErrorKind::PartialResponse))?;
        if step.request != request {
            return Err(failure(InventoryErrorKind::InvalidRequest));
        }
        if let Some(delay) = step.delay {
            tokio::time::sleep(delay).await;
        }
        Ok(step.response)
    }
}
fn script(empty: bool) -> Vec<Step> {
    let mut result = vec![Step {
        request: ReadRequest::VersionHead,
        response: version_reply(empty),
        delay: None,
    }];
    for _ in 0..2 {
        for resource in PAGED {
            let rows = if empty { vec![] } else { rows(resource) };
            let total = rows.len();
            for page in 0..total.div_ceil(2).max(1) {
                let data = rows
                    .iter()
                    .skip(page * 2)
                    .take(2)
                    .cloned()
                    .collect::<Vec<_>>();
                result.push(Step { request: ReadRequest::Page { resource, number: u32::try_from(page + 1).unwrap(), count: 2 }, response: reply(json!({"total_items":total,"total_pages":total.div_ceil(2),"data":data,"future_envelope_metadata":{"keep":["B","A","A"]}})), delay: None });
            }
        }
        result.push(Step {
            request: ReadRequest::Workspace,
            response: reply(workspace_row(if empty {
                None
            } else {
                Some("server-version-10")
            })),
            delay: None,
        });
        result.push(Step {
            request: ReadRequest::VersionHead,
            response: version_reply(empty),
            delay: None,
        });
    }
    result
}
fn edit(step: &mut Step, update: impl FnOnce(&mut Value)) {
    let mut value = serde_json::from_slice(&step.response.body).unwrap();
    update(&mut value);
    step.response.body = serde_json::to_vec(&value).unwrap();
}
fn find(steps: &[Step], resource: Resource, number: u32, occurrence: usize) -> usize {
    steps
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.request
                == ReadRequest::Page {
                    resource,
                    number,
                    count: 2,
                }
        })
        .nth(occurrence)
        .unwrap()
        .0
}
async fn refused(steps: Vec<Step>, kind: InventoryErrorKind) {
    let error = discover(&Script::new(steps), &target(), limits())
        .await
        .unwrap_err();
    assert_eq!(error.current_context().kind, kind);
}

#[tokio::test]
async fn complete_synthetic_inventory_preserves_all_resources_and_server_ids() {
    let source = Script::new(script(false));
    let inventory = discover(&source, &target(), limits()).await.unwrap();
    assert_eq!(source.calls(), 19);
    assert_eq!(inventory.contexts().len(), 3);
    assert_eq!(inventory.defaults().len(), 3);
    assert_eq!(inventory.dimensions().len(), 2);
    assert_eq!(inventory.experiments().len(), 2);
    assert_eq!(inventory.experiment_groups().len(), 1);
    assert_eq!(
        inventory.proof().evidence.scope,
        EvidenceScope::SyntheticFixture
    );
    assert!(inventory.proof().provisional_without_writer_exclusion);
    assert_eq!(inventory.proof().identical_full_passes, 2);
    let row = &inventory.contexts()[&n("server-A")];
    assert_eq!(row.projection().id, n("server-A"));
    assert!(row.raw_json().contains("unrelated.owner_config"));
    assert_eq!(
        row.raw_field("unknown_metadata"),
        Some("{\"a\":[1,null,false],\"preserve\":\"é\"}")
    );
    assert!(inventory.defaults()[&n("presentation.future")]
        .raw_json()
        .contains("unknown_constraint"));
    assert!(inventory
        .workspace()
        .raw_json()
        .contains("future_workspace_flags"));
    assert!(inventory
        .raw_pages()
        .iter()
        .any(|p| p.raw_json().contains("future_envelope_metadata")));
    assert_eq!(
        inventory.experiments()[&n("exp-active")]
            .projection()
            .variants[0]
            .context_id,
        Some(n("server-C"))
    );
    assert_eq!(
        inventory.experiments()[&n("exp-retained")]
            .projection()
            .variants[0]
            .context_id,
        Some(n("historical-removed"))
    );
    assert_eq!(
        inventory.experiment_groups()[&n("group-one")]
            .projection()
            .member_experiment_ids,
        vec![n("exp-active"), n("exp-retained")]
    );
    assert!(inventory.version_observations()[0]
        .raw_json()
        .contains("unknown_version_metadata"));
}

#[tokio::test]
async fn empty_inventory_still_reads_every_resource_twice_and_proves_empty_version_head() {
    let source = Script::new(script(true));
    let inventory = discover(&source, &target(), limits()).await.unwrap();
    assert_eq!(source.calls(), 15);
    assert!(inventory.contexts().is_empty());
    assert_eq!(inventory.proof().version, None);
}

#[tokio::test]
async fn uncertified_permissions_or_resource_contracts_refuse_before_any_read() {
    assert!(CertifiedReadTarget::from_registry(&n("candidate-0-115-5")).is_err());
    for case in 0..4 {
        let mut target = target();
        match case {
            0 => target.full_read_scope = false,
            1 => target.all_experiment_history = false,
            2 => target.latest_version_descending = false,
            _ => {
                target.pages.remove(&Resource::ExperimentGroups);
            }
        }
        let source = Script::new(script(false));
        assert!(discover(&source, &target, limits()).await.is_err());
        assert_eq!(source.calls(), 0);
    }
}

#[tokio::test]
async fn every_collection_rejects_duplicate_ids_across_pages_or_within_page() {
    for resource in PAGED {
        let mut steps = script(false);
        let first = find(&steps, resource, 1, 0);
        let data: Value = serde_json::from_slice(&steps[first].response.body).unwrap();
        let duplicate = data["data"][0].clone();
        if let Some(index) = steps.iter().position(|s| {
            s.request
                == ReadRequest::Page {
                    resource,
                    number: 2,
                    count: 2,
                }
        }) {
            edit(&mut steps[index], |v| v["data"][0] = duplicate);
        } else {
            edit(&mut steps[first], |v| {
                v["data"] = json!([duplicate.clone(), duplicate]);
                v["total_items"] = json!(2);
                v["total_pages"] = json!(1);
            });
        }
        refused(steps, InventoryErrorKind::DuplicateIdentity).await;
    }
}

#[tokio::test]
async fn changed_totals_boundaries_short_pages_and_dropped_pages_refuse() {
    for case in 0..5 {
        let mut steps = script(false);
        let index = find(&steps, Resource::Contexts, 2, 0);
        edit(&mut steps[index], |v| match case {
            0 => v["total_items"] = json!(4),
            1 => v["total_pages"] = json!(3),
            2 => v["data"] = json!([]),
            3 => v["page"] = json!(3),
            _ => {
                v.as_object_mut().unwrap().remove("total_items");
            }
        });
        assert!(discover(&Script::new(steps), &target(), limits())
            .await
            .is_err());
    }
    let mut steps = script(false);
    let index = find(&steps, Resource::Contexts, 2, 0);
    steps.remove(index);
    refused(steps, InventoryErrorKind::InvalidRequest).await;
}

#[tokio::test]
async fn repeated_page_with_plausible_totals_is_not_completeness() {
    let mut steps = script(false);
    let first = find(&steps, Resource::Contexts, 1, 0);
    let second = find(&steps, Resource::Contexts, 2, 0);
    let v: Value = serde_json::from_slice(&steps[first].response.body).unwrap();
    edit(&mut steps[second], |next| {
        next["data"] = json!([v["data"][0].clone()])
    });
    refused(steps, InventoryErrorKind::DuplicateIdentity).await;
}

#[tokio::test]
async fn filtered_or_truncated_envelopes_refuse_instead_of_becoming_unknown_metadata() {
    for (key, value) in [
        ("prefix", json!([])),
        ("filters", json!({})),
        ("scope", json!("tenant")),
        ("status", json!(["INPROGRESS"])),
        ("from_date", json!("recent")),
        ("partial", json!(true)),
        ("truncated", json!(true)),
        ("all", json!(true)),
    ] {
        let mut steps = script(false);
        let index = find(&steps, Resource::Contexts, 1, 0);
        edit(&mut steps[index], |v| v[key] = value);
        assert!(discover(&Script::new(steps), &target(), limits())
            .await
            .is_err());
    }
}

#[tokio::test]
async fn http_partial_permission_redirect_missing_and_wrong_media_responses_refuse() {
    for (status, kind) in [
        (206, InventoryErrorKind::PartialResponse),
        (401, InventoryErrorKind::PermissionDenied),
        (403, InventoryErrorKind::PermissionDenied),
        (302, InventoryErrorKind::Redirect),
        (304, InventoryErrorKind::Redirect),
        (404, InventoryErrorKind::HttpStatus),
        (500, InventoryErrorKind::HttpStatus),
    ] {
        let mut steps = script(false);
        steps[0].response.status = status;
        refused(steps, kind).await;
    }
    for case in 0..5 {
        let mut steps = script(false);
        match case {
            0 => steps[0].response.content_length = Some(1),
            1 => steps[0].response.content_range = true,
            2 => steps[0].response.content_type = None,
            3 => steps[0].response.content_encoding = Some("gzip".into()),
            _ => steps[0].response.age = Some("1".into()),
        }
        assert!(discover(&Script::new(steps), &target(), limits())
            .await
            .is_err());
    }
}

#[tokio::test]
async fn limits_are_refusals_not_successful_truncation() {
    for case in 0..6 {
        let source = Script::new(script(false));
        let mut bounds = limits();
        match case {
            0 => bounds.max_response_bytes = 10,
            1 => {
                bounds.max_response_bytes = 1024;
                bounds.max_total_bytes = 1024;
            }
            2 => bounds.max_items_per_resource = 2,
            3 => bounds.max_pages_per_resource = 1,
            4 => bounds.max_requests = 2,
            _ => bounds.max_total_items = 2,
        }
        assert!(discover(&source, &target(), bounds).await.is_err());
    }
    let source = Script::new(script(false));
    let bounds = InventoryLimits {
        page_size: 0,
        ..limits()
    };
    assert!(discover(&source, &target(), bounds).await.is_err());
    assert_eq!(source.calls(), 0);
}

#[tokio::test]
async fn deadline_cancels_a_read_without_retry_or_partial_result() {
    let mut steps = script(false);
    steps[0].delay = Some(Duration::from_millis(50));
    let source = Script::new(steps);
    let bounds = InventoryLimits {
        request_timeout: Duration::from_millis(1),
        total_timeout: Duration::from_millis(5),
        ..limits()
    };
    assert_eq!(
        discover(&source, &target(), bounds)
            .await
            .unwrap_err()
            .current_context()
            .kind,
        InventoryErrorKind::Deadline
    );
    assert_eq!(source.calls(), 1);
}

#[tokio::test]
async fn missing_fields_invalid_dates_and_malformed_rows_in_all_resources_refuse() {
    for (resource, key) in [
        (Resource::Contexts, "override"),
        (Resource::Defaults, "schema"),
        (Resource::Dimensions, "dependency_graph"),
        (Resource::Experiments, "variants"),
        (Resource::ExperimentGroups, "member_experiment_ids"),
    ] {
        let mut steps = script(false);
        let index = find(&steps, resource, 1, 0);
        edit(&mut steps[index], |v| {
            v["data"][0].as_object_mut().unwrap().remove(key);
        });
        assert!(discover(&Script::new(steps), &target(), limits())
            .await
            .is_err());
    }
    let mut steps = script(false);
    let index = find(&steps, Resource::Contexts, 1, 0);
    edit(&mut steps[index], |v| {
        v["data"][0]["created_at"] = json!("invalid date")
    });
    refused(steps, InventoryErrorKind::MalformedResponse).await;
    let mut steps = script(false);
    let index = find(&steps, Resource::Experiments, 1, 0);
    edit(&mut steps[index], |v| {
        v["data"][0]["status"] = json!("NEW_UNCERTIFIED_STATUS")
    });
    refused(steps, InventoryErrorKind::MalformedResponse).await;
}

#[tokio::test]
async fn raw_json_duplicate_members_trailing_bodies_and_numeric_totals_refuse() {
    for bytes in [
        r#"{"total_items":0,"total_items":0,"total_pages":0,"data":[]}"#,
        r#"{"total_items":0,"total_pages":0,"data":[],"x":{"a":1,"\u0061":2}}"#,
        r#"{"total_items":0,"total_pages":0,"data":[]}{}"#,
        r#"{"total_items":1.0,"total_pages":1,"data":[]}"#,
        r#"{"total_items":"0","total_pages":0,"data":[]}"#,
        r#"{"total_items":0,"total_pages":0,"data":null}"#,
    ] {
        let mut steps = script(true);
        steps[0].response.body = bytes.as_bytes().to_vec();
        assert!(discover(&Script::new(steps), &target(), limits())
            .await
            .is_err());
    }
}

#[tokio::test]
async fn same_version_same_counts_but_full_metadata_or_order_changes_refuse() {
    for resource in PAGED {
        let mut steps = script(false);
        let index = find(&steps, resource, 1, 1);
        edit(&mut steps[index], |v| {
            v["data"][0]["unknown_metadata"] = json!({"changed":true})
        });
        refused(steps, InventoryErrorKind::InventoryChanged).await;
    }
    let mut steps = script(false);
    let index = find(&steps, Resource::Contexts, 1, 1);
    edit(&mut steps[index], |v| {
        v["data"][0]["override"]["unrelated.owner_config"]["keep"] = json!(["A", "B", "A"])
    });
    refused(steps, InventoryErrorKind::InventoryChanged).await;
    let mut steps = script(false);
    let index = steps
        .iter()
        .rposition(|s| s.request == ReadRequest::Workspace)
        .unwrap();
    edit(&mut steps[index], |v| {
        v["future_workspace_flags"]["keep"] = json!(false)
    });
    refused(steps, InventoryErrorKind::InventoryChanged).await;
}

#[tokio::test]
async fn version_head_and_workspace_identity_are_checked_at_every_boundary() {
    for boundary in [1, 2] {
        let mut steps = script(false);
        let index = steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.request == ReadRequest::VersionHead)
            .nth(boundary)
            .unwrap()
            .0;
        edit(&mut steps[index], |v| {
            v["data"][0]["id"] = json!("different-version")
        });
        refused(steps, InventoryErrorKind::VersionChanged).await;
    }
    for key in ["workspace_name", "organisation_id", "config_version"] {
        let mut steps = script(false);
        let index = steps
            .iter()
            .position(|s| s.request == ReadRequest::Workspace)
            .unwrap();
        edit(&mut steps[index], |v| v[key] = json!("different"));
        assert!(discover(&Script::new(steps), &target(), limits())
            .await
            .is_err());
    }
}

#[tokio::test]
async fn duplicate_conditions_and_observed_ordered_override_id_collisions_refuse() {
    let mut steps = script(false);
    let index = find(&steps, Resource::Contexts, 1, 0);
    edit(
        &mut steps[index],
        |v| {
            v["data"][1]["value"] =
                json!({"payment_method_type":"Debit","payment_method":"Card","connector":"adyen"})
        },
    );
    refused(steps, InventoryErrorKind::AmbiguousCondition).await;
    let mut steps = script(false);
    let index = find(&steps, Resource::Contexts, 1, 0);
    edit(&mut steps[index], |v| {
        v["data"][1]["override_id"] = json!("override-server-A");
        v["data"][1]["override"]["unrelated.owner_config"]["keep"] = json!(["A", "B", "A"]);
    });
    refused(steps, InventoryErrorKind::OverrideIdentityCollision).await;
}

#[tokio::test]
async fn unsupported_predicates_are_quarantined_and_retained_never_owned_equalities() {
    let mut steps = script(false);
    for occurrence in [0, 1] {
        let index = find(&steps, Resource::Contexts, 1, occurrence);
        edit(&mut steps[index], |v| {
            v["data"][0]["value"] = json!({"and":[{"==":[{"var":"connector"},"adyen"]}]})
        });
    }
    let inventory = discover(&Script::new(steps), &target(), limits())
        .await
        .unwrap();
    let row = &inventory.contexts()[&n("server-A")];
    assert_eq!(
        row.projection().condition,
        ObservedCondition::QuarantinedUnsupportedPredicate
    );
    assert!(row.raw_json().contains("and"));
    for value in [json!(null), json!([]), json!("connector=adyen")] {
        let mut steps = script(false);
        let index = find(&steps, Resource::Contexts, 1, 0);
        edit(&mut steps[index], |v| v["data"][0]["value"] = value);
        assert!(discover(&Script::new(steps), &target(), limits())
            .await
            .is_err());
    }
}

#[test]
fn canonical_raw_json_preserves_numbers_and_arrays_without_a_serde_value_roundtrip() {
    let json=Json::parse(br#"{"z":1.2345678901234567890123456789,"big":1844674407370955161612345,"a":["B","A","A"]}"#).unwrap();
    assert_eq!(
        String::from_utf8(json.canonical().unwrap()).unwrap(),
        r#"{"a":["B","A","A"],"big":1844674407370955161612345,"z":1.2345678901234567890123456789}"#
    );
    assert_eq!(
        Json::parse(br#"{"b":2,"a":1}"#)
            .unwrap()
            .canonical()
            .unwrap(),
        Json::parse(br#"{"a":1,"b":2}"#)
            .unwrap()
            .canonical()
            .unwrap()
    );
    assert_ne!(
        Json::parse(b"1.0").unwrap().canonical().unwrap(),
        Json::parse(b"1").unwrap().canonical().unwrap()
    );
    assert!(Json::parse(format!("{}0{}", "[".repeat(34), "]".repeat(34)).as_bytes()).is_err());
}

#[tokio::test]
async fn errors_and_debug_do_not_echo_response_bodies_or_credentials() {
    let mut steps = script(false);
    steps[0].response.body = b"synthetic-sensitive-never-print".to_vec();
    let error = discover(&Script::new(steps), &target(), limits())
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("synthetic-sensitive"));
    assert!(!format!(
        "{:?}",
        Json::parse(br#"{"hidden":"synthetic-sensitive"}"#).unwrap()
    )
    .contains("synthetic-sensitive"));
}

#[tokio::test]
async fn empty_boundaries_follow_each_resource_contract_including_versions() {
    let mut target = target();
    target.empty_version_total_pages = 1;
    target
        .pages
        .get_mut(&Resource::Defaults)
        .unwrap()
        .empty_total_pages = 1;
    let mut steps = script(true);
    for step in &mut steps {
        if matches!(
            step.request,
            ReadRequest::VersionHead
                | ReadRequest::Page {
                    resource: Resource::Defaults,
                    ..
                }
        ) {
            edit(step, |v| v["total_pages"] = json!(1));
        }
    }
    assert!(discover(&Script::new(steps.clone()), &target, limits())
        .await
        .is_ok());
    target
        .pages
        .get_mut(&Resource::Defaults)
        .unwrap()
        .empty_total_pages = 0;
    assert_eq!(
        discover(&Script::new(steps), &target, limits())
            .await
            .unwrap_err()
            .current_context()
            .kind,
        InventoryErrorKind::PaginationMismatch
    );
    target.empty_version_total_pages = 2;
    let source = Script::new(script(true));
    assert!(discover(&source, &target, limits()).await.is_err());
    assert_eq!(source.calls(), 0);
}

#[tokio::test]
async fn full_inventory_retains_unknown_numeric_lexemes_and_object_order_is_not_drift() {
    let mut steps = script(false);
    let lexeme = "184467440737095516161234567890123456789";
    for occurrence in [0, 1] {
        let index = find(&steps, Resource::Contexts, 1, occurrence);
        let body = String::from_utf8(steps[index].response.body.clone()).unwrap();
        steps[index].response.body = body
            .replace(
                "\"unknown_metadata\":{",
                &format!("\"unknown_metadata\":{{\"large\":{lexeme},"),
            )
            .into_bytes();
    }
    let inventory = discover(&Script::new(steps.clone()), &target(), limits())
        .await
        .unwrap();
    assert!(inventory.contexts()[&n("server-A")]
        .raw_field("unknown_metadata")
        .unwrap()
        .contains(lexeme));
    let index = find(&steps, Resource::Contexts, 1, 1);
    let body = String::from_utf8(steps[index].response.body.clone()).unwrap();
    steps[index].response.body = body
        .replace(
            "\"connector\":\"adyen\",\"payment_method\":\"Card\"",
            "\"payment_method\":\"Card\",\"connector\":\"adyen\"",
        )
        .into_bytes();
    let reordered = discover(&Script::new(steps), &target(), limits())
        .await
        .unwrap();
    assert_eq!(
        inventory.proof().inventory_hash,
        reordered.proof().inventory_hash
    );
    let mut reordered_rows = script(false);
    let index = find(&reordered_rows, Resource::Contexts, 1, 1);
    edit(&mut reordered_rows[index], |v| {
        v["data"].as_array_mut().unwrap().swap(0, 1)
    });
    refused(reordered_rows, InventoryErrorKind::InventoryChanged).await;
}

#[tokio::test]
async fn proof_binds_target_generation_principal_contract_and_permission_receipt() {
    let baseline = discover(&Script::new(script(false)), &target(), limits())
        .await
        .unwrap();
    for case in 0..6 {
        let mut changed = target();
        match case {
            0 => changed.identity.instance_generation += 1,
            1 => changed.identity.target_id = n("another-registration"),
            2 => changed.evidence.reader_principal = n("another-reader"),
            3 => changed.evidence.contract_id = n("another-contract"),
            4 => {
                changed.evidence.permission_receipt =
                    format!("sha256:{}", "4".repeat(64)).try_into().unwrap()
            }
            _ => {
                changed.authorization_fingerprint =
                    fingerprint("reader-authorization", b"Bearer another-synthetic-reader").unwrap()
            }
        }
        let result = discover(&Script::new(script(false)), &changed, limits())
            .await
            .unwrap();
        assert_ne!(
            baseline.proof().inventory_hash,
            result.proof().inventory_hash
        );
    }
}

#[tokio::test]
async fn cohorts_are_retained_without_computation_and_malformed_nested_fields_refuse() {
    let mut steps = script(false);
    for occurrence in [0, 1] {
        let index = find(&steps, Resource::Dimensions, 1, occurrence);
        edit(&mut steps[index], |v| {
            v["data"][0]["dimension_type"] = json!({"LOCAL_COHORT":"existing-local-function"});
            v["data"][1]["dimension_type"] = json!({"REMOTE_COHORT":"existing-remote-function"});
            v["data"][1]["dependency_graph"] = json!({"payment_method":["connector"]});
        });
    }
    let inventory = discover(&Script::new(steps), &target(), limits())
        .await
        .unwrap();
    assert!(
        matches!(&inventory.dimensions()[&n("connector")].projection().kind, DimensionKind::LocalCohort(name) if name == "existing-local-function")
    );
    assert!(
        matches!(&inventory.dimensions()[&n("payment_method")].projection().kind, DimensionKind::RemoteCohort(name) if name == "existing-remote-function")
    );
    for (resource, key, value) in [
        (Resource::Dimensions, "dimension_type", json!("REGULAR")),
        (Resource::Dimensions, "dimension_type", json!({"FUTURE":{}})),
        (
            Resource::Dimensions,
            "dimension_type",
            json!({"LOCAL_COHORT":false}),
        ),
        (
            Resource::Dimensions,
            "dependency_graph",
            json!({"x":[false]}),
        ),
        (
            Resource::Experiments,
            "started_at",
            json!("not-a-timestamp"),
        ),
        (Resource::Experiments, "experiment_type", json!("FUTURE")),
        (Resource::Experiments, "variants", json!([])),
        (
            Resource::ExperimentGroups,
            "buckets",
            json!([{"experiment_id":"exp-active"}]),
        ),
        (
            Resource::ExperimentGroups,
            "member_experiment_ids",
            json!(["same", "same"]),
        ),
    ] {
        let mut steps = script(false);
        let index = find(&steps, resource, 1, 0);
        edit(&mut steps[index], |v| v["data"][0][key] = value);
        assert!(
            discover(&Script::new(steps), &target(), limits())
                .await
                .is_err(),
            "{resource:?}/{key}"
        );
    }
    for (key, value) in [
        ("workspace_status", json!("FUTURE")),
        (
            "workspace_lock",
            json!({"lock_id":"existing","operation":"existing","locked_by":"synthetic","acquired_at":"invalid","expires_at":"invalid"}),
        ),
        ("metrics", json!(null)),
    ] {
        let mut steps = script(false);
        let index = steps
            .iter()
            .position(|s| s.request == ReadRequest::Workspace)
            .unwrap();
        edit(&mut steps[index], |v| {
            v[key] = value;
            if key == "metrics" {
                v.as_object_mut().unwrap().remove(key);
            }
        });
        assert!(discover(&Script::new(steps), &target(), limits())
            .await
            .is_err());
    }
}

#[test]
fn json_node_limit_refuses_a_shallow_but_oversized_structure() {
    let bytes = format!("[{}]", vec!["0"; 100_000].join(","));
    assert_eq!(
        Json::parse(bytes.as_bytes())
            .unwrap_err()
            .current_context()
            .kind,
        InventoryErrorKind::JsonLimit
    );
}
