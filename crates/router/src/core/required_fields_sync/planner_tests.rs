//! Independent synthetic planner fixtures. Passing these does not certify a server or a browser.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::time::Duration;

use serde_json::{json as v, Value};

use super::{
    super::{
        inventory::{RawResponse, ReadRequest, ReadSource},
        ownership::*,
        preflight::*,
        types::*,
        *,
    },
    *,
};

#[cfg(feature = "required_fields_sync")]
#[path = "command_tests.rs"]
mod command_tests;

const REQUIRED: &str = "dynamic_fields.choice._is_required";
const OPTIONS: &str = "dynamic_fields.choice._dropdown_options";
fn n(s: &str) -> Name {
    s.to_owned().try_into().unwrap()
}
fn d(s: &str) -> Digest {
    digest(&s).unwrap()
}
fn key(s: &str) -> ManagedKey {
    s.to_owned().try_into().unwrap()
}
fn condition(connector: &str) -> ContextKey {
    ContextKey {
        connector: n(connector),
        payment_method: n("Card"),
        payment_method_type: n("Debit"),
        mandate_type: None,
    }
}
fn audit(mut row: Value, modified: &str) -> Value {
    for (k, v) in [
        ("description", "manual description — preserve"),
        ("change_reason", "manual reason"),
        ("created_by", "fixture@example.invalid"),
        ("last_modified_by", "fixture@example.invalid"),
        ("created_at", "2026-11-18T00:00:00Z"),
        (modified, "2026-11-18T00:00:00Z"),
    ] {
        row[k] = v!(v);
    }
    row
}
fn row(id: &str, condition: Value, values: Value) -> Value {
    audit(
        v!({"id":id,"override_id":format!("override-{id}"),"weight":condition.as_object().unwrap().keys().map(|k|match k.as_str(){"connector"=>1,"payment_method"=>2,"payment_method_type"=>4,"mandate_type"=>8,_=>16}).sum::<u64>().to_string(),"value":condition,"override":values,
        "future_metadata":{"nested":["B","A","A"],"keep":false}}),
        "last_modified_at",
    )
}
fn default_row(key: &str, value: Value) -> Value {
    let schema = if value.is_boolean() {
        v!({"type":"boolean"})
    } else if value.is_array() {
        v!({"type":"array","items":{"type":"string"}})
    } else if value.is_number() {
        v!({"type":"integer"})
    } else {
        v!({"type":"string"})
    };
    audit(
        v!({"key":key,"value":value,"schema":schema,"value_validation_function_name":null,"value_compute_function_name":null}),
        "last_modified_at",
    )
}
fn dimension(name: &str, position: i32, values: Value) -> Value {
    audit(
        v!({"dimension":name,"position":position,"mandatory":false,"schema":{"type":"string","enum":values,"description":"keep schema text"},"dimension_type":{"REGULAR":{}},"dependency_graph":{},"value_validation_function_name":null,"value_compute_function_name":null}),
        "last_modified_at",
    )
}
fn metadata() -> BTreeMap<String, Value> {
    [
        ("field_render_type", v!("Dropdown")),
        (
            "confirm_request_write_path",
            v!("payment_method_data.choice"),
        ),
        ("intent_data_read_path", v!("choice")),
        ("default_label_text", v!("Choose")),
        ("field_display_order", v!(1)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}
#[derive(Clone)]
struct Fixture {
    contexts: Vec<Value>,
    defaults: Vec<Value>,
    dimensions: Vec<Value>,
    experiments: Vec<Value>,
    groups: Vec<Value>,
    workspace: Value,
    corrupt_export: bool,
}
impl Fixture {
    fn new() -> Self {
        let mut defaults = vec![
            default_row(REQUIRED, v!(false)),
            default_row(OPTIONS, v!(["A", "B"])),
        ];
        defaults.extend(
            metadata()
                .into_iter()
                .map(|(key, value)| default_row(&format!("dynamic_fields.choice._{key}"), value)),
        );
        let mut workspace = audit(
            v!({"workspace_name":"synthetic_workspace","organisation_id":"synthetic_org","organisation_name":"Synthetic","workspace_schema_name":"fixture_schema","workspace_status":"ENABLED","workspace_admin_email":"fixture@example.invalid","config_version":"version-1","mandatory_dimensions":[],"metrics":{},"allow_experiment_self_approval":false,"auto_populate_control":false,"enable_context_validation":true,"enable_change_reason_validation":true,"workspace_lock":null}),
            "last_modified_at",
        );
        workspace.as_object_mut().unwrap().remove("description");
        workspace.as_object_mut().unwrap().remove("change_reason");
        Self {
            contexts: vec![row(
                "server-A",
                v!(condition("a")),
                v!({REQUIRED:true,OPTIONS:["A","B"]}),
            )],
            defaults,
            dimensions: vec![
                dimension("connector", 0, v!(["a", "b"])),
                dimension("payment_method", 1, v!(["Card"])),
                dimension("payment_method_type", 2, v!(["Debit", "Credit"])),
                dimension("mandate_type", 3, v!(["mandate", "non_mandate"])),
            ],
            experiments: vec![],
            groups: vec![],
            workspace,
            corrupt_export: false,
        }
    }
    fn export(&self) -> Value {
        let mut rows = self.contexts.clone();
        rows.sort_by_key(|row| {
            let weight: u64 = row["value"]
                .as_object()
                .unwrap()
                .keys()
                .map(|key| {
                    1_u64
                        << self
                            .dimensions
                            .iter()
                            .find(|d| d["dimension"] == key.as_str())
                            .map(|d| d["position"].as_u64().unwrap())
                            .unwrap_or(0)
                })
                .sum();
            (
                weight,
                row["created_at"].as_str().unwrap().to_owned(),
                row["id"].as_str().unwrap().to_owned(),
            )
        });
        let mut overrides = serde_json::Map::new();
        let contexts:Vec<_>=rows.iter().map(|row|{overrides.insert(row["override_id"].as_str().unwrap().to_owned(),row["override"].clone());v!({"id":row["id"],"condition":row["value"],"override_with_keys":[row["override_id"]],"weight":0,"priority":0})}).collect();
        if self.corrupt_export {
            overrides.clear();
        }
        v!({"contexts":contexts,"overrides":overrides,"default_configs":self.defaults.iter().map(|r|(r["key"].as_str().unwrap().to_owned(),r["value"].clone())).collect::<BTreeMap<_,_>>()})
    }
    async fn inventory(&self) -> CompleteInventory {
        inventory::discover(
            self,
            &inventory::tests::target(),
            InventoryLimits::default(),
        )
        .await
        .unwrap()
    }
}
impl ReadSource for Fixture {
    async fn read(
        &self,
        request: ReadRequest,
        _: Duration,
        _: usize,
    ) -> InventoryResult<RawResponse> {
        let value = match request {
            ReadRequest::Page {
                resource,
                number,
                count,
            } => {
                assert_eq!(number, 1);
                assert_eq!(count, 100);
                let rows = match resource {
                    Resource::Contexts => &self.contexts,
                    Resource::Defaults => &self.defaults,
                    Resource::Dimensions => &self.dimensions,
                    Resource::Experiments => &self.experiments,
                    Resource::ExperimentGroups => &self.groups,
                    _ => panic!("not a collection"),
                };
                v!({"total_items":rows.len(),"total_pages":usize::from(!rows.is_empty()),"data":rows})
            }
            ReadRequest::Workspace => self.workspace.clone(),
            ReadRequest::VersionHead => {
                v!({"total_items":1,"total_pages":1,"data":[{"id":"version-1","config":self.export(),"created_at":"2026-11-18T00:00:00Z","description":"synthetic immutable version","tags":[]}]})
            }
        };
        Ok(RawResponse {
            status: 200,
            content_type: Some("application/json".to_owned()),
            content_length: None,
            content_encoding: None,
            content_range: false,
            age: None,
            body: serde_json::to_vec(&value).unwrap(),
        })
    }
}
fn desired() -> VerifiedDesired {
    let release = ReleaseIdentity {
        application_revision: "4".repeat(40).try_into().unwrap(),
        source_tree_hash: d("source"),
        artifact: ArtifactIdentity::Offline,
        renderer_implementation: n("synthetic-renderer"),
    };
    let release_hash = digest(&release).unwrap();
    let dimensions: BTreeMap<Name, Vec<Name>> = [
        ("connector", vec![n("a"), n("b")]),
        ("payment_method", vec![n("Card")]),
        ("payment_method_type", vec![n("Debit"), n("Credit")]),
        ("mandate_type", vec![n("mandate"), n("non_mandate")]),
    ]
    .into_iter()
    .map(|(k, v)| (n(k), v))
    .collect();
    let catalog = FieldContractCatalog {
        schema_version: 1,
        contract_id: n("synthetic-fields"),
        sdk: SdkContract {
            contract_id: n("synthetic-sdk"),
            web_revision: "5".repeat(40).try_into().unwrap(),
            shared_revision: "6".repeat(40).try_into().unwrap(),
            evaluator_hash: d("synthetic-evaluator"),
            dimensions: dimensions.clone(),
        },
        fields: BTreeMap::from([(
            n("choice"),
            Decision::Approved {
                value: FieldContract {
                    render_kind: RenderKind::Dropdown,
                    write_path: n("payment_method_data.choice"),
                    read_path: n("choice"),
                    review: n("fixture-review"),
                    source_types: BTreeSet::from([n("DropDown")]),
                    default_options: Some(vec!["A".to_owned(), "B".to_owned()]),
                },
                review: n("fixture-review"),
            },
        )]),
        unrestricted_countries: Decision::Refused {
            reason: n("not-in-scope"),
        },
    };
    let desired = DesiredState {
        schema_version: 2,
        contexts: BTreeMap::from([(
            condition("a"),
            BTreeMap::from([
                (key(REQUIRED), ManagedValue::Required(true)),
                (
                    key(OPTIONS),
                    ManagedValue::Options(vec!["A".to_owned(), "B".to_owned()]),
                ),
            ]),
        )]),
        prerequisites: PrerequisiteRequirements {
            dimension_labels: dimensions
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
            field_contracts: BTreeSet::from([n("choice")]),
        },
        input_manifest: InputManifest {
            schema_version: 1,
            canonical_version: 1,
            build: BuildIdentity::for_offline_renderer(release, "V1,REQUIRED_FIELDS_SYNC").unwrap(),
            settings_provenance: SettingsProvenance::EffectiveValuesOnly,
            effective_settings_hash: d("settings"),
            retained_rules_hash: d("retained"),
            mapping_hash: d("mapping"),
            catalog_hash: d("catalog"),
            exclusions_hash: d("exclusions"),
        },
        diagnostics: vec![],
    };
    VerifiedDesired {
        desired,
        catalog,
        bundle_hash: d("synthetic-bundle"),
        release_hash,
        source_availability_hash: d("feature-scope"),
        excluded_connectors: BTreeSet::new(),
    }
}
fn prereqs(desired: &VerifiedDesired) -> PrerequisitePolicy {
    PrerequisitePolicy {
        schema_version: 1,
        contract_id: n("synthetic-prerequisites"),
        dimensions: [
            "connector",
            "payment_method",
            "payment_method_type",
            "mandate_type",
        ]
        .into_iter()
        .enumerate()
        .map(|(pos, key)| {
            (
                n(key),
                DimensionRule {
                    position: pos as i32,
                    mandatory: false,
                    vocabulary: desired.catalog.sdk.dimensions[&n(key)]
                        .iter()
                        .cloned()
                        .collect(),
                    allow_enum_extension: false,
                    review: n("fixture-review"),
                },
            )
        })
        .collect(),
        workspace_flags: BTreeMap::from([
            ("allow_experiment_self_approval".to_owned(), false),
            ("auto_populate_control".to_owned(), false),
            ("enable_context_validation".to_owned(), true),
            ("enable_change_reason_validation".to_owned(), true),
        ]),
        mandatory_dimensions: BTreeSet::new(),
        fields: BTreeMap::from([(
            n("choice"),
            FieldRule {
                metadata: metadata(),
                allowed_options: Some(vec!["A".to_owned(), "B".to_owned()]),
                review: n("fixture-review"),
            },
        )]),
        create_defaults: vec![],
    }
}
fn grants(inventory: &CompleteInventory) -> OwnershipManifest {
    OwnershipManifest {
        schema_version: 1,
        target: inventory.proof().target.clone(),
        owner: n("fixture-owner"),
        revision: n("revision-1"),
        source_availability_hash: d("feature-scope"),
        grants: inventory
            .contexts()
            .iter()
            .filter_map(|(id, row)| {
                ownership::context_key(&row.raw)
                    .unwrap()
                    .map(|condition| AdoptionGrant {
                        server_id: id.clone(),
                        condition,
                        observed_row_hash: observed_row_hash(inventory, id).unwrap(),
                        attributes: BTreeSet::from([key(REQUIRED), key(OPTIONS)]),
                        allow_row_deletion: false,
                        approver: n("fixture-owner"),
                        review: n("fixture-review"),
                    })
            })
            .collect(),
        creations: vec![],
        excluded_conditions: vec![],
        retirement: None,
    }
}
// Deliberately synthetic identity. Actual core/server identity is checked separately, not certified here.
fn synthetic_identity(raw: &str) -> PlanResult<String> {
    serde_json::to_string(&hash(&checked(Json::parse(raw.as_bytes()))?)?)
        .map_err(|_| refuse(PlanCode::InvalidInput))
}
fn contract(inventory: &CompleteInventory, desired: &VerifiedDesired) -> ProjectionContract {
    let sdk = &desired.catalog.sdk;
    let evidence = &inventory.proof().evidence;
    let stages = [
        SdkStage::JsResolver,
        SdkStage::RustResolver,
        SdkStage::BrowserComponents,
        SdkStage::Checkout,
    ];
    ProjectionContract {
        id: n("synthetic-projection"),
        target: inventory.proof().target.clone(),
        authority: n("fixture-authority"),
        read_contract: evidence.contract_id.clone(),
        server_image: evidence.server_image.clone(),
        server_source: evidence.server_source.clone(),
        scope: EvidenceScope::SyntheticFixture,
        receipt: d("synthetic-only-receipt"),
        sdk_contract: sdk.contract_id.clone(),
        web_revision: sdk.web_revision.clone(),
        shared_revision: sdk.shared_revision.clone(),
        evaluator: sdk.evaluator_hash.clone(),
        case_pack: d("synthetic-case-pack"),
        provenance: v!({"synthetic_fixture":true}),
        artifacts: stages
            .iter()
            .map(|s| (s.clone(), d("synthetic-artifact")))
            .collect(),
        stage_cases: stages
            .into_iter()
            .map(|s| (s, BTreeSet::from([n("independent-fixture-case")])))
            .collect(),
        connector_sets: vec![BTreeSet::from([n("a"), n("b")])],
        browser_scenarios: BTreeSet::from([n("empty-intent"), n("prefilled-intent")]),
        override_identity: synthetic_identity,
        trusted_report_digests: vec![],
    }
}
fn intent(desired: &VerifiedDesired) -> PlanIntent {
    PlanIntent {
        authority: n("fixture-authority"),
        release_hash: desired.release_hash.clone(),
        generation: 1,
    }
}
fn candidate(
    inventory: &CompleteInventory,
    desired: &VerifiedDesired,
    grants: &OwnershipManifest,
    policy: &PrerequisitePolicy,
    contract: &ProjectionContract,
) -> PlanResult<PlanCandidate> {
    prepare(
        inventory,
        desired,
        grants,
        policy,
        intent(desired),
        contract,
        PlanLimits::default(),
    )
}
fn code<T>(result: PlanResult<T>) -> PlanCode {
    match result {
        Err(e) => e.current_context().code,
        Ok(_) => panic!("expected refusal"),
    }
}

#[tokio::test]
async fn unchanged_adoption_is_not_a_write_or_browser_acceptance() {
    let inventory = Fixture::new().inventory().await;
    let desired = desired();
    let contract = contract(&inventory, &desired);
    let a = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract,
    )
    .unwrap();
    let b = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract,
    )
    .unwrap();
    assert!(a.actions.is_empty());
    assert_eq!(a.report.candidate_hash, b.report.candidate_hash);
    assert_eq!(a.report.impact.evaluated_partitions, 27);
    assert_eq!(a.report.impact.evaluated_connector_orders, 2);
    assert!(a.report.browser_impact.is_none());
    assert_eq!(
        code(accept(a, &contract, &[], None, 0)),
        PlanCode::MissingCaseEvidence
    );
    assert_eq!(
        code(ProjectionContract::from_registry(&n("self-asserted-pass"))),
        PlanCode::CertificationPending
    );
}
#[tokio::test]
async fn broad_inventory_and_marker_never_grant_ownership() {
    let mut fixture = Fixture::new();
    fixture.contexts[0]["description"] = v!("Managed required-field contribution (contract rf-v1)");
    let inventory = fixture.inventory().await;
    let desired = desired();
    let mut owner = grants(&inventory);
    owner.grants.clear();
    assert_eq!(
        code(candidate(
            &inventory,
            &desired,
            &owner,
            &prereqs(&desired),
            &contract(&inventory, &desired)
        )),
        PlanCode::OwnershipRequired
    );
}
#[tokio::test]
async fn complete_shared_replacement_retains_all_unowned_values_and_description() {
    let mut fixture = Fixture::new();
    fixture.contexts[0]["override"]["manual.payload"] =
        v!({"ordered":["B","A","A"],"off":false,"nested":{"unicode":"é"}});
    let inventory = fixture.inventory().await;
    let mut desired = desired();
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .remove(&key(OPTIONS));
    let plan = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract(&inventory, &desired),
    )
    .unwrap();
    assert_eq!(plan.actions.len(), 1);
    let action = &plan.actions[0];
    assert_eq!(action.kind, ActionKind::Replace);
    assert_eq!(action.server_id, Some(n("server-A")));
    let before: Value = serde_json::from_str(action.before_json().unwrap()).unwrap();
    let mut expected = before;
    expected["override"]
        .as_object_mut()
        .unwrap()
        .remove(OPTIONS);
    let actual: Value = serde_json::from_str(action.after_json().unwrap()).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(plan.report.classifications["shared"], 1);
    assert_eq!(plan.report.impact.shadowed_row_changes, 1);
    assert_eq!(plan.report.impact.options_changed, 0);
    let report = serde_json::to_string(plan.report()).unwrap();
    assert!(!report.contains("manual.payload"));
    assert!(!report.contains("manual description"));
    assert!(!report.contains("server-A"));
    let raw=Json::parse(br#"{"override":{"manual":900719925474099312345678901234567890,"managed":true},"future":1.2300,"description":"keep\u0020exact"}"#).unwrap();
    let changed = raw
        .replacing(
            "override",
            Json::members(&BTreeMap::from([(
                "manual".to_owned(),
                raw.get("override").unwrap().get("manual").unwrap().clone(),
            )]))
            .unwrap(),
        )
        .unwrap();
    assert_eq!(changed.get("future").unwrap().raw(), "1.2300");
    assert_eq!(
        changed.get("description").unwrap().raw(),
        raw.get("description").unwrap().raw()
    );
    assert!(changed
        .raw()
        .contains("900719925474099312345678901234567890"));
}
#[tokio::test]
async fn stale_target_duplicate_and_future_manual_rows_refuse() {
    let inventory = Fixture::new().inventory().await;
    let desired = desired();
    let policy = prereqs(&desired);
    let contract = contract(&inventory, &desired);
    let original = grants(&inventory);
    let mut owner = original.clone();
    owner.target.instance_generation += 1;
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::BindingMismatch
    );
    owner = original.clone();
    owner.grants[0].observed_row_hash = d("stale");
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::StaleGrant
    );
    owner = original.clone();
    owner.grants.push(owner.grants[0].clone());
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::DuplicateGrant
    );
    owner = original;
    owner.grants.clear();
    owner.creations.push(CreationGrant {
        intent_id: n("pending-intent"),
        condition: condition("a"),
        attributes: BTreeSet::from([key(REQUIRED), key(OPTIONS)]),
        approver: n("fixture-owner"),
        review: n("fixture-review"),
    });
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::OwnershipRequired
    );
}
#[tokio::test]
async fn new_rows_have_stable_ownership_intents_and_no_guessed_server_id() {
    let mut fixture = Fixture::new();
    fixture.contexts.clear();
    let inventory = fixture.inventory().await;
    let desired = desired();
    let policy = prereqs(&desired);
    let contract = contract(&inventory, &desired);
    let mut owner = grants(&inventory);
    owner.creations.push(CreationGrant {
        intent_id: n("pending-intent"),
        condition: condition("a"),
        attributes: BTreeSet::from([key(REQUIRED), key(OPTIONS)]),
        approver: n("fixture-owner"),
        review: n("fixture-review"),
    });
    let a = candidate(&inventory, &desired, &owner, &policy, &contract).unwrap();
    let b = candidate(&inventory, &desired, &owner, &policy, &contract).unwrap();
    assert_eq!(a.report.candidate_hash, b.report.candidate_hash);
    assert!(a.actions[0].ownership_intent.is_some());
    assert!(a.actions[0].server_id.is_none());
    assert!(!a.actions[0].after_json().unwrap().contains("override_id"));
    let mut next = intent(&desired);
    next.generation += 1;
    let c = prepare(
        &inventory,
        &desired,
        &owner,
        &policy,
        next,
        &contract,
        PlanLimits::default(),
    )
    .unwrap();
    assert_ne!(a.actions[0].ownership_intent, c.actions[0].ownership_intent);
}
#[tokio::test]
async fn empty_active_owner_and_exclusions_do_not_authorize_retirement() {
    let inventory = Fixture::new().inventory().await;
    let mut desired = desired();
    let policy = prereqs(&desired);
    let contract = contract(&inventory, &desired);
    let mut owner = grants(&inventory);
    desired.desired.contexts.clear();
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::EmptyDesired
    );
    owner.retirement = Some(RetirementManifest {
        bundle_hash: desired.bundle_hash.clone(),
        intent_generation: 1,
        server_ids: BTreeSet::from([n("server-A")]),
        approver: n("fixture-owner"),
        review: n("fixture-retirement"),
    });
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::RetirementRequired
    );
    owner.grants[0].allow_row_deletion = true;
    let plan = candidate(&inventory, &desired, &owner, &policy, &contract).unwrap();
    assert_eq!(plan.actions[0].kind, ActionKind::Delete);
    assert!(plan.actions[0].after.is_none());
    assert!(plan.report.impact.required_removed > 0);
    owner.excluded_conditions.push(condition("a"));
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::ExcludedScope
    );
}
#[tokio::test]
async fn explicit_default_equal_options_survive_common_and_mandate_inheritance() {
    let mut fixture = Fixture::new();
    fixture.contexts[0]["override"][OPTIONS] = v!(["B", "A"]);
    let mut specific = condition("a");
    specific.mandate_type = Some(MandateGroup::Mandate);
    fixture.contexts.push(row(
        "server-M",
        v!(specific),
        v!({REQUIRED:true,OPTIONS:["A","B"]}),
    ));
    let inventory = fixture.inventory().await;
    let mut desired = desired();
    desired.desired.contexts.insert(
        specific.clone(),
        BTreeMap::from([
            (key(REQUIRED), ManagedValue::Required(true)),
            (
                key(OPTIONS),
                ManagedValue::Options(vec!["A".to_owned(), "B".to_owned()]),
            ),
        ]),
    );
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .insert(
            key(OPTIONS),
            ManagedValue::Options(vec!["B".to_owned(), "A".to_owned()]),
        );
    let a = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract(&inventory, &desired),
    )
    .unwrap();
    assert!(a.actions.is_empty());
    desired
        .desired
        .contexts
        .get_mut(&specific)
        .unwrap()
        .remove(&key(OPTIONS));
    let b = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract(&inventory, &desired),
    )
    .unwrap();
    assert_eq!(b.report.impact.options_reordered, 1);
    assert_eq!(b.report.impact.options_changed, 1);
    assert_eq!(b.actions[0].kind, ActionKind::Replace);
}
#[tokio::test]
async fn default_true_inheritance_can_shadow_row_removal() {
    let mut fixture = Fixture::new();
    fixture.defaults[0]["value"] = v!(true);
    fixture.contexts[0]["override"]["manual.keep"] = v!(true);
    let inventory = fixture.inventory().await;
    let mut desired = desired();
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .remove(&key(REQUIRED));
    let plan = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract(&inventory, &desired),
    )
    .unwrap();
    assert_eq!(plan.report.impact.required_removed, 0);
    assert_eq!(plan.report.impact.managed_removed, 1);
    assert_eq!(plan.report.impact.shadowed_row_changes, 1);
}
#[tokio::test]
async fn valid_catalog_or_seed_cannot_supply_missing_live_metadata() {
    let mut fixture = Fixture::new();
    fixture
        .defaults
        .retain(|r| r["key"] != "dynamic_fields.choice._confirm_request_write_path");
    let inventory = fixture.inventory().await;
    let desired = desired();
    assert!(desired.catalog.fields[&n("choice")].approved().is_ok());
    assert_eq!(
        code(candidate(
            &inventory,
            &desired,
            &grants(&inventory),
            &prereqs(&desired),
            &contract(&inventory, &desired)
        )),
        PlanCode::MissingPresentation
    );
}
#[tokio::test]
async fn missing_managed_default_needs_exact_declared_creation() {
    let mut fixture = Fixture::new();
    fixture.defaults.retain(|r| r["key"] != REQUIRED);
    let inventory = fixture.inventory().await;
    let desired = desired();
    let mut policy = prereqs(&desired);
    let owner = grants(&inventory);
    let contract = contract(&inventory, &desired);
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::MissingDefault
    );
    policy.create_defaults.push(DefaultCreation {
        key: key(REQUIRED),
        value: ManagedValue::Required(false),
        description: "synthetic declared default".to_owned(),
        review: n("fixture-review"),
    });
    let plan = candidate(&inventory, &desired, &owner, &policy, &contract).unwrap();
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].kind, ActionKind::CreateDefault);
    assert_ne!(
        plan.analysis.reference_hashes,
        plan.analysis.rust_reference_hashes
    );
}
#[tokio::test]
async fn enum_extensions_are_typed_exact_and_preserve_existing_schema() {
    let mut fixture = Fixture::new();
    fixture.dimensions[0]["schema"]["enum"] = v!(["b"]);
    let inventory = fixture.inventory().await;
    let desired = desired();
    let mut policy = prereqs(&desired);
    let owner = grants(&inventory);
    let contract = contract(&inventory, &desired);
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::MissingDimension
    );
    policy
        .dimensions
        .get_mut(&n("connector"))
        .unwrap()
        .allow_enum_extension = true;
    let plan = candidate(&inventory, &desired, &owner, &policy, &contract).unwrap();
    assert_eq!(plan.actions[0].kind, ActionKind::ExtendDimension);
    let result: Value = serde_json::from_str(plan.actions[0].after_json().unwrap()).unwrap();
    assert_eq!(result["schema"]["enum"], v!(["b", "a"]));
    assert_eq!(result["schema"]["description"], v!("keep schema text"));
    let mut typo = desired.clone();
    typo.desired
        .prerequisites
        .dimension_labels
        .get_mut(&n("connector"))
        .unwrap()
        .insert(n("Typo"));
    assert_eq!(
        code(candidate(&inventory, &typo, &owner, &policy, &contract)),
        PlanCode::BindingMismatch
    );
}
#[tokio::test]
async fn unknown_functions_schemas_metadata_and_export_gaps_refuse() {
    for (fixture, expected) in {
        let base = Fixture::new();
        let mut function = base.clone();
        function.defaults[0]["value_compute_function_name"] = v!("external-function");
        let mut schema = base.clone();
        schema.defaults[0]["schema"]["not"] = v!({"const":true});
        let mut metadata = base.clone();
        metadata.defaults.push(default_row(
            "dynamic_fields.choice._unknown_behavior",
            v!(true),
        ));
        let mut export = base.clone();
        export.corrupt_export = true;
        let mut dimension = base;
        dimension.dimensions.pop();
        [
            (function, PlanCode::UnsupportedFunction),
            (schema, PlanCode::UnsupportedSchema),
            (metadata, PlanCode::IncompatiblePresentation),
            (export, PlanCode::UnknownProjection),
            (dimension, PlanCode::MissingDimension),
        ]
    } {
        let inventory = fixture.inventory().await;
        let desired = desired();
        assert_eq!(
            code(candidate(
                &inventory,
                &desired,
                &grants(&inventory),
                &prereqs(&desired),
                &contract(&inventory, &desired)
            )),
            expected
        );
    }
}
#[tokio::test]
async fn options_are_not_normalized_and_unfillable_selects_refuse() {
    for value in [
        v!([]),
        v!(["ALL"]),
        v!(["DK,FI,NO,SE"]),
        v!(["a"]),
        v!(["A", 5]),
    ] {
        let mut fixture = Fixture::new();
        fixture.contexts[0]["override"][OPTIONS] = value;
        let inventory = fixture.inventory().await;
        let desired = desired();
        assert!(candidate(
            &inventory,
            &desired,
            &grants(&inventory),
            &prereqs(&desired),
            &contract(&inventory, &desired)
        )
        .is_err());
    }
    let mut fixture = Fixture::new();
    fixture.contexts[0]["override"][OPTIONS] = v!(["B", "A", "A"]);
    let inventory = fixture.inventory().await;
    let mut desired = desired();
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .insert(
            key(OPTIONS),
            ManagedValue::Options(vec!["B".to_owned(), "A".to_owned(), "A".to_owned()]),
        );
    assert!(candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract(&inventory, &desired)
    )
    .unwrap()
    .actions
    .is_empty());
}
fn experiment(status: &str, context: Value, context_id: Value, override_id: Value) -> Value {
    audit(
        v!({"id":"experiment-one","name":"synthetic","experiment_type":"DEFAULT","override_keys":[REQUIRED],"status":status,"traffic_percentage":10,"context":context,"variants":[{"id":"control","variant_type":"CONTROL","context_id":context_id,"override_id":override_id,"overrides":{REQUIRED:false}}],"experiment_group_id":null}),
        "last_modified",
    )
}
#[tokio::test]
async fn retained_references_active_overlaps_and_bucket_members_refuse() {
    for index in 0..4 {
        let mut fixture = Fixture::new();
        match index {0=>fixture.experiments.push(experiment("DISCARDED",v!({"connector":"b"}),v!("server-A"),Value::Null)),1=>fixture.experiments.push(experiment("CONCLUDED",v!({"connector":"b"}),Value::Null,v!("override-server-A"))),2=>fixture.experiments.push(experiment("INPROGRESS",v!({"connector":"a"}),Value::Null,Value::Null)),_=>fixture.groups.push(audit(v!({"id":"group-one","name":"synthetic","context_hash":"observed-hash","context":{"connector":"a"},"traffic_percentage":10,"member_experiment_ids":[],"buckets":[{"experiment_id":"unknown-experiment","variant_id":"control"}],"group_type":"USER_CREATED"}),"last_modified_at"))};
        let inventory = fixture.inventory().await;
        let mut desired = desired();
        desired
            .desired
            .contexts
            .get_mut(&condition("a"))
            .unwrap()
            .remove(&key(OPTIONS));
        assert_eq!(
            code(candidate(
                &inventory,
                &desired,
                &grants(&inventory),
                &prereqs(&desired),
                &contract(&inventory, &desired)
            )),
            PlanCode::ExperimentConflict
        );
    }
}
#[tokio::test]
async fn connector_order_is_exhaustive_and_conflicts_are_reported() {
    let mut fixture = Fixture::new();
    fixture.contexts.push(row(
        "server-B",
        v!(condition("b")),
        v!({REQUIRED:true,OPTIONS:["B","A"]}),
    ));
    let inventory = fixture.inventory().await;
    let mut desired = desired();
    desired.desired.contexts.insert(
        condition("b"),
        BTreeMap::from([
            (key(REQUIRED), ManagedValue::Required(true)),
            (
                key(OPTIONS),
                ManagedValue::Options(vec!["B".to_owned(), "A".to_owned()]),
            ),
        ]),
    );
    let plan = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract(&inventory, &desired),
    )
    .unwrap();
    assert_eq!(plan.report.impact.evaluated_connector_orders, 2);
    assert!(plan.report.impact.connector_order_conflicts > 0);
    assert!(plan.actions.is_empty());
    let request: Value = serde_json::from_str(plan.case_request_json()).unwrap();
    let orders: BTreeSet<_> = request["browser_matrix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["connectors"].to_string())
        .collect();
    assert_eq!(
        orders,
        BTreeSet::from([v!(["a", "b"]).to_string(), v!(["b", "a"]).to_string()])
    );
}
#[tokio::test]
async fn bounds_are_refusals_not_sampling_or_batch_fallbacks() {
    let inventory = Fixture::new().inventory().await;
    let desired = desired();
    let owner = grants(&inventory);
    let policy = prereqs(&desired);
    let contract = contract(&inventory, &desired);
    for limits in [
        PlanLimits {
            max_partitions: 26,
            ..Default::default()
        },
        PlanLimits {
            max_connector_orders: 1,
            ..Default::default()
        },
        PlanLimits {
            max_evaluations: 1,
            ..Default::default()
        },
        PlanLimits {
            max_contexts: 0,
            ..Default::default()
        },
    ] {
        assert_eq!(
            code(prepare(
                &inventory,
                &desired,
                &owner,
                &policy,
                intent(&desired),
                &contract,
                limits
            )),
            PlanCode::LimitExceeded
        );
    }
}
#[tokio::test]
async fn observed_extra_dimensions_are_partitioned_without_ownership() {
    for (dimension_name, literal) in [
        ("country", v!("US")),
        ("profile_id", v!("tenant-one")),
        ("always_collect_billing_details", v!(true)),
    ] {
        let mut fixture = Fixture::new();
        let mut dim = dimension(dimension_name, 4, v!(["US", "tenant-one"]));
        if literal.is_boolean() {
            dim["schema"] = v!({"type":"boolean"});
        }
        fixture.dimensions.push(dim);
        fixture.contexts.push(row(
            "manual-rule",
            v!({dimension_name:literal}),
            v!({REQUIRED:false}),
        ));
        let inventory = fixture.inventory().await;
        let desired = desired();
        let plan = candidate(
            &inventory,
            &desired,
            &grants(&inventory),
            &prereqs(&desired),
            &contract(&inventory, &desired),
        )
        .unwrap();
        assert_eq!(plan.report.impact.evaluated_partitions, 81);
        assert_eq!(plan.report.classifications["unrelated"], 1);
        assert!(plan.actions.is_empty());
    }
}
#[tokio::test]
async fn ordered_identity_collisions_are_rejected_by_registered_profile() {
    fn collide(_: &str) -> PlanResult<String> {
        Ok("synthetic-array-insensitive-identity".to_owned())
    }
    let mut fixture = Fixture::new();
    fixture.contexts.push(row(
        "server-B",
        v!(condition("b")),
        v!({REQUIRED:true,OPTIONS:["B","A"]}),
    ));
    let inventory = fixture.inventory().await;
    let desired = desired();
    let mut owner = grants(&inventory);
    owner.grants.retain(|g| g.server_id == n("server-A"));
    let mut contract = contract(&inventory, &desired);
    contract.override_identity = collide;
    assert_eq!(
        code(candidate(
            &inventory,
            &desired,
            &owner,
            &prereqs(&desired),
            &contract
        )),
        PlanCode::OrderedValueCollision
    );
}
#[test]
fn manifests_refuse_duplicate_keys_unknown_fields_and_duplicate_sets() {
    assert!(parse_manifest::<PlanIntent>(br#"{"generation":1,"generation":2}"#).is_err());
    let mut policy = prereqs(&desired());
    policy
        .fields
        .get_mut(&n("choice"))
        .unwrap()
        .metadata
        .insert("future_behavior".to_owned(), v!(true));
    let source = v!({"server_id":"a","condition":condition("a"),"observed_row_hash":d("row"),"attributes":[REQUIRED,REQUIRED],"allow_row_deletion":false,"approver":"owner","review":"review"});
    assert!(parse_manifest::<AdoptionGrant>(&serde_json::to_vec(&source).unwrap()).is_err());
    let mut intent = v!(intent(&desired()));
    intent["force"] = v!(true);
    assert!(parse_manifest::<PlanIntent>(&serde_json::to_vec(&intent).unwrap()).is_err());
}

// Synthetic receipts exercise refusal/binding logic only. There is no production registration.
fn synthetic_reports(candidate: &PlanCandidate, contract: &mut ProjectionContract) -> Vec<Vec<u8>> {
    let mut reports = Vec::new();
    for phase in 0..2 {
        for stage in [
            SdkStage::JsResolver,
            SdkStage::RustResolver,
            SdkStage::BrowserComponents,
            SdkStage::Checkout,
        ] {
            let resolver = matches!(stage, SdkStage::JsResolver | SdkStage::RustResolver);
            let hashes = if stage == SdkStage::JsResolver {
                &candidate.analysis.reference_hashes
            } else {
                &candidate.analysis.rust_reference_hashes
            };
            let bound = BoundSdkReport {
                schema_version: 1,
                request_hash: candidate.report.case_request_hash.clone(),
                phase: if phase == 0 { "before" } else { "after" }.to_owned(),
                report: Step1Report {
                    schema_version: 1,
                    contract_id: "required-fields-sdk-v1".to_owned(),
                    pack_sha256: serde_json::to_value(&contract.case_pack)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .trim_start_matches("sha256:")
                        .to_owned(),
                    provenance: contract.provenance.clone(),
                    stage: stage.clone(),
                    execution: Execution {
                        engine: stage.clone(),
                        artifact_sha256: serde_json::to_value(&contract.artifacts[&stage])
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .trim_start_matches("sha256:")
                            .to_owned(),
                        runtime: "synthetic-only".to_owned(),
                    },
                    results: vec![CaseResult {
                        id: n("independent-fixture-case"),
                        status: "passed".to_owned(),
                        detail: "synthetic receipt validation fixture; not executed SDK evidence"
                            .to_owned(),
                    }],
                    status: "passed".to_owned(),
                },
                reference_hashes: if resolver {
                    hashes
                        .iter()
                        .map(|(a, b)| if phase == 0 { a.clone() } else { b.clone() })
                        .collect()
                } else {
                    vec![]
                },
                browser_observations: if resolver {
                    vec![]
                } else {
                    candidate
                        .analysis
                        .browser_case_hashes
                        .iter()
                        .zip(&candidate.analysis.browser_expected)
                        .map(|(id, (a, b))| BrowserObservation {
                            case_hash: id.clone(),
                            selected_descriptors_hash: if phase == 0 {
                                a.clone()
                            } else {
                                b.clone()
                            },
                            included_fields_hash: d("synthetic-inclusion"),
                            visible_fields_hash: d("synthetic-visibility"),
                            validation_hash: d("synthetic-validation"),
                            payload_hash: d(if phase == 0 || candidate.actions.is_empty() {
                                "synthetic-before-payload"
                            } else {
                                "synthetic-after-payload"
                            }),
                        })
                        .collect()
                },
            };
            let bytes = serde_json::to_vec(&bound).unwrap();
            contract
                .trusted_report_digests
                .push(hash(&Json::parse(&bytes).unwrap()).unwrap());
            reports.push(bytes);
        }
    }
    reports
}
fn approval(candidate: &PlanCandidate, evidence: &CaseEvidenceReport) -> SemanticApproval {
    SemanticApproval {
        schema_version: 1,
        target: candidate.target.clone(),
        candidate_hash: candidate.report.candidate_hash.clone(),
        actions_hash: candidate.actions_hash.clone(),
        case_request_hash: candidate.report.case_request_hash.clone(),
        impact_hash: candidate.report.impact_hash.clone(),
        sdk_evidence_hash: evidence.sdk_evidence_hash.clone(),
        approver: n("fixture-owner"),
        review: n("fixture-review"),
        expires_at_unix: 100,
    }
}
#[tokio::test]
async fn trusted_all_stage_receipts_are_required_and_browser_effects_stay_separate() {
    let inventory = Fixture::new().inventory().await;
    let desired = desired();
    let mut contract = contract(&inventory, &desired);
    let plan = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract,
    )
    .unwrap();
    let reports = synthetic_reports(&plan, &mut contract);
    let slices: Vec<_> = reports.iter().map(Vec::as_slice).collect();
    let evidence = examine_cases(&plan, &contract, &slices).unwrap();
    assert_eq!(
        evidence.browser_impact[&SdkStage::Checkout].payload_changed,
        0
    );
    assert_eq!(plan.report.impact.required_added, 0);
    let accepted = accept(plan, &contract, &slices, None, 0).unwrap();
    assert!(accepted.report().plan_hash.is_some());
    assert_eq!(accepted.report().hosted_apply, "disabled");
    assert!(accepted.report().provisional);
    assert!(matches!(
        accepted.report().scope,
        EvidenceScope::SyntheticFixture
    ));
    let plan = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract,
    )
    .unwrap();
    contract.trusted_report_digests.clear();
    assert_eq!(
        code(accept(plan, &contract, &slices, None, 0)),
        PlanCode::MissingCaseEvidence
    );
}
#[tokio::test]
async fn every_semantic_write_requires_exact_unexpired_approval() {
    let inventory = Fixture::new().inventory().await;
    let mut desired = desired();
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .remove(&key(OPTIONS));
    let owner = grants(&inventory);
    let policy = prereqs(&desired);
    let mut contract = contract(&inventory, &desired);
    let make = |contract: &ProjectionContract| {
        candidate(&inventory, &desired, &owner, &policy, contract).unwrap()
    };
    let plan = make(&contract);
    let reports = synthetic_reports(&plan, &mut contract);
    let slices: Vec<_> = reports.iter().map(Vec::as_slice).collect();
    let evidence = examine_cases(&plan, &contract, &slices).unwrap();
    let exact = approval(&plan, &evidence);
    assert_eq!(
        code(accept(plan, &contract, &slices, None, 0)),
        PlanCode::ApprovalRequired
    );
    assert_eq!(
        code(accept(
            make(&contract),
            &contract,
            &slices,
            Some(&exact),
            100
        )),
        PlanCode::ApprovalExpired
    );
    for which in 0..7 {
        let mut bad = exact.clone();
        match which {
            0 => bad.target.instance_generation += 1,
            1 => bad.candidate_hash = d("wrong"),
            2 => bad.actions_hash = d("wrong"),
            3 => bad.case_request_hash = d("wrong"),
            4 => bad.impact_hash = d("wrong"),
            5 => bad.sdk_evidence_hash = d("wrong"),
            _ => bad.schema_version += 1,
        };
        assert_eq!(
            code(accept(make(&contract), &contract, &slices, Some(&bad), 0)),
            PlanCode::ApprovalRequired
        );
    }
    assert_eq!(
        accept(make(&contract), &contract, &slices, Some(&exact), 0)
            .unwrap()
            .report()
            .review_state,
        "provisional_reviewed"
    );
}
#[tokio::test]
async fn case_receipts_bind_phase_artifact_request_provenance_and_all_cases() {
    let inventory = Fixture::new().inventory().await;
    let desired = desired();
    let mut contract = contract(&inventory, &desired);
    let plan = candidate(
        &inventory,
        &desired,
        &grants(&inventory),
        &prereqs(&desired),
        &contract,
    )
    .unwrap();
    let original = synthetic_reports(&plan, &mut contract);
    for which in 0..7 {
        let mut reports = original.clone();
        let mut changed: Value = serde_json::from_slice(&reports[0]).unwrap();
        match which {
            0 => changed["request_hash"] = v!(d("wrong")),
            1 => changed["report"]["pack_sha256"] = v!("0".repeat(64)),
            2 => changed["report"]["execution"]["artifact_sha256"] = v!("0".repeat(64)),
            3 => changed["report"]["provenance"] = v!({}),
            4 => changed["report"]["results"] = v!([]),
            5 => changed["reference_hashes"] = v!([]),
            _ => changed["report"]["status"] = v!("not_run"),
        };
        reports[0] = serde_json::to_vec(&changed).unwrap();
        contract
            .trusted_report_digests
            .push(hash(&Json::parse(&reports[0]).unwrap()).unwrap());
        let slices: Vec<_> = reports.iter().map(Vec::as_slice).collect();
        assert_eq!(
            code(examine_cases(&plan, &contract, &slices)),
            PlanCode::InvalidCaseEvidence
        );
    }
    let mut duplicate = original;
    duplicate[1] = duplicate[0].clone();
    assert_eq!(
        code(examine_cases(
            &plan,
            &contract,
            &duplicate.iter().map(Vec::as_slice).collect::<Vec<_>>()
        )),
        PlanCode::InvalidCaseEvidence
    );
}
#[tokio::test]
async fn unknown_predicates_and_active_alias_ties_refuse() {
    let mut fixture = Fixture::new();
    fixture.contexts.push(row(
        "manual-unsupported",
        v!({"connector":{"in":["a"]}}),
        v!({REQUIRED:true}),
    ));
    let inventory = fixture.inventory().await;
    let desired = desired();
    assert_eq!(
        code(candidate(
            &inventory,
            &desired,
            &grants(&inventory),
            &prereqs(&desired),
            &contract(&inventory, &desired)
        )),
        PlanCode::UnsupportedPredicate
    );
    let mut fixture = Fixture::new();
    for row in fixture.defaults.clone() {
        let mut alias = row;
        let old = alias["key"].as_str().unwrap().to_owned();
        alias["key"] = v!(old.replace("choice._", "alias._"));
        fixture.defaults.push(alias);
    }
    fixture
        .defaults
        .iter_mut()
        .find(|r| r["key"] == "dynamic_fields.alias._is_required")
        .unwrap()["value"] = v!(true);
    let inventory = fixture.inventory().await;
    let mut desired = desired;
    desired
        .catalog
        .fields
        .insert(n("alias"), desired.catalog.fields[&n("choice")].clone());
    let mut policy = prereqs(&desired);
    policy
        .fields
        .insert(n("alias"), policy.fields[&n("choice")].clone());
    assert_eq!(
        code(candidate(
            &inventory,
            &desired,
            &grants(&inventory),
            &policy,
            &contract(&inventory, &desired)
        )),
        PlanCode::ConnectorOrderConflict
    );
}

/// Writes only synthetic cases into a previously registered owned filesystem fixture.
/// The independent JS/core probes consume these; no synthetic receipt is issued by the exporter.
#[tokio::test]
#[ignore = "requires RF_STEP6_FIXTURE_ROOT with an ownership registration"]
async fn export_synthetic_planner_cases() {
    use std::io::Write as _;
    let root = std::path::PathBuf::from(
        std::env::var("RF_STEP6_FIXTURE_ROOT").expect("register fixture first"),
    );
    let owner: Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(owner["root"], v!(root));
    assert_eq!(owner["cleanup_scope"], v!(root));
    assert_eq!(owner["purpose"], "Step 6 offline synthetic planner checks");
    assert!(root.is_absolute());
    let mut profiles = Vec::new();
    for name in [
        "unchanged",
        "inherited-options",
        "missing-default",
        "connector-orders",
        "mandate-inheritance",
    ] {
        let mut fixture = Fixture::new();
        let mut desired = desired();
        let mut policy = prereqs(&desired);
        if name == "inherited-options" {
            fixture.contexts[0]["override"][OPTIONS] = v!(["B", "A"]);
        }
        if name == "missing-default" {
            fixture.defaults.retain(|r| r["key"] != REQUIRED);
            policy.create_defaults.push(DefaultCreation {
                key: key(REQUIRED),
                value: ManagedValue::Required(false),
                description: "synthetic declared default".to_owned(),
                review: n("fixture-review"),
            });
        }
        if name == "connector-orders" {
            fixture.contexts.push(row(
                "server-B",
                v!(condition("b")),
                v!({REQUIRED:true,OPTIONS:["B","A"]}),
            ));
            desired.desired.contexts.insert(
                condition("b"),
                BTreeMap::from([
                    (key(REQUIRED), ManagedValue::Required(true)),
                    (
                        key(OPTIONS),
                        ManagedValue::Options(vec!["B".to_owned(), "A".to_owned()]),
                    ),
                ]),
            );
        }
        if name == "mandate-inheritance" {
            fixture.contexts[0]["override"][OPTIONS] = v!(["A"]);
            desired
                .desired
                .contexts
                .get_mut(&condition("a"))
                .unwrap()
                .insert(key(OPTIONS), ManagedValue::Options(vec!["A".to_owned()]));
            let mut specific = condition("a");
            specific.mandate_type = Some(MandateGroup::Mandate);
            fixture.contexts.push(row(
                "server-M",
                v!(specific),
                v!({REQUIRED:true,OPTIONS:["A","B"]}),
            ));
            desired.desired.contexts.insert(
                specific,
                BTreeMap::from([(key(REQUIRED), ManagedValue::Required(true))]),
            );
        }
        let inventory = fixture.inventory().await;
        let plan = candidate(
            &inventory,
            &desired,
            &grants(&inventory),
            &policy,
            &contract(&inventory, &desired),
        )
        .unwrap();
        let request: Value = serde_json::from_str(plan.case_request_json()).unwrap();
        profiles.push(v!({"id":name,"request":request,"js_reference_hashes":plan.analysis.reference_hashes,"rust_reference_hashes":plan.analysis.rust_reference_hashes,"report":plan.report()}));
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("planner-cases.json"))
        .unwrap();
    file.write_all(
        &serde_json::to_vec_pretty(
            &v!({"schema_version":1,"scope":"synthetic_fixture","profiles":profiles}),
        )
        .unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn changed_feature_availability_and_cross_phase_order_identity_refuse() {
    let inventory = Fixture::new().inventory().await;
    let mut desired = desired();
    let owner = grants(&inventory);
    let policy = prereqs(&desired);
    let mut contract = contract(&inventory, &desired);
    desired.source_availability_hash = d("feature-removed");
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::RetirementRequired
    );
    desired.source_availability_hash = owner.source_availability_hash.clone();
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .insert(
            key(OPTIONS),
            ManagedValue::Options(vec!["B".to_owned(), "A".to_owned()]),
        );
    contract.override_identity = |_| Ok("synthetic-same-id-for-reordered-list".to_owned());
    assert_eq!(
        code(candidate(&inventory, &desired, &owner, &policy, &contract)),
        PlanCode::OrderedValueCollision
    );
}

#[tokio::test]
async fn shared_preservation_and_second_plan_hold_for_nested_value_variations() {
    let contract_for_next = contract;
    let values = [
        v!(null),
        v!(false),
        v!(0),
        v!("é\nkeep"),
        v!(["B", "A", "A"]),
        v!({"nested":{"off":false,"ordered":[3,2,1]},"nothing":null}),
    ];
    for (index, value) in values.into_iter().enumerate() {
        let mut fixture = Fixture::new();
        fixture.contexts[0]["description"] = v!(format!("manual {index} — preserve"));
        fixture.contexts[0]["override"]["manual.owned_elsewhere"] = value;
        let inventory = fixture.inventory().await;
        let mut desired = desired();
        desired
            .desired
            .contexts
            .get_mut(&condition("a"))
            .unwrap()
            .remove(&key(OPTIONS));
        let policy = prereqs(&desired);
        let owner = grants(&inventory);
        let contract = contract(&inventory, &desired);
        let first = candidate(&inventory, &desired, &owner, &policy, &contract).unwrap();
        let action = &first.actions[0];
        let mut expected = fixture.contexts[0].clone();
        expected["override"]
            .as_object_mut()
            .unwrap()
            .remove(OPTIONS);
        let projected: Value = serde_json::from_str(action.after_json().unwrap()).unwrap();
        assert_eq!(projected, expected);
        // Synthetic observation of the projected row, with a newly reviewed exact row hash.
        // This is the pure second-plan invariant; no server apply/restore is simulated as success.
        fixture.contexts[0] = projected;
        let next = fixture.inventory().await;
        let second = candidate(
            &next,
            &desired,
            &grants(&next),
            &policy,
            &contract_for_next(&next, &desired),
        )
        .unwrap();
        assert!(second.actions.is_empty());
    }
}
#[test]
fn empty_values_do_not_hide_unsupported_schema_constraints() {
    assert_eq!(
        code(schema_accepts(
            &json(&v!({"type":"array","items":{"type":"string","pattern":"unknown-constraint"}}))
                .unwrap(),
            &json(&v!([])).unwrap()
        )),
        PlanCode::UnsupportedSchema
    );
}
