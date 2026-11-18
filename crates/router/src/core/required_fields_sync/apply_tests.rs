//! Independently applied synthetic HTTP payloads; not server, browser or platform certification.
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Mutex,
};

use super::{
    super::super::{
        apply::{self, ApplyRuntime, Cancellation, PreparedApply},
        apply_contract as ac, authority as a, coordination as c, journal as j, mutation as m,
    },
    *,
};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Deleted,
    Unchanged,
    Changed,
    PrerequisitesOnly,
    ThreeRequests,
    TwoContexts,
}
struct Case {
    prepared: PreparedApply,
    target: ac::CertifiedMutationTarget,
    intent: a::ReleaseIntent,
    plan_hash: Digest,
    fixture: Fixture,
    actions: Vec<PlannedAction>,
    frozen: Vec<j::PreparedRequest>,
}
async fn case(shape: Shape) -> Case {
    let mut fixture = Fixture::new();
    // An unrelated override and nested row metadata must survive replacement byte-for-byte.
    fixture.contexts[0]["override"]["routing.note"] = v!("retain");
    fixture
        .defaults
        .push(default_row("routing.note", v!("fallback")));
    let mut desired = desired();
    if matches!(shape, Shape::Deleted) {
        fixture.contexts[0]["override"]
            .as_object_mut()
            .unwrap()
            .remove("routing.note");
        fixture.contexts[0]
            .as_object_mut()
            .unwrap()
            .remove("future_metadata");
        desired.desired.contexts.clear();
    }
    if matches!(shape, Shape::Changed | Shape::TwoContexts) {
        desired
            .desired
            .contexts
            .get_mut(&condition("a"))
            .unwrap()
            .remove(&key(OPTIONS));
    }
    if matches!(shape, Shape::TwoContexts) {
        desired.desired.contexts.insert(
            condition("b"),
            BTreeMap::from([
                (key(REQUIRED), ManagedValue::Required(true)),
                (
                    key(OPTIONS),
                    ManagedValue::Options(vec!["A".into(), "B".into()]),
                ),
            ]),
        );
    }
    if matches!(shape, Shape::PrerequisitesOnly | Shape::ThreeRequests) {
        fixture.defaults.retain(|r| r["key"] != REQUIRED);
    }
    if matches!(shape, Shape::ThreeRequests) {
        fixture.contexts.clear();
        fixture.dimensions[0]["schema"]["enum"] = v!(["b"]);
    }
    let inventory = fixture.inventory().await;
    let release = ReleaseIdentity {
        application_revision: "4".repeat(40).try_into().unwrap(),
        source_tree_hash: d("apply-tree"),
        artifact: ArtifactIdentity::Image {
            index_digest: d("image"),
            platform_digests: BTreeMap::from([(n("linux-arm64"), d("image"))]),
        },
        renderer_implementation: n("synthetic-renderer"),
    };
    desired.release_hash = digest(&release).unwrap();
    desired.desired.input_manifest.build = BuildIdentity::from_feature_keys(
        release,
        "REQUIRED_FIELDS_SYNC,V1",
        "REQUIRED_FIELDS_SYNC,V1",
    )
    .unwrap();
    let bundle = v!({"synthetic_only":true,"desired":desired.desired,"catalog":desired.catalog});
    desired.bundle_hash = canonical_hash(HashDomain::Bundle, &bundle).unwrap();
    let mut ownership = grants(&inventory);
    if matches!(shape, Shape::Deleted) {
        ownership.grants[0].allow_row_deletion = true;
        ownership.retirement = Some(RetirementManifest {
            bundle_hash: desired.bundle_hash.clone(),
            intent_generation: 1,
            server_ids: BTreeSet::from([n("server-A")]),
            approver: n("owner"),
            review: n("delete-review"),
        });
    }
    if matches!(shape, Shape::ThreeRequests | Shape::TwoContexts) {
        ownership.creations.push(CreationGrant {
            intent_id: n("synthetic-create"),
            condition: condition(if matches!(shape, Shape::TwoContexts) {
                "b"
            } else {
                "a"
            }),
            attributes: BTreeSet::from([key(REQUIRED), key(OPTIONS)]),
            approver: n("synthetic-owner"),
            review: n("synthetic-review"),
        });
    }
    let mut prerequisites = prereqs(&desired);
    if matches!(shape, Shape::PrerequisitesOnly | Shape::ThreeRequests) {
        prerequisites.create_defaults.push(DefaultCreation {
            key: key(REQUIRED),
            value: ManagedValue::Required(false),
            description: "synthetic default".into(),
            review: n("review"),
        });
    }
    if matches!(shape, Shape::ThreeRequests) {
        prerequisites
            .dimensions
            .get_mut(&n("connector"))
            .unwrap()
            .allow_enum_extension = true;
    }
    let mut projection = contract(&inventory, &desired);
    let candidate = candidate(
        &inventory,
        &desired,
        &ownership,
        &prerequisites,
        &projection,
    )
    .unwrap();
    let reports = synthetic_reports(&candidate, &mut projection);
    let slices: Vec<_> = reports.iter().map(Vec::as_slice).collect();
    let evidence = examine_cases(&candidate, &projection, &slices).unwrap();
    let approval = approval(&candidate, &evidence);
    let plan = accept(candidate, &projection, &slices, Some(&approval), 0).unwrap();
    let plan_hash = plan.report().plan_hash.clone().unwrap();
    let mut registration = c::tests::registration();
    registration.target = inventory.proof().target.clone();
    registration.authority = projection.authority.clone();
    let reader = registration.readers.remove(&n("main")).unwrap();
    registration
        .readers
        .insert(registration.authority.clone(), reader);
    registration.journal_contract = j::tests::certificate().contract_id().clone();
    let mut intent = c::tests::intent();
    intent.target = registration.target.clone();
    intent.authority = registration.authority.clone();
    intent.generation = 1;
    intent.executor_image = d("image");
    intent.release = desired.release_hash.clone();
    intent.bundle = desired.bundle_hash.clone();
    let manifest = &desired.desired.input_manifest;
    intent.input_manifest = canonical_hash(HashDomain::InputManifest, manifest).unwrap();
    intent.build = hash(&json(&manifest.build).unwrap()).unwrap();
    intent.policy = manifest.mapping_hash.clone();
    intent.catalog = manifest.catalog_hash.clone();
    intent.settings = manifest.effective_settings_hash.clone();
    let mut reader = intent.readers.remove(&n("main")).unwrap();
    reader.image = intent.executor_image.clone();
    reader.build = intent.build.clone();
    reader.inputs = intent.input_manifest.clone();
    intent.readers.insert(intent.authority.clone(), reader);
    let audit = [
        "created_at",
        "last_modified_at",
        "created_by",
        "last_modified_by",
        "change_reason",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    let target = ac::CertifiedMutationTarget {
        id: registration.server_contract.clone(),
        registration,
        read_contract: inventory.proof().evidence.contract_id.clone(),
        server_image: inventory.proof().evidence.server_image.clone(),
        server_source: inventory.proof().evidence.server_source.clone(),
        scope: EvidenceScope::SyntheticFixture,
        max_prerequisites: 32,
        max_context_actions: 100,
        max_body: 512 * 1024,
        request_ms: 30,
        rollback_statuses: BTreeSet::from([400, 409, 422]),
        context_audit: audit.clone(),
        default_audit: audit.clone(),
        dimension_audit: audit,
    };
    let guard = j::SecretGuard::default();
    let prepared = PreparedApply::prepare(
        j::Capture {
            plan: &plan,
            inventory: &inventory,
            ownership: &ownership,
            prerequisites: &prerequisites,
            projection: &projection,
            bundle: j::RestrictedJson::parse(&serde_json::to_vec(&bundle).unwrap(), &guard)
                .unwrap(),
            identity: j::RecoveryIdentity {
                target: target.registration.target.clone(),
                endpoint_identity: target.registration.endpoint_identity.clone(),
                coordinator: target.registration.coordinator.object.clone(),
                executor_image: intent.executor_image.clone(),
                retained_recovery_image: d("recovery-image"),
            },
            retention: j::Retention {
                created_at_unix: 1,
                retain_until_unix: 1 + j::MIN_RETENTION_SECONDS,
                rollback_window_seconds: 86400,
                protected_hold: true,
            },
            requests: vec![],
            unreadable_attributes: vec![],
            server_managed_attributes: target.context_audit.iter().map(|s| n(s)).collect(),
        },
        &target,
        &guard,
    )
    .unwrap();
    let frozen = prepared.requests().to_vec();
    Case {
        prepared,
        target,
        intent,
        plan_hash,
        fixture,
        actions: plan.actions().to_vec(),
        frozen,
    }
}

#[derive(Clone, Copy, Debug, Default)]
enum StorageFault {
    #[default]
    None,
    Prepare,
    Intent,
    IntentLostAck,
    Transport,
    Resolution,
    FinalReport,
}
struct Store {
    inner: j::tests::MemoryStore,
    events: Mutex<Vec<Value>>,
    fault: StorageFault,
    reports: AtomicUsize,
}
impl Store {
    fn new(fault: StorageFault) -> Self {
        Self {
            inner: j::tests::MemoryStore::default(),
            events: Mutex::new(vec![]),
            fault,
            reports: AtomicUsize::new(0),
        }
    }
}
impl j::JournalReader for Store {
    async fn read(&self, key: &str, max: usize) -> j::JournalResult<Option<j::StoredObject>> {
        self.inner.read(key, max).await
    }
}
impl j::ImmutableJournalStore for Store {
    async fn create(
        &self,
        key: &str,
        bytes: &[u8],
        retention: &j::Retention,
    ) -> j::JournalResult<Name> {
        let event: Value = serde_json::from_slice(bytes).unwrap();
        let kind = event["body"]["kind"].as_str();
        let report_number = if kind == Some("run_report") {
            self.reports.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            0
        };
        if matches!(
            (self.fault, kind),
            (StorageFault::Prepare, None)
                | (StorageFault::Intent, Some("intent"))
                | (StorageFault::Transport, Some("observation"))
                | (StorageFault::Resolution, Some("resolution"))
        ) || (matches!(self.fault, StorageFault::FinalReport) && report_number == 2)
        {
            return Err(j::JournalError {
                code: j::JournalCode::AccessDenied,
            }
            .into());
        }
        let result = self.inner.create(key, bytes, retention).await?;
        let lost_ack = matches!(self.fault, StorageFault::IntentLostAck) && kind == Some("intent");
        self.events.lock().unwrap().push(event);
        if lost_ack {
            return Err(j::JournalError {
                code: j::JournalCode::UnknownStorageOutcome,
            }
            .into());
        }
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug, Default)]
enum Fault {
    #[default]
    None,
    Reject(usize),
    LostBefore,
    LostAt(usize),
    LostAfter,
    PostCommit512,
    Error500Before,
    SuccessBefore,
    WrongVersion,
    ReadAfter,
    Diverge,
    LeaseAfter(usize),
    GateBeforeFirst,
    EditBeforeBulk,
    RetryInitialRead,
    TimeoutBefore,
    TimeoutAfter,
    CancelAfter,
    CancelBefore,
    Pause,
    MalformedResponse,
    MalformedRejection,
}
struct Server<'a> {
    state: Mutex<Fixture>,
    store: &'a Store,
    coordinator: &'a c::tests::Memory,
    controls: &'a c::tests::Controls,
    cancel: &'a Cancellation,
    fault: Fault,
    calls: Mutex<Vec<(String, Vec<u8>)>>,
    reads: AtomicUsize,
}
struct CurrentVersion(Fixture);
impl ReadSource for CurrentVersion {
    async fn read(
        &self,
        request: ReadRequest,
        timeout: Duration,
        cap: usize,
    ) -> InventoryResult<RawResponse> {
        let version = matches!(&request, ReadRequest::VersionHead);
        let mut response = self.0.read(request, timeout, cap).await?;
        if version {
            let mut value: Value = serde_json::from_slice(&response.body).unwrap();
            value["data"][0]["id"] = self.0.workspace["config_version"].clone();
            response.body = serde_json::to_vec(&value).unwrap();
        }
        Ok(response)
    }
}
impl ac::ApplyReader for Server<'_> {
    async fn complete(&self) -> ac::ApplyResult<CompleteInventory> {
        let read = self.reads.fetch_add(1, Ordering::SeqCst) + 1;
        let dispatched = self.calls.lock().unwrap().len();
        if matches!(self.fault, Fault::ReadAfter) && dispatched > 0
            || matches!(self.fault, Fault::RetryInitialRead) && read == 1
        {
            return Err(ac::ApplyError {
                code: ac::ApplyCode::ReadIncomplete,
            }
            .into());
        }
        if matches!(self.fault, Fault::GateBeforeFirst) && read == 2 {
            self.controls.unavailable.store(true, Ordering::SeqCst);
        }
        if matches!(self.fault, Fault::EditBeforeBulk) && read == 6 {
            self.state.lock().unwrap().workspace["metrics"] = v!({"external":"edit"});
        }
        let fixture = self.state.lock().unwrap().clone();
        inventory::discover(
            &CurrentVersion(fixture),
            &inventory::tests::target(),
            InventoryLimits::default(),
        )
        .await
        .map_err(|_| {
            ac::ApplyError {
                code: ac::ApplyCode::ReadIncomplete,
            }
            .into()
        })
    }
}
impl m::MutationWriter for Server<'_> {
    async fn send(&self, dispatch: m::Dispatch<'_>) -> m::WireReply {
        let request = dispatch.request();
        let request_hash = request.hash().unwrap();
        let ordinal = {
            let mut calls = self.calls.lock().unwrap();
            calls.push((request.path().to_owned(), request.body().to_vec()));
            calls.len()
        };
        let event = self.store.events.lock().unwrap().last().unwrap().clone();
        assert_eq!(
            event["body"]["kind"], "intent",
            "durable intent before every dispatch"
        );
        assert_eq!(
            event["body"]["request"]["body"]
                .as_str()
                .unwrap()
                .as_bytes(),
            request.body(),
            "frozen bytes"
        );
        let active = self
            .coordinator
            .row
            .lock()
            .unwrap()
            .1
            .active
            .clone()
            .unwrap();
        assert_eq!(active.phase, c::RunPhase::RequestInFlight);
        assert_eq!(active.request_in_flight, Some(request_hash));
        if matches!(self.fault, Fault::Pause) {
            std::future::pending::<()>().await;
        }
        let status = match self.fault {
            Fault::Reject(n) if n == ordinal => 400,
            Fault::PostCommit512 => 512,
            Fault::MalformedRejection => 400,
            Fault::Error500Before => 500,
            _ => 200,
        };
        let commit = status != 400
            && !matches!(self.fault,Fault::LostAt(n) if n==ordinal)
            && !matches!(
                self.fault,
                Fault::LostBefore
                    | Fault::Error500Before
                    | Fault::SuccessBefore
                    | Fault::TimeoutBefore
            );
        if commit {
            // Interpret independently specified SDK wire shapes, never copy the planner's after rows.
            let body: Value = serde_json::from_slice(request.body()).unwrap();
            let mut state = self.state.lock().unwrap();
            let mut next = state.clone();
            match request.path() {
                "/default-config" => {
                    let mut row = default_row(body["key"].as_str().unwrap(), body["value"].clone());
                    row["schema"] = body["schema"].clone();
                    row["description"] = body["description"].clone();
                    row["change_reason"] = body["change_reason"].clone();
                    next.defaults.push(row);
                }
                "/dimension/connector" => {
                    assert_eq!(request.method(), j::Method::Patch);
                    next.dimensions[0]["schema"] = body["schema"].clone();
                    next.dimensions[0]["description"] = body["description"].clone();
                    next.dimensions[0]["change_reason"] = body["change_reason"].clone();
                }
                "/context/bulk-operations" => {
                    for (i, op) in body["operations"].as_array().unwrap().iter().enumerate() {
                        if let Some(value) = op.get("REPLACE") {
                            let row = next
                                .contexts
                                .iter_mut()
                                .find(|r| r["id"] == value["context"]["id"])
                                .unwrap();
                            row["override"] = value["override"].clone();
                            row["override_id"] = v!(format!("new-{ordinal}-{i}"));
                            row["description"] = value["description"].clone();
                            row["change_reason"] = value["change_reason"].clone();
                        } else if let Some(value) = op.get("PUT") {
                            let mut row = row(
                                &format!("created-{ordinal}-{i}"),
                                value["context"].clone(),
                                value["override"].clone(),
                            );
                            row.as_object_mut().unwrap().remove("future_metadata");
                            row["description"] = value["description"].clone();
                            row["change_reason"] = value["change_reason"].clone();
                            next.contexts.push(row);
                        } else if let Some(value) = op.get("DELETE") {
                            next.contexts.retain(|r| &r["id"] != value);
                        } else {
                            panic!("unsupported wire action");
                        }
                    }
                }
                path => panic!("unexpected mutation path {path}"),
            }
            next.workspace["config_version"] = v!(format!("version-{}", ordinal + 1));
            if matches!(self.fault, Fault::Diverge) {
                next.contexts[0]["future_metadata"] = v!({"lost":"unowned"});
            }
            *state = next;
        }
        if matches!(self.fault,Fault::LeaseAfter(n) if n==ordinal) {
            self.controls.observation.lock().unwrap().intent.generation += 1;
        }
        if matches!(self.fault, Fault::CancelAfter) {
            self.cancel.cancel();
        }
        if matches!(self.fault, Fault::TimeoutBefore | Fault::TimeoutAfter) {
            std::future::pending::<()>().await;
        }
        if matches!(self.fault, Fault::LostBefore | Fault::LostAfter)
            || matches!(self.fault,Fault::LostAt(n) if n==ordinal)
        {
            return m::WireReply::unknown(j::TransportOutcome::ConnectionLost);
        }
        let version = if matches!(self.fault, Fault::WrongVersion) {
            n("unrelated-version")
        } else {
            n(self.state.lock().unwrap().workspace["config_version"]
                .as_str()
                .unwrap())
        };
        m::WireReply {
            status: Some(status),
            body_hash: Some(d("synthetic-response")),
            version: Some(version),
            request_id: Some(n("synthetic-request")),
            outcome: j::TransportOutcome::Response,
            valid_response: !matches!(
                self.fault,
                Fault::MalformedResponse | Fault::MalformedRejection
            ),
        }
    }
}
#[derive(Clone, Copy, Debug)]
enum Verify {
    Missing,
    CompleteSynthetic,
    WrongBinding,
    Deadline,
    ExpiredApproval,
    LostAuthority,
}
struct Verifier<'a> {
    mode: Verify,
    clock: &'a c::tests::Clock,
    controls: &'a c::tests::Controls,
}

impl ac::ApplyVerifier for Verifier<'_> {
    async fn verify(
        &self,
        binding: &ac::VerificationBinding,
    ) -> ac::ApplyResult<ac::VerifiedServing> {
        match self.mode {
            Verify::Missing => {
                return Err(ac::ApplyError {
                    code: ac::ApplyCode::VerificationFailed,
                }
                .into())
            }
            Verify::Deadline => std::future::pending::<()>().await,
            Verify::ExpiredApproval => self.clock.0.store(10_000_000_000, Ordering::SeqCst),
            Verify::LostAuthority => self.controls.unavailable.store(true, Ordering::SeqCst),
            _ => (),
        }
        Ok(ac::VerifiedServing {binding:if matches!(self.mode,Verify::WrongBinding) {d("wrong")} else {ac::hash(binding).unwrap()},scope:EvidenceScope::SyntheticFixture,
            evidence:j::RestrictedJson::parse(br#"{"scope":"synthetic-fixture-only","V1":"modeled","V2":"modeled","V3":"modeled","V4":"modeled"}"#,&j::SecretGuard::default()).unwrap()})
    }
}
struct ResultRecord {
    report: ac::ApplyReport,
    calls: usize,
    reads: usize,
    recovered: Option<j::RecoveredRun>,
    state: Fixture,
    blocked: bool,
}
async fn run_case(
    shape: Shape,
    fault: Fault,
    storage: StorageFault,
    verification: Verify,
) -> ResultRecord {
    let case = case(shape).await;
    let store = Store::new(storage);
    let cert = j::tests::certificate();
    let guard = j::SecretGuard::default();
    let execution_target = c::tests::certificate(case.target.registration.clone());
    let coordinator = c::tests::Memory::new(&case.target.registration);
    let clock = c::tests::Clock(AtomicU64::new(1000));
    let owner = c::tests::owner();
    let controls = c::tests::Controls::new(&case.target.registration, &case.intent, &owner, 1000);
    let request = || c::ExecutionRequest {
        owner: owner.clone(),
        operation: c::Operation::Apply,
        intent: case.intent.clone(),
        plan: case.plan_hash.clone(),
    };
    let session = c::ExecutionSession::acquire(
        &coordinator,
        &controls,
        &clock,
        &execution_target,
        request(),
        None,
    )
    .await
    .unwrap();
    let cancel = Cancellation::default();
    if matches!(fault, Fault::CancelBefore) {
        cancel.cancel();
    }
    let verifier = Verifier {
        mode: verification,
        clock: &clock,
        controls: &controls,
    };
    let server = Server {
        state: Mutex::new(case.fixture),
        store: &store,
        coordinator: &coordinator,
        controls: &controls,
        cancel: &cancel,
        fault,
        calls: Mutex::new(vec![]),
        reads: AtomicUsize::new(0),
    };
    let runtime = ApplyRuntime {
        session: &session,
        target: &case.target,
        journal_store: &store,
        journal_certificate: &cert,
        guard: &guard,
        writer: &server,
        reader: &server,
        verifier: &verifier,
        cancellation: &cancel,
        limits: apply::ApplyLimits {
            read_attempts: 3,
            read_ms: 100,
            verification_ms: 20,
        },
    };
    let report = runtime.execute(case.prepared, None).await;
    let active = coordinator.row.lock().unwrap().1.active.clone().unwrap();
    let recovered = match active.journal.as_ref() {
        Some(anchor) => Some(
            j::recover(&store, &cert, &anchor.locator, Some(anchor))
                .await
                .unwrap(),
        ),
        None => None,
    };
    let blocked = c::ExecutionSession::acquire(
        &coordinator,
        &controls,
        &clock,
        &execution_target,
        request(),
        None,
    )
    .await
    .is_err();
    let calls = server.calls.lock().unwrap().len();
    let state = server.state.lock().unwrap().clone();
    ResultRecord {
        report,
        calls,
        reads: server.reads.load(Ordering::SeqCst),
        recovered,
        state,
        blocked,
    }
}

#[tokio::test]
async fn no_change_has_zero_sp_writes_or_markers_and_requires_real_verification() {
    for (verify, outcome, exit) in [
        (Verify::Missing, ac::ApplyOutcome::NoChangeUnverified, 6),
        (
            Verify::CompleteSynthetic,
            ac::ApplyOutcome::NoChangeVerified,
            0,
        ),
    ] {
        let r = run_case(Shape::Unchanged, Fault::None, StorageFault::None, verify).await;
        assert_eq!((r.report.outcome, r.report.exit_code), (outcome, exit));
        assert_eq!(r.calls, 0);
        assert!(r.blocked);
        assert_eq!(r.state.workspace["config_version"], "version-1");
        assert!(r.recovered.unwrap().unresolved.pending_requests.is_empty());
    }
}
#[tokio::test]
async fn prerequisites_are_separate_contexts_one_bulk_and_unowned_rows_survive() {
    let r = run_case(
        Shape::ThreeRequests,
        Fault::None,
        StorageFault::None,
        Verify::Missing,
    )
    .await;
    assert_eq!(
        (r.report.outcome, r.calls),
        (ac::ApplyOutcome::AppliedUnverified, 3)
    );
    let recovered = r.recovered.unwrap();
    assert_eq!(recovered.resolutions.len(), 3);
    assert!(recovered.unresolved.pending_requests.is_empty());
    let paths: Vec<_> = recovered
        .snapshot
        .requests()
        .iter()
        .map(j::PreparedRequest::path)
        .collect();
    assert_eq!(
        paths,
        [
            "/dimension/connector",
            "/default-config",
            "/context/bulk-operations"
        ]
    );
    assert_eq!(r.state.dimensions[0]["schema"]["enum"], v!(["b", "a"]));
    let r = run_case(
        Shape::TwoContexts,
        Fault::None,
        StorageFault::None,
        Verify::CompleteSynthetic,
    )
    .await;
    assert_eq!(
        (r.report.outcome, r.calls),
        (ac::ApplyOutcome::AppliedVerified, 1)
    );
    assert_eq!(r.state.contexts[0]["override"]["routing.note"], "retain");
    assert_eq!(
        r.state.contexts[0]["future_metadata"],
        v!({"nested":["B","A","A"],"keep":false})
    );
    assert_eq!(
        r.state.contexts[0]["description"],
        "manual description — preserve"
    );
    assert!(r.blocked);
}
#[tokio::test]
async fn prerequisite_only_progress_is_an_applied_outcome() {
    let r = run_case(
        Shape::PrerequisitesOnly,
        Fault::None,
        StorageFault::None,
        Verify::Missing,
    )
    .await;
    assert_eq!(
        (r.report.outcome, r.calls, r.report.exit_code),
        (ac::ApplyOutcome::AppliedUnverified, 1, 6)
    );
}
#[tokio::test]
async fn rejection_unknown_commit_and_after_observation_follow_exact_precedence() {
    use ac::ApplyOutcome as O;
    for (fault, outcome, exit) in [
        (Fault::Reject(1), O::RejectedNoChange, 7),
        (Fault::LostBefore, O::CommitUnknown, 5),
        (Fault::LostAfter, O::AppliedUnverified, 6),
        (Fault::PostCommit512, O::AppliedUnverified, 6),
        (Fault::Error500Before, O::CommitUnknown, 5),
        (Fault::SuccessBefore, O::CommitUnknown, 5),
        (Fault::WrongVersion, O::CommitUnknown, 5),
        (Fault::MalformedRejection, O::CommitUnknown, 5),
        (Fault::MalformedResponse, O::AppliedUnverified, 6),
        (Fault::ReadAfter, O::CommitUnknown, 5),
        (Fault::Diverge, O::CommitUnknown, 5),
        (Fault::TimeoutBefore, O::CommitUnknown, 5),
        (Fault::TimeoutAfter, O::AppliedUnverified, 6),
        (Fault::LeaseAfter(1), O::AppliedUnverified, 6),
        (Fault::CancelAfter, O::AppliedUnverified, 6),
    ] {
        let r = run_case(
            Shape::Changed,
            fault,
            StorageFault::None,
            Verify::CompleteSynthetic,
        )
        .await;
        assert_eq!(
            (r.report.outcome, r.report.exit_code),
            (outcome, exit),
            "{fault:?}"
        );
        assert_eq!(r.calls, 1, "never retry {fault:?}");
        assert!(r.blocked);
        if matches!(fault, Fault::ReadAfter) {
            assert_eq!(r.reads, 5);
        }
        if outcome == O::CommitUnknown {
            assert_eq!(r.recovered.unwrap().unresolved.pending_requests.len(), 1);
        }
    }
}
#[tokio::test]
async fn prior_commits_survive_context_rollback_or_later_guard_loss() {
    for (fault, calls) in [
        (Fault::Reject(3), 3),
        (Fault::LeaseAfter(1), 1),
        (Fault::EditBeforeBulk, 2),
    ] {
        let r = run_case(
            Shape::ThreeRequests,
            fault,
            StorageFault::None,
            Verify::Missing,
        )
        .await;
        assert_eq!(
            (r.report.outcome, r.report.exit_code),
            (ac::ApplyOutcome::PartialApplied, 4),
            "{fault:?}"
        );
        assert_eq!(r.calls, calls);
        assert!(r.state.contexts.is_empty());
        assert_eq!(r.state.dimensions[0]["schema"]["enum"], v!(["b", "a"]));
    }
    let r = run_case(
        Shape::ThreeRequests,
        Fault::LostAt(3),
        StorageFault::None,
        Verify::Missing,
    )
    .await;
    assert_eq!(
        (r.report.outcome, r.report.exit_code, r.calls),
        (ac::ApplyOutcome::CommitUnknown, 5, 3)
    );
    assert_eq!(r.report.committed.len(), 2);
    assert_eq!(r.recovered.unwrap().unresolved.pending_requests.len(), 1);
}
#[tokio::test]
async fn storage_faults_stop_before_send_or_leave_a_durable_unknown_intent() {
    for (fault, outcome, calls) in [
        (StorageFault::Prepare, ac::ApplyOutcome::Refused, 0),
        (StorageFault::Intent, ac::ApplyOutcome::Refused, 0),
        (StorageFault::IntentLostAck, ac::ApplyOutcome::Refused, 0),
        (StorageFault::Transport, ac::ApplyOutcome::CommitUnknown, 1),
        (
            StorageFault::Resolution,
            ac::ApplyOutcome::AppliedUnverified,
            1,
        ),
        (
            StorageFault::FinalReport,
            ac::ApplyOutcome::AppliedUnverified,
            1,
        ),
    ] {
        let r = run_case(
            Shape::Changed,
            Fault::None,
            fault,
            Verify::CompleteSynthetic,
        )
        .await;
        assert_eq!(r.report.outcome, outcome, "{fault:?}");
        assert_eq!(r.calls, calls);
        assert!(r.blocked);
        if matches!(
            fault,
            StorageFault::IntentLostAck | StorageFault::Transport | StorageFault::Resolution
        ) {
            assert_eq!(r.recovered.unwrap().unresolved.pending_requests.len(), 1);
        }
    }
}
#[tokio::test]
async fn missing_mismatched_or_timed_out_verification_never_reports_success() {
    for verify in [
        Verify::Missing,
        Verify::WrongBinding,
        Verify::Deadline,
        Verify::ExpiredApproval,
        Verify::LostAuthority,
    ] {
        let r = run_case(Shape::Changed, Fault::None, StorageFault::None, verify).await;
        assert_eq!(
            (r.report.outcome, r.report.exit_code),
            (ac::ApplyOutcome::AppliedUnverified, 6)
        );
        assert_eq!(r.calls, 1);
    }
}
#[tokio::test]
async fn read_retry_restarts_discovery_but_missing_barrier_blocks_dispatch() {
    let cancelled = run_case(
        Shape::Changed,
        Fault::CancelBefore,
        StorageFault::None,
        Verify::Missing,
    )
    .await;
    assert_eq!(
        (
            cancelled.report.outcome,
            cancelled.report.exit_code,
            cancelled.calls
        ),
        (ac::ApplyOutcome::Refused, 2, 0)
    );
    assert!(cancelled.recovered.is_none());
    let r = run_case(
        Shape::Changed,
        Fault::RetryInitialRead,
        StorageFault::None,
        Verify::Missing,
    )
    .await;
    assert_eq!(r.calls, 1);
    assert_eq!(r.reads, 4);
    let r = run_case(
        Shape::Changed,
        Fault::GateBeforeFirst,
        StorageFault::None,
        Verify::Missing,
    )
    .await;
    assert_eq!(
        (r.report.outcome, r.report.exit_code, r.calls),
        (ac::ApplyOutcome::Refused, 3, 0)
    );
    assert!(r.blocked);
}
#[tokio::test]
async fn entire_plan_limits_and_unsupported_contracts_refuse_before_any_write() {
    let mut case = case(Shape::TwoContexts).await;
    let guard = j::SecretGuard::default();
    assert_eq!(case.frozen.len(), 1);
    let body: Value = serde_json::from_slice(case.frozen[0].body()).unwrap();
    assert_eq!(body["operations"].as_array().unwrap().len(), 2);
    case.target.max_context_actions = 1;
    assert!(m::compile(&case.actions, &case.target, &guard).is_err());
    case.target.max_context_actions = 100;
    case.target.max_body = case.frozen[0].body().len() - 1;
    assert!(m::compile(&case.actions, &case.target, &guard).is_err());
    case.target.max_body = 512 * 1024;
    case.target.rollback_statuses.insert(512);
    assert!(m::compile(&case.actions, &case.target, &guard).is_err());
    case.target.rollback_statuses.remove(&512);
    // A malformed last item refuses compilation of the whole plan, including earlier valid items.
    let last = case.actions.last_mut().unwrap();
    let mut row: Value = serde_json::from_str(last.after.as_ref().unwrap().raw()).unwrap();
    row["description"] = Value::Null;
    last.after = Some(Json::parse(&serde_json::to_vec(&row).unwrap()).unwrap());
    assert!(m::compile(&case.actions, &case.target, &guard).is_err());
    assert!(ac::CertifiedMutationTarget::from_registry(&n("anything")).is_err());
    assert!(m::AdministrationMutator::from_registered(
        &n("anything"),
        reqwest::header::HeaderValue::from_static("synthetic")
    )
    .is_err());
    assert!(apply::journal_capacity(1024, 1024, &case.frozen).is_ok());
    let many = vec![case.frozen[0].clone(); 32];
    assert!(apply::journal_capacity(32 * 1024 * 1024, 64 * 1024 * 1024, &many).is_err());
    assert!(apply::journal_capacity(usize::MAX, 1, &case.frozen).is_err());
}

#[tokio::test]
async fn dropping_executor_after_send_retains_recoverable_pending_intent_and_blocks_successor() {
    let case = case(Shape::Changed).await;
    let store = Store::new(StorageFault::None);
    let cert = j::tests::certificate();
    let guard = j::SecretGuard::default();
    let target = c::tests::certificate(case.target.registration.clone());
    let coordinator = c::tests::Memory::new(&case.target.registration);
    let clock = c::tests::Clock(AtomicU64::new(1000));
    let owner = c::tests::owner();
    let controls = c::tests::Controls::new(&case.target.registration, &case.intent, &owner, 1000);
    let request = || c::ExecutionRequest {
        owner: owner.clone(),
        operation: c::Operation::Apply,
        intent: case.intent.clone(),
        plan: case.plan_hash.clone(),
    };
    let session =
        c::ExecutionSession::acquire(&coordinator, &controls, &clock, &target, request(), None)
            .await
            .unwrap();
    let cancel = Cancellation::default();
    let server = Server {
        state: Mutex::new(case.fixture),
        store: &store,
        coordinator: &coordinator,
        controls: &controls,
        cancel: &cancel,
        fault: Fault::Pause,
        calls: Mutex::new(vec![]),
        reads: AtomicUsize::new(0),
    };
    let verifier = ac::UnavailableVerification;
    let runtime = ApplyRuntime {
        session: &session,
        target: &case.target,
        journal_store: &store,
        journal_certificate: &cert,
        guard: &guard,
        writer: &server,
        reader: &server,
        verifier: &verifier,
        cancellation: &cancel,
        limits: apply::ApplyLimits::default(),
    };
    {
        let work = runtime.execute(case.prepared, None);
        tokio::pin!(work);
        tokio::select! {
            _ = &mut work => panic!("must stop the executor while its request is pending"),
            () = async {while server.calls.lock().unwrap().is_empty() {tokio::task::yield_now().await;}} => (),
        }
    }
    let active = coordinator.row.lock().unwrap().1.active.clone().unwrap();
    assert_eq!(active.phase, c::RunPhase::RequestInFlight);
    let anchor = active.journal.unwrap();
    let recovered = j::recover(&store, &cert, &anchor.locator, Some(&anchor))
        .await
        .unwrap();
    assert_eq!(recovered.unresolved.pending_requests.len(), 1);
    assert!(recovered.observations.is_empty());
    assert!(recovered.run_reports.is_empty());
    assert!(session.check_authority().await.is_err());
    assert!(c::ExecutionSession::acquire(
        &coordinator,
        &controls,
        &clock,
        &target,
        request(),
        None
    )
    .await
    .is_err());
    assert_eq!(server.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn one_context_transaction_rolls_back_all_actions_and_reports_are_sanitized() {
    let original = case(Shape::TwoContexts).await.fixture;
    let r = run_case(
        Shape::TwoContexts,
        Fault::Reject(1),
        StorageFault::None,
        Verify::Missing,
    )
    .await;
    assert_eq!(
        (r.report.outcome, r.report.exit_code, r.calls),
        (ac::ApplyOutcome::RejectedNoChange, 7, 1)
    );
    assert_eq!(r.state.contexts, original.contexts);
    let report = serde_json::to_string(&r.report).unwrap();
    for private in [
        "routing.note",
        "manual description",
        "fixture-org",
        "fixture-workspace",
        "future_metadata",
    ] {
        assert!(!report.contains(private));
    }
}
#[test]
fn all_outcome_exit_codes_and_unknown_over_partial_are_explicit() {
    use ac::{ApplyCode as C, ApplyOutcome as O};
    for (planned, sent, committed, rejected, unknown, preflight, verified, reason, want) in [
        (0, 0, 0, 0, 0, true, true, None, (O::NoChangeVerified, 0)),
        (0, 0, 0, 0, 0, true, false, None, (O::NoChangeUnverified, 6)),
        (
            1,
            0,
            0,
            0,
            0,
            false,
            false,
            Some(C::InvalidInput),
            (O::Refused, 2),
        ),
        (
            1,
            0,
            0,
            0,
            0,
            false,
            false,
            Some(C::NotAuthoritative),
            (O::Refused, 3),
        ),
        (1, 1, 0, 1, 0, true, false, None, (O::RejectedNoChange, 7)),
        (1, 1, 1, 0, 0, true, true, None, (O::AppliedVerified, 0)),
        (3, 1, 1, 0, 0, true, false, None, (O::PartialApplied, 4)),
        (3, 2, 1, 0, 1, true, false, None, (O::CommitUnknown, 5)),
        (
            1,
            1,
            1,
            0,
            0,
            true,
            false,
            Some(C::LeaseLost),
            (O::AppliedUnverified, 6),
        ),
    ] {
        assert_eq!(
            ac::classify(ac::OutcomeFacts {
                planned,
                dispatched: sent,
                committed,
                rejected,
                unknown,
                preflight,
                verified,
                reason
            }),
            want
        );
    }
}

#[path = "recovery_tests.rs"]
mod recovery_tests;
