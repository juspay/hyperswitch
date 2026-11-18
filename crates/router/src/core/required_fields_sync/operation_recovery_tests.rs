use super::{
    super::super::super::super::{operation::*, operation_executor::*},
    *,
};

#[tokio::test]
async fn command_recovery_status_and_inverse_planning_are_read_only_and_mutation_reuses_gates() {
    for kind in [
        Kind::Status,
        Kind::RestorePlan,
        Kind::RestoreApply,
        Kind::Resume,
    ] {
        for deny_exclusion in [false, true] {
            let old = original(
                Shape::Changed,
                if kind == Kind::Status && deny_exclusion {
                    Fault::ReadAfter
                } else if kind == Kind::Resume {
                    Fault::LostBefore
                } else {
                    Fault::None
                },
                StorageFault::None,
            )
            .await;
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
            let candidate = if kind == Kind::Resume {
                observation.prepare_resume(input()).unwrap()
            } else {
                observation
                    .restore_draft(
                        input(),
                        RestoreOrigin::RunBefore,
                        &j::SecretGuard::default(),
                    )
                    .unwrap()
                    .prepare(&observation, input())
                    .unwrap()
            };
            let reports = synthetic_reports(candidate.candidate(), &mut projection);
            let slices: Vec<_> = reports.iter().map(Vec::as_slice).collect();
            let evidence = examine_cases(candidate.candidate(), &projection, &slices).unwrap();
            let approved = approval(candidate.candidate(), &evidence);
            let accepted = candidate
                .accept(&projection, &slices, Some(&approved), 0)
                .unwrap();
            let intent = recovery_intent(&old, &observation, accepted.plan(), &desired);
            let owner = c::Owner {
                job_uid: n("successor-job"),
                run: d("successor-run"),
            };
            let controls = authorize_controls(&old, &intent, &owner, &observation);
            if deny_exclusion {
                controls
                    .observation
                    .lock()
                    .unwrap()
                    .barrier
                    .predecessor
                    .as_mut()
                    .unwrap()
                    .accepted_requests_drained = false;
                if kind == Kind::RestorePlan {
                    controls.unavailable.store(true, Ordering::SeqCst);
                }
            }
            let target = c::tests::certificate(old.target.registration.clone());
            let clock = c::tests::Clock(AtomicU64::new(1000));
            let cert = j::tests::certificate();
            let guard = j::SecretGuard::default();
            let cancel = Cancellation::default();
            let verifier = Verifier {
                mode: Verify::Missing,
                clock: &clock,
                controls: &controls,
            };
            let server = Server {
                state: Mutex::new(old.state.clone()),
                store: &old.store,
                coordinator: &old.coordinator,
                controls: &controls,
                cancel: &cancel,
                fault: Fault::None,
                calls: Mutex::new(vec![]),
                reads: AtomicUsize::new(0),
            };
            let executor = Executor {
                target: &target,
                mutation: &old.target,
                coordinator: &old.coordinator,
                controls: &controls,
                clock: &clock,
                journal: &old.store,
                certificate: &cert,
                reader: &server,
                writer: &server,
                verifier: &verifier,
                guard: &guard,
                cancellation: &cancel,
                owner,
                limits: apply::ApplyLimits {
                    read_attempts: 1,
                    read_ms: 100,
                    verification_ms: 20,
                },
            };
            let invocation = Invocation {
                kind,
                target: old.target.registration.target.target_id.clone(),
                source_run: Some(old.anchor.locator.run().clone()),
                generation: if kind == Kind::Status { None } else { Some(2) },
                plan_hash: if kind.mutates() {
                    accepted.plan().report().plan_hash.clone()
                } else {
                    None
                },
            };
            let config = Configuration {
                schema_version: 1,
                mode: if kind.mutates() {
                    Mode::Apply
                } else if kind == Kind::Status {
                    Mode::Disabled
                } else {
                    Mode::Plan
                },
                target: old.target.registration.target.clone(),
                journal_store: n("synthetic-store"),
                planning: if kind == Kind::Status {
                    None
                } else {
                    Some(Planning {
                        input_profile: "profile.json".into(),
                        projection_contract: projection.id.clone(),
                        ownership,
                        prerequisites,
                        intent: intent.clone(),
                        limits: PlanLimits::default(),
                        browser_reports: vec![],
                        approval: Some(approved),
                    })
                },
            };
            let identity = ExecutionIdentity {
                image: intent.executor_image.clone(),
                executable: d("exe"),
                build: intent.build.clone(),
                immutable_profile: d("profile"),
                feature_keys: intent.feature_keys.clone(),
            };
            let observed = ObservedExecutable {
                digest: d("exe"),
                features: intent.feature_keys.clone(),
            };
            let image = image();
            let before_coordinator = old.coordinator.row.lock().unwrap().clone();
            let before_events = old.store.events.lock().unwrap().len();
            let result = if kind == Kind::Status {
                executor.status(&invocation, &config).await
            } else {
                executor
                    .execute(
                        &invocation,
                        &config,
                        Inputs {
                            desired: &desired,
                            bundle: j::RestrictedJson::parse(
                                &serde_json::to_vec(&bundle).unwrap(),
                                &guard,
                            )
                            .unwrap(),
                            profile_hash: d("profile"),
                            observed: &observed,
                            projection: &projection,
                            browser_reports: reports,
                            identity: &identity,
                            retained_image: &image,
                            retention: retention(),
                        },
                    )
                    .await
            };
            let calls = server.calls.lock().unwrap().len();
            if !kind.mutates() {
                let body = result.unwrap();
                assert_eq!(calls, 0);
                assert_eq!(before_events, old.store.events.lock().unwrap().len());
                assert_eq!(
                    serde_json::to_value(&before_coordinator).unwrap(),
                    serde_json::to_value(&*old.coordinator.row.lock().unwrap()).unwrap()
                );
                if kind == Kind::Status {
                    assert_eq!(body.exit_code(), if deny_exclusion { 5 } else { 3 });
                    assert_eq!(server.reads.load(Ordering::SeqCst), 0);
                } else {
                    assert_eq!(body.exit_code(), 0);
                    assert!(matches!(body, ResultBody::RestorePlan { .. }));
                }
            } else if deny_exclusion {
                assert!(result.is_err());
                assert_eq!(calls, 0);
            } else {
                let body = result.unwrap();
                assert_eq!(calls, 1, "{kind:?}");
                assert_eq!(
                    body.exit_code(),
                    6,
                    "missing verification must remain unverified"
                );
            }
        }
    }
}
