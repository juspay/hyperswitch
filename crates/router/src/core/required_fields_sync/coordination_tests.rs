//! Independent synthetic protocol cases. No test result certifies an actual cluster/barrier.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
use std::{collections::BTreeMap, sync::Mutex};

use super::{
    super::{authority, types::TargetIdentity},
    *,
};

pub(crate) fn n(value: &str) -> Name {
    value.to_owned().try_into().unwrap()
}
pub(crate) fn d(value: &str) -> Digest {
    hash(&value).unwrap()
}
pub(crate) fn registration() -> TargetRegistration {
    TargetRegistration {
        schema_version: 1,
        target: TargetIdentity {
            target_id: n("synthetic-target"),
            organisation: n("synthetic-org"),
            workspace: n("synthetic-workspace"),
            instance_generation: 1,
        },
        backing_instance: d("physical-backend"),
        endpoint_identity: d("endpoint"),
        coordinator: CoordinatorIdentity {
            cluster: d("coordinator-cluster"),
            namespace: n("rf-fixture"),
            object: n("rf-target"),
            uid: n("fixture-object-uid"),
        },
        authority: n("main"),
        issuer: n("deployment-controller"),
        server_contract: d("uncertified-server"),
        journal_contract: d("synthetic-journal"),
        controls_contract: d("synthetic-controls"),
        readers: BTreeMap::from([
            (
                n("main"),
                ReaderRegistration {
                    region: n("region-a"),
                    cluster: d("cluster-a"),
                    namespace: n("payments"),
                    workload: n("main"),
                    uid: n("main-uid"),
                    kind: WorkloadKind::Rollout,
                    traffic_resources: vec![
                        n("stable-service"),
                        n("canary-service"),
                        n("mesh-route"),
                        n("endpoint-slices"),
                    ],
                },
            ),
            (
                n("custom"),
                ReaderRegistration {
                    region: n("region-b"),
                    cluster: d("cluster-b"),
                    namespace: n("custom"),
                    workload: n("custom"),
                    uid: n("custom-uid"),
                    kind: WorkloadKind::Deployment,
                    traffic_resources: vec![n("service"), n("ingress"), n("endpoint-slices")],
                },
            ),
        ]),
    }
}
pub(crate) fn intent() -> ReleaseIntent {
    ReleaseIntent {
        schema_version: 1,
        recovery: None,
        target: registration().target,
        issuer: n("deployment-controller"),
        authority: n("main"),
        generation: 7,
        source: "4".repeat(40).try_into().unwrap(),
        application_uid: n("application-uid"),
        git_revision: "5".repeat(40).try_into().unwrap(),
        executor_image: d("image-new"),
        build: d("build"),
        feature_keys: vec![n("REQUIRED_FIELDS_SYNC"), n("V1")],
        input_manifest: d("inputs"),
        bundle: d("bundle"),
        policy: d("policy"),
        catalog: d("catalog"),
        settings: d("settings"),
        release: d("release"),
        readers: BTreeMap::from([
            (
                n("main"),
                ReaderBuild {
                    image: d("image-new"),
                    build: d("build"),
                    inputs: d("inputs"),
                    compatibility: d("both-state-sdk-main"),
                },
            ),
            (
                n("custom"),
                ReaderBuild {
                    image: d("image-old-supported"),
                    build: d("build-old"),
                    inputs: d("inputs-old"),
                    compatibility: d("both-state-sdk-custom"),
                },
            ),
        ]),
    }
}
pub(crate) fn owner() -> Owner {
    Owner {
        job_uid: n("job-1-uid"),
        run: d("run-1"),
    }
}
pub(crate) fn observation(
    target: &TargetRegistration,
    intent: &ReleaseIntent,
    owner: &Owner,
    now: u64,
) -> SafetyObservation {
    let workloads: BTreeMap<_, _> = target
        .readers
        .iter()
        .map(|(id, registered)| {
            let build = &intent.readers[id];
            (
                id.clone(),
                WorkloadObservation {
                    registration: registered.clone(),
                    spec_generation: 3,
                    observed_generation: 3,
                    desired_replicas: 2,
                    updated_replicas: 2,
                    ready_replicas: 2,
                    old_replicas: 0,
                    paused: false,
                    degraded: false,
                    progressing: false,
                    promoted: true,
                    old_connections_drained: true,
                    controller_resource_version: n("workload-rv-3"),
                    traffic_resource_versions: registered
                        .traffic_resources
                        .iter()
                        .cloned()
                        .map(|resource| (resource, n("route-rv-9")))
                        .collect(),
                    compatibility: build.compatibility.clone(),
                    pods: (0..2)
                        .map(|i| PodObservation {
                            uid: n(&format!("{}-pod-{i}", id.as_str())),
                            image: build.image.clone(),
                            build: build.build.clone(),
                            inputs: build.inputs.clone(),
                            ready: true,
                        })
                        .collect(),
                    traffic_pod_uids: (0..2)
                        .map(|i| n(&format!("{}-pod-{i}", id.as_str())))
                        .collect(),
                },
            )
        })
        .collect();
    SafetyObservation {
        target: target.target.clone(),
        coordinator: target.coordinator.clone(),
        controls_contract: target.controls_contract.clone(),
        intent: intent.clone(),
        intent_gate: d("held-deployment-gate"),
        membership: hash(&workloads).unwrap(),
        observed_at_ms: now,
        valid_until_ms: now + 120_000,
        complete_reader_discovery: true,
        deployment_gate_held: true,
        workloads,
        barrier: BarrierObservation {
            id: d("enforced-writer-barrier"),
            owner: owner.job_uid.clone(),
            run: owner.run.clone(),
            scope: target.scope_hash().unwrap(),
            enforcement: d("synthetic-enforcement"),
            covered_paths: required_writer_paths().into_iter().collect(),
            enforced: true,
            privileged_edit_detected: false,
            predecessor: None,
        },
    }
}
pub(crate) struct Clock(pub AtomicU64);
impl CoordinationClock for Clock {
    fn now_ms(&self) -> CoordinationResult<u64> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
pub(crate) struct Controls {
    pub intent: Mutex<ReleaseIntent>,
    pub observation: Mutex<SafetyObservation>,
    pub unavailable: AtomicBool,
}
impl Controls {
    pub(crate) fn new(
        target: &TargetRegistration,
        intent: &ReleaseIntent,
        owner: &Owner,
        now: u64,
    ) -> Self {
        Self {
            intent: Mutex::new(intent.clone()),
            observation: Mutex::new(observation(target, intent, owner, now)),
            unavailable: AtomicBool::new(false),
        }
    }
}
impl SafetyControls for Controls {
    async fn current_intent(&self) -> CoordinationResult<ReleaseIntent> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(refuse(CoordinationCode::PermissionDenied));
        }
        Ok(self.intent.lock().unwrap().clone())
    }
    async fn establish(
        &self,
        _: &Owner,
        _: Option<&ActiveRun>,
    ) -> CoordinationResult<SafetyObservation> {
        self.observe(&owner()).await
    }
    async fn observe(&self, _: &Owner) -> CoordinationResult<SafetyObservation> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(refuse(CoordinationCode::BarrierRequired));
        }
        Ok(self.observation.lock().unwrap().clone())
    }
}
#[derive(Clone, Copy, Default)]
pub(crate) enum Fault {
    #[default]
    None,
    Denied,
    Conflict,
    LostAck,
    WrongReadback,
    Pause,
}
pub(crate) struct Memory {
    pub row: Mutex<(u64, CoordinationState)>,
    pub fault: Mutex<Fault>,
    pub writes: AtomicU64,
}
impl Memory {
    pub(crate) fn new(target: &TargetRegistration) -> Self {
        Self {
            row: Mutex::new((1, CoordinationState::initial(target).unwrap())),
            fault: Mutex::new(Fault::None),
            writes: AtomicU64::new(0),
        }
    }
    fn fault(&self, fault: Fault) {
        *self.fault.lock().unwrap() = fault;
    }
}
impl CoordinatorStore for Memory {
    async fn read(&self) -> CoordinationResult<ObservedState> {
        if matches!(*self.fault.lock().unwrap(), Fault::Denied) {
            return Err(refuse(CoordinationCode::PermissionDenied)
                .attach_printable("synthetic-private-diagnostic"));
        }
        let row = self.row.lock().unwrap();
        Ok(ObservedState {
            version: n(&row.0.to_string()),
            state: row.1.clone(),
            wire: None,
        })
    }
    async fn compare_exchange(
        &self,
        observed: &ObservedState,
        next: &CoordinationState,
    ) -> CoordinationResult<ObservedState> {
        let fault = *self.fault.lock().unwrap();
        if matches!(fault, Fault::Pause) {
            std::future::pending::<()>().await;
        }
        if matches!(fault, Fault::Conflict) {
            return Err(refuse(CoordinationCode::Conflict));
        }
        let mut row = self.row.lock().unwrap();
        if observed.version != n(&row.0.to_string()) {
            return Err(refuse(CoordinationCode::Conflict));
        }
        row.0 += 1;
        row.1 = next.clone();
        self.writes.fetch_add(1, Ordering::SeqCst);
        if matches!(fault, Fault::LostAck) {
            return Err(refuse(CoordinationCode::UnknownOutcome));
        }
        let mut returned = row.1.clone();
        if matches!(fault, Fault::WrongReadback) {
            returned.highest_intent = None;
        }
        Ok(ObservedState {
            version: n(&row.0.to_string()),
            state: returned,
            wire: None,
        })
    }
}
pub(crate) fn certificate(target: TargetRegistration) -> CertifiedExecutionTarget {
    CertifiedExecutionTarget {
        registration: target,
        synthetic: true,
    }
}
fn request() -> ExecutionRequest {
    ExecutionRequest {
        owner: owner(),
        operation: Operation::Apply,
        intent: intent(),
        plan: d("plan"),
    }
}
fn code<T>(result: CoordinationResult<T>) -> CoordinationCode {
    match result {
        Ok(_) => panic!("expected refusal"),
        Err(e) => e.current_context().code,
    }
}

#[test]
fn registry_refuses_ambiguous_backends_coordinators_and_unregistered_targets() {
    let target = registration();
    validate_registry(std::slice::from_ref(&target)).unwrap();
    let mut alias = target.clone();
    alias.target.target_id = n("other-region");
    alias.coordinator.cluster = d("regional-lock");
    assert_eq!(
        code(validate_registry(&[target.clone(), alias])),
        CoordinationCode::TargetMismatch
    );
    let mut alias = target.clone();
    alias.backing_instance = d("other-backend");
    alias.target.target_id = n("different");
    alias.coordinator.uid = n("recreated-object-uid");
    assert_eq!(
        code(validate_registry(&[target.clone(), alias])),
        CoordinationCode::TargetMismatch
    );
    let mut alias = target.clone();
    alias.backing_instance = d("other-backend");
    alias.target.target_id = n("different");
    assert_eq!(
        code(validate_registry(&[target.clone(), alias])),
        CoordinationCode::TargetMismatch
    );
    let mut changed = target.clone();
    changed.target.instance_generation += 1;
    assert_eq!(changed.scope_hash().unwrap(), target.scope_hash().unwrap());
    assert_eq!(
        code(CertifiedExecutionTarget::from_registry(&n("production"))),
        CoordinationCode::CertificationPending
    );
}
#[test]
fn rollback_requires_a_new_generation_not_a_newer_image_tag() {
    let target = registration();
    let old = intent();
    let mut next = old.clone();
    next.executor_image = d("older-image");
    next.readers.get_mut(&n("main")).unwrap().image = next.executor_image.clone();
    assert_eq!(
        code(validate_intent_advance(&old, &next, &target)),
        CoordinationCode::StaleIntent
    );
    next.generation += 1;
    validate_intent_advance(&old, &next, &target).unwrap();
    assert_eq!(
        code(validate_intent_advance(&next, &old, &target)),
        CoordinationCode::StaleIntent
    );
    next.feature_keys.push(n("V2"));
    assert_eq!(
        code(next.validate(&target)),
        CoordinationCode::NotAuthoritative
    );
    assert!(parse_contract::<ReleaseIntent>(br#"{"generation":1,"generation":2}"#).is_err());
    let mut value = serde_json::to_value(old).unwrap();
    value["force"] = serde_json::json!(true);
    assert!(parse_contract::<ReleaseIntent>(&encode(&value).unwrap()).is_err());
}
#[test]
fn complete_rollout_rejects_every_mixed_or_incomplete_reader_view() {
    let target = registration();
    let intent = intent();
    let owner = owner();
    let base = observation(&target, &intent, &owner, 1000);
    validate_safety(
        &base,
        &target,
        &intent,
        &owner.job_uid,
        &owner.run,
        1000,
        15000,
    )
    .unwrap();
    for fault in 0..17 {
        let mut view = base.clone();
        let w = view.workloads.get_mut(&n("main")).unwrap();
        match fault {
            0 => w.pods[1].image = d("old-image"),
            1 => w.pods[1].inputs = d("wrong-settings"),
            2 => w.observed_generation -= 1,
            3 => w.paused = true,
            4 => w.degraded = true,
            5 => w.progressing = true,
            6 => w.promoted = false,
            7 => w.old_connections_drained = false,
            8 => w.old_replicas = 1,
            9 => w.traffic_pod_uids.push(n("unknown-endpoint")),
            10 => w.pods.push(w.pods[0].clone()),
            11 => w.desired_replicas += 1,
            12 => w.compatibility = d("only-before-compatible"),
            13 => w.registration.kind = WorkloadKind::Deployment,
            14 => {
                w.traffic_resource_versions.remove(&n("canary-service"));
            }
            15 => view.complete_reader_discovery = false,
            16 => view.deployment_gate_held = false,
            _ => unreachable!(),
        }
        view.membership = hash(&view.workloads).unwrap();
        assert_eq!(
            code(validate_safety(
                &view,
                &target,
                &intent,
                &owner.job_uid,
                &owner.run,
                1000,
                15000
            )),
            CoordinationCode::RolloutIncomplete,
            "case {fault}"
        );
    }
    let mut view = base;
    view.workloads.remove(&n("custom"));
    assert!(validate_safety(
        &view,
        &target,
        &intent,
        &owner.job_uid,
        &owner.run,
        1000,
        15000
    )
    .is_err());
}
#[test]
fn lease_or_reread_never_substitutes_for_external_enforcement() {
    let target = registration();
    let intent = intent();
    let owner = owner();
    for fault in 0..5 {
        let mut view = observation(&target, &intent, &owner, 1000);
        match fault {
            0 => view.barrier.enforced = false,
            1 => {
                view.barrier.covered_paths.pop();
            }
            2 => view.barrier.privileged_edit_detected = true,
            3 => view.barrier.scope = d("different-backend"),
            4 => view.barrier.owner = n("other-job"),
            _ => unreachable!(),
        }
        assert_eq!(
            code(validate_safety(
                &view,
                &target,
                &intent,
                &owner.job_uid,
                &owner.run,
                1000,
                15000
            )),
            CoordinationCode::BarrierRequired
        );
    }
}
#[tokio::test]
async fn acquire_claims_once_and_owner_checked_release_retains_unresolved_work() {
    let target = certificate(registration());
    let store = Memory::new(&target.registration);
    let clock = Clock(AtomicU64::new(1000));
    let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
    let session = ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
        .await
        .unwrap();
    assert_eq!(
        code(ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None).await),
        CoordinationCode::RecoveryRequired
    );
    session.renew().await.unwrap();
    session.release_to_recovery().await.unwrap();
    let row = store.read().await.unwrap();
    assert!(row.state.lease.is_none());
    assert_eq!(row.state.active.unwrap().phase, RunPhase::Unresolved);
    clock.0.store(1_000_000, Ordering::SeqCst);
    assert_eq!(
        code(ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None).await),
        CoordinationCode::RecoveryRequired
    );
}
#[tokio::test]
async fn stale_job_or_unsupported_controls_refuse_before_claim() {
    let target = certificate(registration());
    let store = Memory::new(&target.registration);
    let clock = Clock(AtomicU64::new(1000));
    let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
    let mut stale = request();
    stale.intent.generation -= 1;
    assert_eq!(
        code(ExecutionSession::acquire(&store, &controls, &clock, &target, stale, None).await),
        CoordinationCode::NotAuthoritative
    );
    assert_eq!(
        code(
            ExecutionSession::acquire(
                &store,
                &UnavailableSafetyControls,
                &clock,
                &target,
                request(),
                None
            )
            .await
        ),
        CoordinationCode::CertificationPending
    );
    assert_eq!(store.writes.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn independently_polled_heartbeat_renews_while_other_work_is_waiting() {
    let target = certificate(registration());
    let store = Memory::new(&target.registration);
    let clock = Clock(AtomicU64::new(1000));
    let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
    let session = ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
        .await
        .unwrap();
    clock.0.store(11_000, Ordering::SeqCst);
    controls.observation.lock().unwrap().observed_at_ms = 11_000;
    tokio::select! {
        result = session.renew_until_stopped() => panic!("heartbeat ended: {result:?}"),
        () = async {
            tokio::time::sleep(Duration::from_millis(RENEW_MS + 500)).await;
            let row = store.read().await.unwrap();
            assert_eq!(row.state.lease.unwrap().expires_at_ms, 71_000);
            assert_eq!(row.state.active.unwrap().owner, owner());
            assert_eq!(store.writes.load(Ordering::SeqCst), 3);
            session.stop();
        } => {}
    }
    assert_eq!(code(session.renew().await), CoordinationCode::LeaseLost);
    assert!(store.read().await.unwrap().state.active.is_some());
}

#[tokio::test]
async fn fresh_control_response_cannot_resurrect_a_lease_expired_during_the_read() {
    struct SlowControls<'a> {
        controls: Controls,
        clock: &'a Clock,
    }
    impl SafetyControls for SlowControls<'_> {
        async fn current_intent(&self) -> CoordinationResult<ReleaseIntent> {
            self.controls.current_intent().await
        }
        async fn establish(
            &self,
            owner: &Owner,
            previous: Option<&ActiveRun>,
        ) -> CoordinationResult<SafetyObservation> {
            self.controls.establish(owner, previous).await
        }
        async fn observe(&self, owner: &Owner) -> CoordinationResult<SafetyObservation> {
            // Model process suspension during an awaited control call, with fresh evidence
            // on resumption but an expired coordinator lease.
            let mut view = self.controls.observe(owner).await?;
            self.clock.0.store(62_000, Ordering::SeqCst);
            view.observed_at_ms = 62_000;
            view.valid_until_ms = 182_000;
            Ok(view)
        }
    }
    let target = certificate(registration());
    let store = Memory::new(&target.registration);
    let clock = Clock(AtomicU64::new(1000));
    let controls = SlowControls {
        controls: Controls::new(&target.registration, &intent(), &owner(), 1000),
        clock: &clock,
    };
    let session = ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
        .await
        .unwrap();
    assert_eq!(code(session.renew().await), CoordinationCode::LeaseLost);
    assert_eq!(store.writes.load(Ordering::SeqCst), 2);
    assert!(store.read().await.unwrap().state.active.is_some());
}
#[tokio::test]
async fn barrier_failure_after_claim_leaves_a_durable_block() {
    let target = certificate(registration());
    let store = Memory::new(&target.registration);
    let clock = Clock(AtomicU64::new(1000));
    let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
    controls.observation.lock().unwrap().barrier.enforced = false;
    assert_eq!(
        code(ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None).await),
        CoordinationCode::BarrierRequired
    );
    assert_eq!(
        store.read().await.unwrap().state.active.unwrap().phase,
        RunPhase::Preparing
    );
}
#[tokio::test]
async fn suspension_renew_failure_wrong_owner_and_clock_reversal_stop_dispatch() {
    for failure in 0..6 {
        let target = certificate(registration());
        let store = Memory::new(&target.registration);
        let clock = Clock(AtomicU64::new(1000));
        let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
        let session =
            ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
                .await
                .unwrap();
        match failure {
            0 => clock.0.store(62_000, Ordering::SeqCst),
            1 => store.fault(Fault::Conflict),
            2 => store.fault(Fault::LostAck),
            3 => {
                let mut row = store.row.lock().unwrap();
                row.1.lease.as_mut().unwrap().owner.job_uid = n("successor");
                row.1.active.as_mut().unwrap().owner.job_uid = n("successor");
            }
            4 => clock.0.store(999, Ordering::SeqCst),
            5 => {
                controls
                    .observation
                    .lock()
                    .unwrap()
                    .barrier
                    .privileged_edit_detected = true
            }
            _ => unreachable!(),
        }
        assert!(session.renew().await.is_err());
        assert!(session
            .recheck_before_dispatch(&d("request"))
            .await
            .is_err());
        if failure == 3 {
            assert!(session.release_to_recovery().await.is_err());
            assert_eq!(
                store
                    .row
                    .lock()
                    .unwrap()
                    .1
                    .lease
                    .as_ref()
                    .unwrap()
                    .owner
                    .job_uid,
                n("successor")
            );
        }
    }
}
#[tokio::test]
async fn scaling_even_to_compatible_new_pods_invalidates_the_held_population() {
    let target = certificate(registration());
    let store = Memory::new(&target.registration);
    let clock = Clock(AtomicU64::new(1000));
    let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
    let session = ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
        .await
        .unwrap();
    {
        let mut view = controls.observation.lock().unwrap();
        let w = view.workloads.get_mut(&n("main")).unwrap();
        let mut pod = w.pods[0].clone();
        pod.uid = n("scaled-new-pod");
        w.traffic_pod_uids.push(pod.uid.clone());
        w.pods.push(pod);
        w.desired_replicas += 1;
        w.ready_replicas += 1;
        w.updated_replicas += 1;
        view.membership = hash(&view.workloads).unwrap();
    }
    assert_eq!(
        code(session.renew().await),
        CoordinationCode::MembershipChanged
    );
    assert!(session.running().is_err());
}
#[tokio::test]
async fn changed_release_stops_old_jobs_and_cancelled_cas_cannot_resume() {
    let target = certificate(registration());
    let store = Memory::new(&target.registration);
    let clock = Clock(AtomicU64::new(1000));
    let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
    let session = ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
        .await
        .unwrap();
    store.fault(Fault::Pause);
    assert!(
        tokio::time::timeout(Duration::from_millis(5), session.renew())
            .await
            .is_err()
    );
    store.fault(Fault::None);
    assert!(session.running().is_err());
    let target = certificate(registration());
    let store = Memory::new(&target.registration);
    let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
    let session = ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
        .await
        .unwrap();
    controls.observation.lock().unwrap().intent.generation += 1;
    assert_eq!(
        code(session.renew().await),
        CoordinationCode::NotAuthoritative
    );
}
#[tokio::test]
async fn lost_claim_ack_readback_corruption_and_private_errors_are_not_success() {
    for fault in [Fault::LostAck, Fault::WrongReadback, Fault::Denied] {
        let target = certificate(registration());
        let store = Memory::new(&target.registration);
        store.fault(fault);
        let clock = Clock(AtomicU64::new(1000));
        let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
        let error =
            match ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
                .await
            {
                Ok(_) => panic!("unexpected success"),
                Err(e) => e,
            };
        assert!(!format!("{error:?}").contains("synthetic-private-diagnostic"));
    }
}
#[tokio::test]
async fn recovery_requires_actual_old_request_exclusion_not_just_an_expired_lease() {
    for failure in 0..5 {
        let target = certificate(registration());
        let store = Memory::new(&target.registration);
        let clock = Clock(AtomicU64::new(1000));
        let controls = Controls::new(&target.registration, &intent(), &owner(), 1000);
        let first = ExecutionSession::acquire(&store, &controls, &clock, &target, request(), None)
            .await
            .unwrap();
        first.release_to_recovery().await.unwrap();
        let previous = store.row.lock().unwrap().1.active.clone().unwrap();
        // Exercise the independent reader API for a released pre-preparation claim. The
        // persisted journal case is covered by the planner/journal integration test.
        let proof = inspect_recovery(
            &journal::tests::MemoryStore::default(),
            &journal::tests::certificate(),
            &previous,
        )
        .await
        .unwrap();
        let successor = Owner {
            job_uid: n("job-2-uid"),
            run: d("run-2"),
        };
        let mut observed = observation(&target.registration, &intent(), &successor, 1000);
        observed.barrier.predecessor = Some(PreviousExecutorExclusion {
            run: previous.owner.run.clone(),
            owner: previous.owner.job_uid.clone(),
            revoked_and_stopped: true,
            accepted_requests_drained: true,
            direct_state_observed: true,
            journal_head: proof.head.clone(),
            observed_inventory: d("after-exclusion-state"),
            receipt: d("synthetic-closure"),
        });
        let exclusion = observed.barrier.predecessor.as_mut().unwrap();
        match failure {
            0 => exclusion.revoked_and_stopped = false,
            1 => exclusion.accepted_requests_drained = false,
            2 => exclusion.direct_state_observed = false,
            3 => exclusion.journal_head = d("stale-head"),
            4 => (),
            _ => unreachable!(),
        }
        *controls.observation.lock().unwrap() = observed;
        let next = ExecutionRequest {
            owner: successor,
            operation: Operation::Resume,
            intent: intent(),
            plan: d("recovery-plan"),
        };
        let result =
            ExecutionSession::acquire(&store, &controls, &clock, &target, next, Some(&proof)).await;
        if failure < 4 {
            assert_eq!(code(result), CoordinationCode::OldRequestNotExcluded);
        } else {
            result.unwrap();
            assert_eq!(store.row.lock().unwrap().1.predecessors.len(), 1);
        }
    }
}
#[test]
fn public_status_contains_only_opaque_run_target_references() {
    let report = encode(
        &CoordinationState::initial(&registration())
            .unwrap()
            .report(),
    )
    .unwrap();
    let text = String::from_utf8(report).unwrap();
    for private in [
        "synthetic-org",
        "synthetic-workspace",
        "job-1-uid",
        "region-a",
    ] {
        assert!(!text.contains(private));
    }
    assert!(text.contains("disabled"));
    assert_eq!(
        code(authority::parse_contract::<TargetRegistration>(b"{}")),
        CoordinationCode::InvalidInput
    );
}
