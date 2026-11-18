//! Synthetic end-to-end recovery using real journal/planner/coordinator/apply modules.
//! Neither these fixtures nor their modeled V1–V4 receipts certify a platform or server.
use super::{super::super::super::recovery::*, *};

struct Original {
    target: ac::CertifiedMutationTarget,
    intent: a::ReleaseIntent,
    store: Store,
    coordinator: c::tests::Memory,
    state: Fixture,
    anchor: j::UnresolvedRunRecord,
    old_calls: usize,
}
async fn original(shape: Shape, fault: Fault, storage: StorageFault) -> Original {
    let case = case(shape).await;
    let store = Store::new(storage);
    let cert = j::tests::certificate();
    let guard = j::SecretGuard::default();
    let target = c::tests::certificate(case.target.registration.clone());
    let coordinator = c::tests::Memory::new(&case.target.registration);
    let clock = c::tests::Clock(AtomicU64::new(1000));
    let owner = c::tests::owner();
    let controls = c::tests::Controls::new(&case.target.registration, &case.intent, &owner, 1000);
    let session = c::ExecutionSession::acquire(
        &coordinator,
        &controls,
        &clock,
        &target,
        c::ExecutionRequest {
            owner,
            operation: c::Operation::Apply,
            intent: case.intent.clone(),
            plan: case.plan_hash.clone(),
        },
        None,
    )
    .await
    .unwrap();
    let cancel = Cancellation::default();
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
    let verifier = Verifier {
        mode: Verify::Missing,
        clock: &clock,
        controls: &controls,
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
            read_attempts: 1,
            read_ms: 100,
            verification_ms: 20,
        },
    };
    let report = runtime.execute(case.prepared, None).await;
    assert_ne!(report.exit_code, 0);
    let anchor = coordinator
        .row
        .lock()
        .unwrap()
        .1
        .active
        .as_ref()
        .unwrap()
        .journal
        .clone()
        .unwrap();
    let recovered = j::recover(&store, &cert, &anchor.locator, Some(&anchor))
        .await
        .unwrap();
    let state = server.state.lock().unwrap().clone();
    let old_calls = server.calls.lock().unwrap().len();
    drop(server);
    drop(session);
    Original {
        target: case.target,
        intent: case.intent,
        store,
        coordinator,
        state,
        anchor: recovered.unresolved,
        old_calls,
    }
}
async fn complete(state: &Fixture) -> CompleteInventory {
    inventory::discover(
        &CurrentVersion(state.clone()),
        &inventory::tests::target(),
        InventoryLimits::default(),
    )
    .await
    .unwrap()
}
async fn observe(old: &Original, current: &CompleteInventory) -> RecoveryObservation {
    RecoverySource::load(
        &old.store,
        &j::tests::certificate(),
        &old.anchor.locator,
        Some(&old.anchor),
    )
    .await
    .unwrap()
    .observe(current, &old.target, &j::SecretGuard::default())
    .unwrap()
}
async fn basis(old: &Original) -> (VerifiedDesired, Value) {
    let run = j::recover(
        &old.store,
        &j::tests::certificate(),
        &old.anchor.locator,
        Some(&old.anchor),
    )
    .await
    .unwrap();
    let root: Value = serde_json::from_slice(run.snapshot.contents().bytes()).unwrap();
    let mut bundle: Value = serde_json::from_str(root["bundle"].as_str().unwrap()).unwrap();
    let mut desired = desired();
    desired.desired.contexts.clear();
    for entry in bundle["desired"]["contexts"].as_array().unwrap() {
        desired.desired.contexts.insert(
            serde_json::from_value(entry[0].clone()).unwrap(),
            serde_json::from_value(entry[1].clone()).unwrap(),
        );
    }
    desired.catalog = serde_json::from_value(bundle["catalog"].clone()).unwrap();
    let mut release: ReleaseIdentity =
        serde_json::from_value(bundle["desired"]["input_manifest"]["build"]["release"].clone())
            .unwrap();
    release.artifact = ArtifactIdentity::Image {
        index_digest: d("recovery-image"),
        platform_digests: BTreeMap::from([(n("linux-arm64"), d("recovery-image"))]),
    };
    desired.release_hash = digest(&release).unwrap();
    desired.desired.input_manifest.build = BuildIdentity::from_feature_keys(
        release,
        "REQUIRED_FIELDS_SYNC,V1",
        "REQUIRED_FIELDS_SYNC,V1",
    )
    .unwrap();
    bundle["desired"] = v!(desired.desired);
    desired.bundle_hash = canonical_hash(HashDomain::Bundle, &bundle).unwrap();
    (desired, bundle)
}
fn planning<'a>(
    inventory: &'a CompleteInventory,
    desired: &'a VerifiedDesired,
    ownership: &'a OwnershipManifest,
    prerequisites: &'a PrerequisitePolicy,
    contract: &'a ProjectionContract,
) -> RecoveryPlanning<'a> {
    RecoveryPlanning {
        inventory,
        desired,
        ownership,
        prerequisites,
        contract,
        intent: PlanIntent {
            authority: contract.authority.clone(),
            release_hash: desired.release_hash.clone(),
            generation: 2,
        },
        limits: PlanLimits::default(),
    }
}
fn accept_recovery(
    candidate: RecoveryCandidate,
    projection: &mut ProjectionContract,
) -> AcceptedRecoveryPlan {
    let reports = synthetic_reports(candidate.candidate(), projection);
    let slices = reports.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let evidence = examine_cases(candidate.candidate(), projection, &slices).unwrap();
    let approval = approval(candidate.candidate(), &evidence);
    candidate
        .accept(projection, &slices, Some(&approval), 0)
        .unwrap()
}
fn recovery_intent(
    old: &Original,
    observation: &RecoveryObservation,
    plan: &ReadOnlyPlan,
    desired: &VerifiedDesired,
) -> a::ReleaseIntent {
    let mut intent = old.intent.clone();
    intent.generation = 2;
    intent.executor_image = d("recovery-image");
    intent.release = desired.release_hash.clone();
    intent.bundle = plan.report().inputs.bundle_hash.clone();
    let manifest = &desired.desired.input_manifest;
    intent.input_manifest = canonical_hash(HashDomain::InputManifest, manifest).unwrap();
    intent.build = hash(&json(&manifest.build).unwrap()).unwrap();
    // Current serving reader still uses the old application image; it need not have the command.
    intent.recovery = Some(observation.intent(&intent.readers).unwrap());
    intent
}
fn image() -> RetainedRecoveryImage {
    RetainedRecoveryImage {
        image: d("recovery-image"),
        receipt: d("retention-receipt"),
        scope: EvidenceScope::SyntheticFixture,
        retained_until_unix: 100_000_000,
    }
}
fn retention() -> j::Retention {
    j::Retention {
        created_at_unix: 1,
        retain_until_unix: 1 + j::MIN_RETENTION_SECONDS,
        rollback_window_seconds: 86400,
        protected_hold: true,
    }
}
fn identity(old: &Original) -> j::RecoveryIdentity {
    j::RecoveryIdentity {
        target: old.target.registration.target.clone(),
        endpoint_identity: old.target.registration.endpoint_identity.clone(),
        coordinator: old.target.registration.coordinator.object.clone(),
        executor_image: d("recovery-image"),
        retained_recovery_image: d("recovery-image"),
    }
}
fn authorize_controls(
    old: &Original,
    intent: &a::ReleaseIntent,
    owner: &c::Owner,
    observation: &RecoveryObservation,
) -> c::tests::Controls {
    let controls = c::tests::Controls::new(&old.target.registration, intent, owner, 1000);
    let previous = old
        .coordinator
        .row
        .lock()
        .unwrap()
        .1
        .active
        .as_ref()
        .unwrap()
        .owner
        .clone();
    controls.observation.lock().unwrap().barrier.predecessor = Some(a::PreviousExecutorExclusion {
        run: previous.run,
        owner: previous.job_uid,
        revoked_and_stopped: true,
        accepted_requests_drained: true,
        direct_state_observed: true,
        journal_head: observation.report().journal_head.clone(),
        observed_inventory: observation.report().current_inventory.clone(),
        receipt: d("synthetic-drain-evidence"),
    });
    controls
}

#[tokio::test]
async fn observes_lost_create_and_crash_boundaries_without_dispatch_or_assuming_timeout_cancelled()
{
    for (shape, fault, storage, want, calls) in [
        (
            Shape::TwoContexts,
            Fault::ReadAfter,
            StorageFault::None,
            RequestState::After,
            1,
        ),
        (
            Shape::Changed,
            Fault::LostBefore,
            StorageFault::None,
            RequestState::Before,
            1,
        ),
        (
            Shape::Changed,
            Fault::None,
            StorageFault::IntentLostAck,
            RequestState::Before,
            0,
        ),
    ] {
        let old = original(shape, fault, storage).await;
        let current = complete(&old.state).await;
        let observation = observe(&old, &current).await;
        assert_eq!(observation.report().requests[0].state, want);
        assert!(observation.report().requires_executor_and_request_exclusion);
        assert_eq!(old.old_calls, calls);
        assert_eq!(observation.report().hosted_apply, "disabled");
    }
}

#[tokio::test]
async fn three_way_restores_absence_and_order_preserving_current_unrelated_data() {
    let mut old = original(Shape::Changed, Fault::None, StorageFault::None).await;
    old.state.contexts[0]["description"] = v!("new manual description");
    old.state.contexts[0]["override"]["routing.note"] =
        v!({"ordered":["Y","X","Y"],"large":"retained"});
    old.state.contexts[0]["future_metadata"] = v!({"manual":true});
    let inventory = complete(&old.state).await;
    let observation = observe(&old, &inventory).await;
    let (desired, _) = basis(&old).await;
    let ownership = grants(&inventory);
    let prerequisites = prereqs(&desired);
    let projection = contract(&inventory, &desired);
    let input = || {
        planning(
            &inventory,
            &desired,
            &ownership,
            &prerequisites,
            &projection,
        )
    };
    let draft = observation
        .restore_draft(
            input(),
            RestoreOrigin::RunBefore,
            &j::SecretGuard::default(),
        )
        .unwrap();
    let candidate = draft.prepare(&observation, input()).unwrap();
    let after: Value =
        serde_json::from_str(candidate.candidate().actions()[0].after_json().unwrap()).unwrap();
    assert_eq!(after["override"][OPTIONS], v!(["A", "B"]));
    assert_eq!(
        after["override"]["routing.note"],
        old.state.contexts[0]["override"]["routing.note"]
    );
    assert_eq!(after["description"], "new manual description");
    assert_eq!(after["future_metadata"], v!({"manual":true}));
}

#[tokio::test]
async fn refuses_divergent_owned_values_and_wrong_identity_or_missing_grants() {
    for fault in [
        "owned",
        "owned_unchanged",
        "id",
        "grant",
        "instance",
        "unknown",
    ] {
        let mut old = original(
            Shape::Changed,
            if fault == "unknown" {
                Fault::LostBefore
            } else {
                Fault::None
            },
            StorageFault::None,
        )
        .await;
        if fault == "owned_unchanged" {
            old.state.contexts[0]["override"][REQUIRED] = v!(false);
        }
        if fault == "owned" {
            old.state.contexts[0]["override"][OPTIONS] = v!(["B", "A"]);
        }
        if fault == "id" {
            old.state.contexts[0]["id"] = v!("manual-recreated-id");
        }
        if fault == "unknown" {
            old.state.contexts[0]["description"] = v!("concurrent edit during unknown request");
        }
        let inventory = complete(&old.state).await;
        let source = RecoverySource::load(
            &old.store,
            &j::tests::certificate(),
            &old.anchor.locator,
            Some(&old.anchor),
        )
        .await
        .unwrap();
        if fault == "instance" {
            old.target.registration.target.instance_generation += 1;
            assert!(source
                .observe(&inventory, &old.target, &j::SecretGuard::default())
                .is_err());
            continue;
        }
        let observation = source
            .observe(&inventory, &old.target, &j::SecretGuard::default())
            .unwrap();
        let (desired, _) = basis(&old).await;
        let mut ownership = grants(&inventory);
        if fault == "grant" {
            ownership.grants.clear();
        }
        let prerequisites = prereqs(&desired);
        let projection = contract(&inventory, &desired);
        assert!(
            observation
                .restore_draft(
                    planning(
                        &inventory,
                        &desired,
                        &ownership,
                        &prerequisites,
                        &projection
                    ),
                    RestoreOrigin::RunBefore,
                    &j::SecretGuard::default()
                )
                .is_err(),
            "{fault}"
        );
    }
}

#[tokio::test]
async fn created_row_with_new_unrelated_key_is_retained_and_additive_prerequisites_are_reported() {
    let mut old = original(Shape::ThreeRequests, Fault::None, StorageFault::None).await;
    old.state.contexts[0]["override"]["routing.note"] = v!("manual new value");
    old.state.contexts[0]["description"] = v!("manual new description");
    let inventory = complete(&old.state).await;
    let observation = observe(&old, &inventory).await;
    let (desired, _) = basis(&old).await;
    let mut ownership = grants(&inventory);
    let prerequisites = prereqs(&desired);
    let projection = contract(&inventory, &desired);
    let draft = observation
        .restore_draft(
            planning(
                &inventory,
                &desired,
                &ownership,
                &prerequisites,
                &projection,
            ),
            RestoreOrigin::FirstAdoption,
            &j::SecretGuard::default(),
        )
        .unwrap();
    ownership.retirement = Some(RetirementManifest {
        bundle_hash: draft.bundle_hash().clone(),
        intent_generation: 2,
        server_ids: ownership
            .grants
            .iter()
            .map(|g| g.server_id.clone())
            .collect(),
        approver: n("owner"),
        review: n("restore-review"),
    });
    let candidate = draft
        .prepare(
            &observation,
            planning(
                &inventory,
                &desired,
                &ownership,
                &prerequisites,
                &projection,
            ),
        )
        .unwrap();
    assert_eq!(candidate.retained_prerequisites().len(), 2);
    assert_eq!(candidate.candidate().actions().len(), 1);
    assert_eq!(
        candidate.candidate().actions()[0].kind(),
        &ActionKind::Replace
    );
    let after: Value =
        serde_json::from_str(candidate.candidate().actions()[0].after_json().unwrap()).unwrap();
    assert_eq!(after["override"], v!({"routing.note":"manual new value"}));
    assert_eq!(after["description"], "manual new description");
}

async fn recover_and_execute_fault(
    old: Original,
    restore: bool,
    verify: Verify,
    lost: bool,
    write_fault: Fault,
) -> (ac::ApplyReport, usize, Fixture, j::RecoveredRun) {
    let inventory = complete(&old.state).await;
    let observation = observe(&old, &inventory).await;
    let (desired, bundle) = basis(&old).await;
    let ownership = grants(&inventory);
    let prerequisites = prereqs(&desired);
    let mut projection = contract(&inventory, &desired);
    let input = || {
        planning(
            &inventory,
            &desired,
            &ownership,
            &prerequisites,
            &projection,
        )
    };
    let candidate = if restore {
        observation
            .restore_draft(
                input(),
                RestoreOrigin::RunBefore,
                &j::SecretGuard::default(),
            )
            .unwrap()
            .prepare(&observation, input())
            .unwrap()
    } else {
        observation.prepare_resume(input()).unwrap()
    };
    let accepted = accept_recovery(candidate, &mut projection);
    let intent = recovery_intent(&old, &observation, accepted.plan(), &desired);
    let owner = c::Owner {
        job_uid: n("recovery-job"),
        run: d("recovery-run"),
    };
    let controls = authorize_controls(&old, &intent, &owner, &observation);
    let target = c::tests::certificate(old.target.registration.clone());
    let clock = c::tests::Clock(AtomicU64::new(1000));
    let active = old
        .coordinator
        .row
        .lock()
        .unwrap()
        .1
        .active
        .clone()
        .unwrap();
    let cert = j::tests::certificate();
    let guard = j::SecretGuard::default();
    let proof = c::inspect_recovery(&old.store, &cert, &active)
        .await
        .unwrap();
    let session = c::ExecutionSession::acquire(
        &old.coordinator,
        &controls,
        &clock,
        &target,
        c::ExecutionRequest {
            owner: owner.clone(),
            operation: if restore {
                c::Operation::Restore
            } else {
                c::Operation::Resume
            },
            intent,
            plan: accepted.plan().report().plan_hash.clone().unwrap(),
        },
        Some(&proof),
    )
    .await
    .unwrap();
    let auth = observation
        .record(
            &accepted,
            &session,
            &old.store,
            &cert,
            &image(),
            &retention(),
            &guard,
        )
        .await
        .unwrap();
    let baseline = auth.baseline.clone();
    let capture = j::Capture {
        plan: accepted.plan(),
        inventory: &inventory,
        ownership: &ownership,
        prerequisites: &prerequisites,
        projection: &projection,
        bundle: accepted.bundle().cloned().unwrap_or_else(|| {
            j::RestrictedJson::parse(&serde_json::to_vec(&bundle).unwrap(), &guard).unwrap()
        }),
        identity: identity(&old),
        retention: retention(),
        requests: vec![],
        unreadable_attributes: vec![],
        server_managed_attributes: old.target.context_audit.iter().map(|s| n(s)).collect(),
    };
    let prepared = if matches!(write_fault, Fault::CancelBefore) {
        PreparedApply::prepare(capture, &old.target, &guard).unwrap()
    } else {
        PreparedApply::prepare_recovery(capture, &old.target, &guard, auth).unwrap()
    };
    if lost {
        controls
            .observation
            .lock()
            .unwrap()
            .barrier
            .predecessor
            .as_mut()
            .unwrap()
            .accepted_requests_drained = false;
    }
    let cancel = Cancellation::default();
    let verifier = Verifier {
        mode: verify,
        clock: &clock,
        controls: &controls,
    };
    let server = Server {
        state: Mutex::new(old.state.clone()),
        store: &old.store,
        coordinator: &old.coordinator,
        controls: &controls,
        cancel: &cancel,
        fault: write_fault,
        calls: Mutex::new(vec![]),
        reads: AtomicUsize::new(0),
    };
    let report = ApplyRuntime {
        session: &session,
        target: &old.target,
        journal_store: &old.store,
        journal_certificate: &cert,
        guard: &guard,
        writer: &server,
        reader: &server,
        verifier: &verifier,
        cancellation: &cancel,
        limits: apply::ApplyLimits {
            read_attempts: 1,
            read_ms: 100,
            verification_ms: 20,
        },
    }
    .execute(prepared, Some(&baseline))
    .await;
    let calls = server.calls.lock().unwrap().len();
    let state = server.state.lock().unwrap().clone();
    let recovered = if lost || matches!(write_fault, Fault::CancelBefore) {
        j::recover(&old.store, &cert, &old.anchor.locator, Some(&old.anchor))
            .await
            .unwrap()
    } else {
        j::recover_run(
            &old.store,
            &cert,
            &old.target.registration.target,
            &owner.run,
            None,
        )
        .await
        .unwrap()
    };
    if !lost && !matches!(write_fault, Fault::CancelBefore) {
        let unavailable = BrokenStore {
            inner: &old.store,
            mode: "readback",
        };
        assert!(
            j::recover_run(
                &unavailable,
                &cert,
                &old.target.registration.target,
                &owner.run,
                None
            )
            .await
            .is_err(),
            "missing recovery decision cannot be ignored by a later reader"
        );
    }
    (report, calls, state, recovered)
}
#[tokio::test]
async fn resume_observes_commit_without_replay_and_still_requires_verification() {
    for verify in [Verify::Missing, Verify::CompleteSynthetic] {
        let old = original(Shape::Changed, Fault::ReadAfter, StorageFault::None).await;
        let (report, calls, _, recovered) = recover_and_execute(old, false, verify, false).await;
        assert_eq!(calls, 0);
        assert_eq!(
            report.outcome,
            if matches!(verify, Verify::Missing) {
                ac::ApplyOutcome::NoChangeUnverified
            } else {
                ac::ApplyOutcome::NoChangeVerified
            }
        );
        assert!(recovered.snapshot.requests().is_empty());
        assert!(recovered.snapshot.data.recovery.is_some());
        assert_eq!(
            recovered.first_adoption.data.identity.executor_image,
            d("image")
        );
    }
}
#[tokio::test]
async fn restore_runs_existing_apply_journal_and_verification_gates_and_preserves_baseline() {
    let old = original(Shape::Changed, Fault::None, StorageFault::None).await;
    let baseline_hash = old.anchor.first_adoption.hash().clone();
    let (report, calls, state, recovered) =
        recover_and_execute(old, true, Verify::Missing, false).await;
    assert_eq!(calls, 1);
    assert_eq!(report.outcome, ac::ApplyOutcome::AppliedUnverified);
    assert_eq!(state.contexts[0]["override"][OPTIONS], v!(["A", "B"]));
    assert_eq!(recovered.unresolved.first_adoption.hash(), &baseline_hash);
    assert_eq!(recovered.resolutions.len(), 1);
}
#[tokio::test]
async fn before_state_replans_once_only_after_exclusion_and_lost_exclusion_blocks_dispatch() {
    let old = original(Shape::Changed, Fault::LostBefore, StorageFault::None).await;
    let (report, calls, _, _) = recover_and_execute(old, false, Verify::Missing, false).await;
    assert_eq!(calls, 1);
    assert_eq!(report.outcome, ac::ApplyOutcome::AppliedUnverified);
    let old = original(Shape::Changed, Fault::LostBefore, StorageFault::None).await;
    let (report, calls, _, _) =
        recover_and_execute(old, false, Verify::CompleteSynthetic, true).await;
    assert_eq!(calls, 0);
    assert_eq!(report.outcome, ac::ApplyOutcome::Refused);
}

#[tokio::test]
async fn recreation_uses_saved_description_new_server_identity_and_current_prerequisite_experiment_gates(
) {
    for fault in ["none", "experiment", "dimension", "occupied"] {
        let mut old = original(Shape::Deleted, Fault::None, StorageFault::None).await;
        assert!(old.state.contexts.is_empty());
        if fault == "experiment" {
            old.state.experiments.push(experiment(
                "INPROGRESS",
                v!({"connector":"a"}),
                Value::Null,
                Value::Null,
            ));
        }
        if fault == "dimension" {
            old.state.dimensions[0]["position"] = v!(4);
        }
        if fault == "occupied" {
            old.state.contexts.push(row(
                "manual-row",
                v!(condition("a")),
                v!({REQUIRED:true,OPTIONS:["A","B"]}),
            ));
        }
        let inventory = complete(&old.state).await;
        let observation = observe(&old, &inventory).await;
        let (desired, _) = basis(&old).await;
        let mut ownership = grants(&inventory);
        if fault != "occupied" {
            ownership.creations.push(CreationGrant {
                intent_id: n("recreate-original"),
                condition: condition("a"),
                attributes: BTreeSet::from([key(REQUIRED), key(OPTIONS)]),
                approver: n("owner"),
                review: n("recreation"),
            });
        }
        let prerequisites = prereqs(&desired);
        let projection = contract(&inventory, &desired);
        let inputs = || {
            planning(
                &inventory,
                &desired,
                &ownership,
                &prerequisites,
                &projection,
            )
        };
        let draft = observation.restore_draft(
            inputs(),
            RestoreOrigin::RunBefore,
            &j::SecretGuard::default(),
        );
        if fault == "occupied" {
            assert!(draft.is_err());
            continue;
        }
        let candidate = draft.unwrap().prepare(&observation, inputs());
        if fault != "none" {
            assert!(candidate.is_err(), "{fault}");
            continue;
        }
        let candidate = candidate.unwrap();
        let action = &candidate.candidate().actions()[0];
        assert_eq!(action.kind(), &ActionKind::Create);
        assert!(action.server_id().is_none());
        let after: Value = serde_json::from_str(action.after_json().unwrap()).unwrap();
        assert_eq!(after["description"], "manual description — preserve");
        assert!(after.get("id").is_none());
        assert_eq!(after["override"], v!({REQUIRED:true,OPTIONS:["A","B"]}));
    }
}
#[tokio::test]
async fn first_adoption_delete_needs_unchanged_owned_row_and_exact_retirement_review() {
    for fault in [
        "none",
        "description",
        "metadata",
        "deletion_grant",
        "retirement",
    ] {
        let mut old = original(Shape::ThreeRequests, Fault::None, StorageFault::None).await;
        if fault == "description" {
            old.state.contexts[0]["description"] = v!("new manual description");
        }
        if fault == "metadata" {
            old.state.contexts[0]["future_metadata"] = v!({"new":true});
        }
        let inventory = complete(&old.state).await;
        let observation = observe(&old, &inventory).await;
        let (desired, _) = basis(&old).await;
        let mut ownership = grants(&inventory);
        ownership.grants[0].allow_row_deletion = fault != "deletion_grant";
        let prerequisites = prereqs(&desired);
        let projection = contract(&inventory, &desired);
        let draft = observation.restore_draft(
            planning(
                &inventory,
                &desired,
                &ownership,
                &prerequisites,
                &projection,
            ),
            RestoreOrigin::FirstAdoption,
            &j::SecretGuard::default(),
        );
        if ["description", "metadata", "deletion_grant"].contains(&fault) {
            assert!(draft.is_err(), "{fault}");
            continue;
        }
        let draft = draft.unwrap();
        if fault != "retirement" {
            ownership.retirement = Some(RetirementManifest {
                bundle_hash: draft.bundle_hash().clone(),
                intent_generation: 2,
                server_ids: ownership
                    .grants
                    .iter()
                    .map(|g| g.server_id.clone())
                    .collect(),
                approver: n("owner"),
                review: n("adoption-undo"),
            });
        }
        let candidate = draft.prepare(
            &observation,
            planning(
                &inventory,
                &desired,
                &ownership,
                &prerequisites,
                &projection,
            ),
        );
        if fault == "retirement" {
            assert!(candidate.is_err());
            continue;
        }
        let candidate = candidate.unwrap();
        assert_eq!(
            candidate.candidate().actions()[0].kind(),
            &ActionKind::Delete
        );
        assert_eq!(candidate.retained_prerequisites().len(), 2);
    }
}

struct BrokenStore<'a> {
    inner: &'a Store,
    mode: &'static str,
}
impl j::JournalReader for BrokenStore<'_> {
    async fn read(&self, key: &str, max: usize) -> j::JournalResult<Option<j::StoredObject>> {
        let mut value = self.inner.read(key, max).await?;
        if self.mode == "missing" && key.ends_with("/prepared") {
            return Ok(None);
        }
        if self.mode == "corrupt" && key.ends_with("/prepared") {
            value.as_mut().unwrap().bytes.pop();
        }
        if self.mode == "readback" && key.ends_with("/recovery") {
            return Ok(None);
        }
        Ok(value)
    }
}
impl j::ImmutableJournalStore for BrokenStore<'_> {
    async fn create(
        &self,
        key: &str,
        bytes: &[u8],
        retention: &j::Retention,
    ) -> j::JournalResult<Name> {
        if self.mode == "write" && key.ends_with("/recovery") {
            return Err(j::JournalError {
                code: j::JournalCode::AccessDenied,
            }
            .into());
        }
        self.inner.create(key, bytes, retention).await
    }
}
#[tokio::test]
async fn recovery_authorization_refuses_each_missing_gate_without_mutation() {
    for fault in [
        "drain",
        "revoke",
        "head",
        "inventory",
        "generation",
        "snapshot",
        "baseline",
        "image",
        "retention",
        "lease",
        "write",
        "readback",
        "missing",
        "corrupt",
    ] {
        let old = original(Shape::Changed, Fault::LostBefore, StorageFault::None).await;
        let inventory = complete(&old.state).await;
        let observation = observe(&old, &inventory).await;
        let (desired, _) = basis(&old).await;
        let ownership = grants(&inventory);
        let prerequisites = prereqs(&desired);
        let mut projection = contract(&inventory, &desired);
        let candidate = observation
            .prepare_resume(planning(
                &inventory,
                &desired,
                &ownership,
                &prerequisites,
                &projection,
            ))
            .unwrap();
        let accepted = accept_recovery(candidate, &mut projection);
        let mut intent = recovery_intent(&old, &observation, accepted.plan(), &desired);
        if fault == "generation" {
            intent.generation = old.intent.generation;
        }
        if fault == "snapshot" {
            intent.recovery.as_mut().unwrap().snapshot = d("wrong-snapshot");
        }
        if fault == "baseline" {
            intent.recovery.as_mut().unwrap().first_adoption = d("wrong-baseline");
        }
        let owner = c::Owner {
            job_uid: n("recovery-job"),
            run: d("recovery-run"),
        };
        let controls = authorize_controls(&old, &intent, &owner, &observation);
        {
            let mut control = controls.observation.lock().unwrap();
            let p = control.barrier.predecessor.as_mut().unwrap();
            match fault {
                "drain" => p.accepted_requests_drained = false,
                "revoke" => p.revoked_and_stopped = false,
                "head" => p.journal_head = d("wrong-head"),
                "inventory" => p.observed_inventory = d("wrong-inventory"),
                _ => (),
            }
        }
        let clock = c::tests::Clock(AtomicU64::new(1000));
        let target = c::tests::certificate(old.target.registration.clone());
        let cert = j::tests::certificate();
        let guard = j::SecretGuard::default();
        let active = old
            .coordinator
            .row
            .lock()
            .unwrap()
            .1
            .active
            .clone()
            .unwrap();
        let proof = c::inspect_recovery(&old.store, &cert, &active)
            .await
            .unwrap();
        let session = c::ExecutionSession::acquire(
            &old.coordinator,
            &controls,
            &clock,
            &target,
            c::ExecutionRequest {
                owner,
                operation: c::Operation::Resume,
                intent,
                plan: accepted.plan().report().plan_hash.clone().unwrap(),
            },
            Some(&proof),
        )
        .await;
        if ["drain", "revoke", "head", "generation"].contains(&fault) {
            assert!(session.is_err(), "{fault}");
            continue;
        }
        let session = session.unwrap();
        if fault == "lease" {
            clock.0.store(100_000, Ordering::SeqCst);
        }
        let mut retained = image();
        if fault == "image" {
            retained.image = d("unavailable-image");
        }
        if fault == "retention" {
            retained.retained_until_unix = 1;
        }
        let store = BrokenStore {
            inner: &old.store,
            mode: fault,
        };
        assert!(
            observation
                .record(
                    &accepted,
                    &session,
                    &store,
                    &cert,
                    &retained,
                    &retention(),
                    &guard
                )
                .await
                .is_err(),
            "{fault}"
        );
        assert_eq!(old.old_calls, 1, "record never dispatched a second request");
    }
}
#[tokio::test]
async fn candidate_needs_current_bound_reports_approval_and_secret_free_artifacts() {
    let old = original(Shape::Changed, Fault::None, StorageFault::None).await;
    let inventory = complete(&old.state).await;
    let observation = observe(&old, &inventory).await;
    let (desired, _) = basis(&old).await;
    let ownership = grants(&inventory);
    let prerequisites = prereqs(&desired);
    let mut projection = contract(&inventory, &desired);
    let candidate = observation
        .restore_draft(
            planning(
                &inventory,
                &desired,
                &ownership,
                &prerequisites,
                &projection,
            ),
            RestoreOrigin::RunBefore,
            &j::SecretGuard::default(),
        )
        .unwrap()
        .prepare(
            &observation,
            planning(
                &inventory,
                &desired,
                &ownership,
                &prerequisites,
                &projection,
            ),
        )
        .unwrap();
    let reports = synthetic_reports(candidate.candidate(), &mut projection);
    let slices = reports.iter().map(Vec::as_slice).collect::<Vec<_>>();
    assert!(candidate.accept(&projection, &slices, None, 0).is_err());
    assert!(RetainedRecoveryImage::from_registry(&n("unregistered-image")).is_err());
    let encoded = serde_json::to_string(observation.report()).unwrap();
    assert!(!encoded.contains("manual description"));
    assert!(!encoded.contains("routing.note"));
}

#[path = "recovery_fixture_tests.rs"]
mod fixture;

async fn recover_and_execute(
    old: Original,
    restore: bool,
    verify: Verify,
    lost: bool,
) -> (ac::ApplyReport, usize, Fixture, j::RecoveredRun) {
    recover_and_execute_fault(old, restore, verify, lost, Fault::None).await
}
#[tokio::test]
async fn restoration_unknown_request_and_post_commit_error_remain_recoverable_not_successful() {
    for (fault, outcome) in [
        (Fault::LostBefore, ac::ApplyOutcome::CommitUnknown),
        (Fault::PostCommit512, ac::ApplyOutcome::AppliedUnverified),
    ] {
        let old = original(Shape::Changed, Fault::None, StorageFault::None).await;
        let (report, calls, _, recovered) =
            recover_and_execute_fault(old, true, Verify::CompleteSynthetic, false, fault).await;
        assert_eq!(calls, 1);
        assert_eq!(report.outcome, outcome);
        assert_ne!(report.exit_code, 0);
        assert!(recovered.snapshot.data.recovery.is_some());
        assert_eq!(
            recovered.unresolved.pending_requests.len(),
            usize::from(matches!(fault, Fault::LostBefore))
        );
    }
}
#[tokio::test]
async fn already_restored_values_make_zero_action_plan_but_missing_cases_still_refuse() {
    let mut old = original(Shape::Changed, Fault::None, StorageFault::None).await;
    old.state.contexts[0]["override"][OPTIONS] = v!(["A", "B"]);
    let inventory = complete(&old.state).await;
    let observation = observe(&old, &inventory).await;
    let (desired, _) = basis(&old).await;
    let ownership = grants(&inventory);
    let prerequisites = prereqs(&desired);
    let projection = contract(&inventory, &desired);
    let inputs = || {
        planning(
            &inventory,
            &desired,
            &ownership,
            &prerequisites,
            &projection,
        )
    };
    let candidate = observation
        .restore_draft(
            inputs(),
            RestoreOrigin::RunBefore,
            &j::SecretGuard::default(),
        )
        .unwrap()
        .prepare(&observation, inputs())
        .unwrap();
    assert!(candidate.candidate().actions().is_empty());
    assert!(candidate.accept(&projection, &[], None, 0).is_err());
}

#[tokio::test]
async fn prerequisite_only_progress_is_retained_and_remaining_context_is_freshly_planned() {
    let old = original(Shape::ThreeRequests, Fault::LostAt(3), StorageFault::None).await;
    let inventory = complete(&old.state).await;
    let observation = observe(&old, &inventory).await;
    assert_eq!(
        observation
            .report()
            .requests
            .iter()
            .map(|r| r.state)
            .collect::<Vec<_>>(),
        vec![
            RequestState::After,
            RequestState::After,
            RequestState::Before
        ]
    );
    let (desired, _) = basis(&old).await;
    let mut ownership = grants(&inventory);
    ownership.creations.push(CreationGrant {
        intent_id: n("remaining-create"),
        condition: condition("a"),
        attributes: BTreeSet::from([key(REQUIRED), key(OPTIONS)]),
        approver: n("owner"),
        review: n("remaining-context"),
    });
    let prerequisites = prereqs(&desired);
    let projection = contract(&inventory, &desired);
    let candidate = observation
        .prepare_resume(planning(
            &inventory,
            &desired,
            &ownership,
            &prerequisites,
            &projection,
        ))
        .unwrap();
    assert_eq!(candidate.retained_prerequisites().len(), 2);
    assert_eq!(candidate.candidate().actions().len(), 1);
    assert_eq!(
        candidate.candidate().actions()[0].kind(),
        &ActionKind::Create
    );
}

#[tokio::test]
async fn ordinary_apply_preparation_cannot_bypass_recovery_authorization() {
    let old = original(Shape::Changed, Fault::None, StorageFault::None).await;
    let (report, calls, _, _) = recover_and_execute_fault(
        old,
        true,
        Verify::CompleteSynthetic,
        false,
        Fault::CancelBefore,
    )
    .await;
    assert_eq!(report.outcome, ac::ApplyOutcome::Refused);
    assert_eq!(calls, 0);
}
