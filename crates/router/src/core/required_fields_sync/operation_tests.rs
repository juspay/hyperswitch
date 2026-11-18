//! Synthetic command-to-engine evidence, deliberately not a server/platform certificate.
use super::{
    super::super::super::{attestation, operation::*, operation_executor::*},
    *,
};

fn config(case: &Case) -> Configuration {
    Configuration {
        schema_version: 1,
        mode: Mode::Apply,
        target: case.target.registration.target.clone(),
        journal_store: n("synthetic-journal"),
        planning: Some(Planning {
            input_profile: "synthetic-profile.json".into(),
            projection_contract: case.projection.id.clone(),
            ownership: case.ownership.clone(),
            prerequisites: case.prerequisites.clone(),
            intent: case.intent.clone(),
            limits: PlanLimits::default(),
            browser_reports: vec![],
            approval: Some(case.approval.clone()),
        }),
    }
}
fn invocation(case: &Case) -> Invocation {
    Invocation {
        kind: Kind::Apply,
        target: case.target.registration.target.target_id.clone(),
        source_run: None,
        generation: Some(1),
        plan_hash: Some(case.plan_hash.clone()),
    }
}
fn identity(case: &Case) -> ExecutionIdentity {
    ExecutionIdentity {
        image: case.intent.executor_image.clone(),
        executable: d("synthetic-executable"),
        build: case.intent.build.clone(),
        immutable_profile: d("synthetic-profile"),
        feature_keys: vec![n("REQUIRED_FIELDS_SYNC"), n("V1")],
    }
}
#[test]
fn executable_observation_hashes_the_actual_process_image() {
    use sha2::{Digest as _, Sha256};
    let observed = ObservedExecutable::capture(vec![n("REQUIRED_FIELDS_SYNC"), n("V1")]).unwrap();
    let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let expected: Digest = format!("sha256:{:x}", Sha256::digest(bytes))
        .try_into()
        .unwrap();
    assert_eq!(observed.digest, expected);
}
async fn execute(
    shape: Shape,
    fault: Fault,
    verification: Verify,
    alter: &str,
) -> (ac::ApplyResult<ResultBody>, usize, usize) {
    let case = case(shape).await;
    let store = Store::new(StorageFault::None);
    let cert = j::tests::certificate();
    let guard = j::SecretGuard::default();
    let target = c::tests::certificate(case.target.registration.clone());
    let coordinator = c::tests::Memory::new(&case.target.registration);
    let clock = c::tests::Clock(AtomicU64::new(1000));
    let owner = c::tests::owner();
    let controls = c::tests::Controls::new(&case.target.registration, &case.intent, &owner, 1000);
    let cancel = Cancellation::default();
    let server = Server {
        state: Mutex::new(case.fixture.clone()),
        store: &store,
        coordinator: &coordinator,
        controls: &controls,
        cancel: &cancel,
        fault,
        calls: Mutex::new(vec![]),
        reads: AtomicUsize::new(0),
    };
    let verifier = Verifier {
        mode: verification,
        clock: &clock,
        controls: &controls,
    };
    let runtime = Executor {
        target: &target,
        mutation: &case.target,
        coordinator: &coordinator,
        controls: &controls,
        clock: &clock,
        journal: &store,
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
    let mut config = config(&case);
    let mut invocation = invocation(&case);
    let mut identity = identity(&case);
    let image = recovery_tests::image();
    let mut reports = case.reports.clone();
    let observed = ObservedExecutable {
        digest: d("synthetic-executable"),
        features: vec![n("REQUIRED_FIELDS_SYNC"), n("V1")],
    };
    match alter {
        "disabled" => config.mode = Mode::Disabled,
        "plan-mode" => config.mode = Mode::Plan,
        "target" => config.target.instance_generation += 1,
        "intent" => invocation.generation = Some(2),
        "plan-hash" => invocation.plan_hash = Some(d("wrong-plan")),
        "image" => identity.image = d("wrong-image"),
        "executable" => identity.executable = d("wrong-executable"),
        "profile" => identity.immutable_profile = d("wrong-profile"),
        "features" => identity.feature_keys.push(n("V2")),
        "settings" => config.planning.as_mut().unwrap().intent.settings = d("wrong-settings"),
        "source" => {
            config.planning.as_mut().unwrap().intent.source = "9".repeat(40).try_into().unwrap()
        }
        "reports" => reports.clear(),
        "approval" => config.planning.as_mut().unwrap().approval = None,
        "controls" => controls.unavailable.store(true, Ordering::SeqCst),
        "none" => (),
        _ => panic!("unknown fault"),
    }
    let result = runtime
        .execute(
            &invocation,
            &config,
            Inputs {
                desired: &case.desired,
                bundle: j::RestrictedJson::parse(
                    &serde_json::to_vec(&case.bundle).unwrap(),
                    &guard,
                )
                .unwrap(),
                profile_hash: d("synthetic-profile"),
                observed: &observed,
                projection: &case.projection,
                browser_reports: reports,
                identity: &identity,
                retained_image: &image,
                retention: recovery_tests::retention(),
            },
        )
        .await;
    let calls = server.calls.lock().unwrap().len();
    let reads = server.reads.load(Ordering::SeqCst);
    (result, calls, reads)
}

#[tokio::test]
async fn command_executor_keeps_every_engine_outcome_and_never_invents_verification() {
    for (shape, fault, verification, outcome, exit) in [
        (
            Shape::Unchanged,
            Fault::None,
            Verify::CompleteSynthetic,
            ac::ApplyOutcome::NoChangeVerified,
            0,
        ),
        (
            Shape::Unchanged,
            Fault::None,
            Verify::Missing,
            ac::ApplyOutcome::NoChangeUnverified,
            6,
        ),
        (
            Shape::Changed,
            Fault::None,
            Verify::CompleteSynthetic,
            ac::ApplyOutcome::AppliedVerified,
            0,
        ),
        (
            Shape::Changed,
            Fault::None,
            Verify::Missing,
            ac::ApplyOutcome::AppliedUnverified,
            6,
        ),
        (
            Shape::Changed,
            Fault::Reject(1),
            Verify::Missing,
            ac::ApplyOutcome::RejectedNoChange,
            7,
        ),
        (
            Shape::ThreeRequests,
            Fault::Reject(2),
            Verify::Missing,
            ac::ApplyOutcome::PartialApplied,
            4,
        ),
        (
            Shape::Changed,
            Fault::ReadAfter,
            Verify::Missing,
            ac::ApplyOutcome::CommitUnknown,
            5,
        ),
    ] {
        let (result, calls, _) = execute(shape, fault, verification, "none").await;
        let body = result.unwrap();
        assert_eq!(body.exit_code(), exit, "{shape:?} {fault:?}");
        let ResultBody::Execution { report } = &body else {
            panic!("execution report expected")
        };
        assert_eq!(report.outcome, outcome);
        assert_eq!(report.dispatched, calls);
        let metrics = Metrics::of(Kind::Apply, &body, 12);
        assert_eq!(metrics.requests_dispatched, calls);
        if exit != 0 {
            assert!(metrics.verification.iter().all(|(_, v)| *v == "pending"));
        }
        let public = serde_json::to_string(&body).unwrap();
        assert!(!public.contains("routing.note"));
        assert!(!public.contains("Bearer"));
    }
}
#[tokio::test]
async fn command_executor_refuses_unbound_inputs_before_dispatch() {
    for fault in [
        "disabled",
        "plan-mode",
        "target",
        "intent",
        "plan-hash",
        "image",
        "executable",
        "profile",
        "features",
        "settings",
        "source",
        "reports",
        "approval",
        "controls",
    ] {
        let (result, calls, _) = execute(
            Shape::Changed,
            Fault::None,
            Verify::CompleteSynthetic,
            fault,
        )
        .await;
        if let Ok(ResultBody::Execution { report }) = result {
            assert_eq!(report.outcome, ac::ApplyOutcome::Refused, "{fault}");
            assert_eq!(report.leases_acquired, 1);
            assert!(report.exit_code == 2 || report.exit_code == 3);
        } else {
            assert!(result.is_err(), "{fault}");
        }
        assert_eq!(calls, 0, "{fault}");
    }
}
#[tokio::test]
async fn strict_operation_config_defaults_disabled_and_rejects_unknown_fields() {
    let case = case(Shape::Changed).await;
    let config = config(&case);
    let mut encoded = serde_json::to_value(config).unwrap();
    encoded.as_object_mut().unwrap().remove("mode");
    let decoded: Configuration = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(decoded.mode, Mode::Disabled);
    assert!(invocation(&case).validate(&decoded).is_err());
    encoded["writer_token"] = v!("synthetic-private-value");
    assert!(serde_json::from_value::<Configuration>(encoded).is_err());
    for keys in [
        "V1",
        "REQUIRED_FIELDS_SYNC,V1,V2",
        "V1,REQUIRED_FIELDS_SYNC",
        "REQUIRED_FIELDS_SYNC,V1,V1",
    ] {
        assert!(attestation::features(keys).is_err());
    }
}

#[tokio::test]
#[ignore = "requires explicit RF_OPERATION_FIXTURE_ROOT ownership registration"]
async fn operation_file_contract_and_exported_binary_inputs() {
    use std::{
        io::Write as _,
        path::{Path, PathBuf},
    };

    use super::super::super::super::operation as op;
    use crate::configs::cli::{OperationArgs, RequiredFieldsCommand};
    let root = PathBuf::from(std::env::var("RF_OPERATION_FIXTURE_ROOT").unwrap());
    let owner: Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(owner["purpose"], "Step 13 isolated command checks");
    assert_eq!(owner["root"], v!(root));
    assert_eq!(owner["cleanup_scope"], v!(root));
    assert!(owner["hosted_target"].is_null());
    let root = root.join("operation-inputs");
    std::fs::create_dir(&root).unwrap();
    let case = case(Shape::Changed).await;
    let apply = config(&case);
    let mut restore = apply.clone();
    restore.planning.as_mut().unwrap().intent.recovery = Some(a::RecoveryIntent {
        from_run: d("source-run"),
        snapshot: d("source-snapshot"),
        first_adoption: d("baseline"),
        reader_builds: d("readers"),
    });
    let mut status = apply.clone();
    status.mode = Mode::Disabled;
    status.planning = None;
    for (name, config) in [
        ("apply", &apply),
        ("restore", &restore),
        ("status", &status),
    ] {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(format!("{name}.json")))
            .unwrap();
        file.write_all(&serde_json::to_vec(config).unwrap())
            .unwrap();
    }
    struct Denied;
    impl op::Runtime for Denied {
        async fn run(
            &self,
            _: &Invocation,
            _: &Configuration,
            _: &Path,
        ) -> super::super::super::super::command::CommandResult<ResultBody> {
            use super::super::super::super::command::*;
            Err(fail(CommandCode::CertificationPending, Phase::Target)
                .attach_printable("synthetic-sensitive-never-log"))
        }
    }
    let args = OperationArgs {
        target: apply.target.target_id.as_str().to_owned(),
        command_config: root.join("apply.json"),
        output: root.join("refusal.json"),
    };
    let command = RequiredFieldsCommand::Apply {
        args: args.clone(),
        intent: 1,
        plan_hash: serde_json::from_value::<String>(v!(case.plan_hash)).unwrap(),
    };
    let (report, exit) = run(command.clone(), &Denied).await.unwrap();
    assert_eq!(exit, 2);
    assert_eq!(report["code"], "certification_pending");
    let bytes = std::fs::read(&args.output).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-sensitive-never-log"));
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), report);
    assert!(
        run(command, &Denied).await.is_err(),
        "must not replace existing report"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(&args.output)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
