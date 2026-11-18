#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
use super::{
    super::coordination::tests::{d, n, owner, registration},
    *,
};

fn initial() -> serde_json::Value {
    serde_json::json!({"apiVersion":"coordination.k8s.io/v1","kind":"Lease","metadata":{
        "name":"rf-target","namespace":"rf-fixture","uid":"fixture-object-uid","resourceVersion":"10",
        "annotations":{STATE_KEY:String::from_utf8(encode(&CoordinationState::initial(&registration()).unwrap()).unwrap()).unwrap(),"unowned.note":"preserve this"},
        "labels":{"owner":"fixture"}},"spec":{"holderIdentity":null,"renewTime":null,"leaseDurationSeconds":60,"leaseTransitions":0,"unowned_future":["B","A","A"]}})
}
#[test]
fn exact_scoped_cas_preserves_metadata_and_uses_returned_resource_version() {
    let target = registration();
    let old = decode(&encode(&initial()).unwrap(), &target).unwrap();
    let mut next = old.state.clone();
    let stamp = IntentStamp {
        generation: 7,
        hash: d("intent"),
    };
    next.highest_intent = Some(stamp.clone());
    next.lease = Some(LeaseOwnership {
        owner: owner(),
        renewed_at_ms: 1000,
        expires_at_ms: 61000,
    });
    next.active = Some(ActiveRun {
        owner: owner(),
        operation: Operation::Apply,
        intent: stamp,
        plan: d("plan"),
        phase: RunPhase::Preparing,
        journal: None,
        request_in_flight: None,
        gate: None,
        membership: None,
        barrier: None,
    });
    let bytes = replacement(&old, &next, &target).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["metadata"]["resourceVersion"], "10");
    assert_eq!(value["metadata"]["uid"], "fixture-object-uid");
    assert_eq!(
        value["metadata"]["annotations"]["unowned.note"],
        "preserve this"
    );
    assert_eq!(
        value["spec"]["unowned_future"],
        serde_json::json!(["B", "A", "A"])
    );
    assert_eq!(value["spec"]["leaseTransitions"], 1);
    assert_eq!(value["spec"]["renewTime"], "1970-01-01T00:00:01Z");
    let observed = decode(&bytes, &target).unwrap();
    let mut released = next;
    released.lease = None;
    released.active.as_mut().unwrap().phase = RunPhase::Unresolved;
    let release: serde_json::Value =
        serde_json::from_slice(&replacement(&observed, &released, &target).unwrap()).unwrap();
    assert!(release["spec"]["holderIdentity"].is_null());
    assert!(release["metadata"]["annotations"][STATE_KEY]
        .as_str()
        .unwrap()
        .contains("unresolved"));
}
#[test]
fn wrong_uid_namespace_owner_gc_partial_and_malformed_objects_refuse() {
    for fault in 0..8 {
        let mut value = initial();
        match fault {
            0 => value["metadata"]["uid"] = serde_json::json!("replacement-object"),
            1 => value["metadata"]["namespace"] = serde_json::json!("other"),
            2 => {
                value["metadata"]["ownerReferences"] = serde_json::json!([{"uid":"ephemeral-job"}])
            }
            3 => value["metadata"]["deletionTimestamp"] = serde_json::json!("2026-11-18T00:00:00Z"),
            4 => value["spec"]["holderIdentity"] = serde_json::json!("other-job"),
            5 => value["spec"]["strategy"] = serde_json::json!("OldestEmulationVersion"),
            6 => value["metadata"]["annotations"][STATE_KEY] = serde_json::json!("{}"),
            7 => value["kind"] = serde_json::json!("LeaseList"),
            _ => unreachable!(),
        }
        assert!(
            decode(&encode(&value).unwrap(), &registration()).is_err(),
            "fault {fault}"
        );
    }
    assert!(decode(br#"{"kind":"Lease","kind":"Lease"}"#, &registration()).is_err());
}
#[test]
fn closed_registration_and_endpoint_guard_do_not_load_ambient_credentials() {
    assert!(KubernetesCoordinator::from_registered(
        &n("arbitrary"),
        HeaderValue::from_static("synthetic")
    )
    .is_err());
    for url in [
        "http://localhost:1234/",
        "https://example.invalid/",
        "http://127.0.0.1:1234/?override=1",
        "http://user@127.0.0.1:1234/",
    ] {
        assert!(KubernetesCoordinator::fixture(
            registration(),
            url.parse().unwrap(),
            n("fixture-owner")
        )
        .is_err());
    }
}

#[tokio::test]
#[ignore = "requires explicitly registered RF_COORDINATION_FIXTURE_ROOT; starts only its owned loopback wire fixture"]
async fn owned_loopback_adapter_cas_conflict_unknown_outcome_and_restart() {
    use std::{
        fs,
        io::{BufRead, BufReader},
        path::PathBuf,
        process::{Command, Stdio},
    };
    let root = PathBuf::from(
        std::env::var("RF_COORDINATION_FIXTURE_ROOT").expect("explicit fixture root"),
    );
    assert!(root.is_absolute());
    assert_eq!(root.canonicalize().unwrap(), root);
    let marker: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(marker["purpose"], "Step 9 isolated coordination checks");
    assert_eq!(marker["root"], root.to_str().unwrap());
    assert_eq!(marker["cleanup_scope"], root.to_str().unwrap());
    let owner_id = n(marker["owner"].as_str().unwrap());
    let child = (0..1000)
        .find_map(|i| {
            let path = root.join(format!("coordinator-wire-{i}"));
            fs::create_dir(&path).ok().map(|()| path)
        })
        .expect("owned child allocation");
    fs::write(child.join("lease.json"), encode(&initial()).unwrap()).unwrap();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("coordination_fixture.py");
    // This check crate owns the opt-in transport fixture; actual router tests refuse this path.
    assert!(script.is_file());
    struct ProcessGuard(std::process::Child);
    impl Drop for ProcessGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let start = || {
        let mut server = ProcessGuard(
            Command::new("python3")
                .arg(&script)
                .arg("--root")
                .arg(&root)
                .arg("--child")
                .arg(child.file_name().unwrap())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let output = server.0.stdout.take().unwrap();
        let (send, receive) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut line = String::new();
            BufReader::new(output).read_line(&mut line).unwrap();
            let _ = send.send(line);
        });
        let line = receive
            .recv_timeout(Duration::from_secs(5))
            .expect("fixture startup deadline");
        reader.join().unwrap();
        let port: u16 = line.trim().parse().unwrap();
        (server, format!("http://127.0.0.1:{port}/").parse().unwrap())
    };
    let (server, url) = start();
    let adapter = KubernetesCoordinator::fixture(registration(), url, owner_id.clone()).unwrap();
    let unverified = decode(&encode(&initial()).unwrap(), &registration()).unwrap();
    assert_eq!(
        adapter
            .compare_exchange(&unverified, &unverified.state)
            .await
            .err()
            .unwrap()
            .current_context()
            .code,
        CoordinationCode::PermissionDenied
    );
    assert!(!child.join("writes").exists());
    let observed = adapter.read().await.unwrap();
    let mut next = observed.state.clone();
    let stamp = IntentStamp {
        generation: 1,
        hash: d("generation-1"),
    };
    next.highest_intent = Some(stamp.clone());
    next.lease = Some(LeaseOwnership {
        owner: owner(),
        renewed_at_ms: 1000,
        expires_at_ms: 61000,
    });
    next.active = Some(ActiveRun {
        owner: owner(),
        operation: Operation::Apply,
        intent: stamp,
        plan: d("plan"),
        phase: RunPhase::Preparing,
        journal: None,
        request_in_flight: None,
        gate: None,
        membership: None,
        barrier: None,
    });
    let written = adapter.compare_exchange(&observed, &next).await.unwrap();
    assert_ne!(written.version, observed.version);
    assert_eq!(
        adapter
            .compare_exchange(&observed, &next)
            .await
            .err()
            .unwrap()
            .current_context()
            .code,
        CoordinationCode::Conflict
    );
    fs::write(child.join("mode"), "deny").unwrap();
    assert_eq!(
        adapter.read().await.err().unwrap().current_context().code,
        CoordinationCode::PermissionDenied
    );
    fs::write(child.join("mode"), "lost-ack").unwrap();
    let mut next = written.state.clone();
    next.lease = None;
    next.active.as_mut().unwrap().phase = RunPhase::Unresolved;
    assert_eq!(
        adapter
            .compare_exchange(&written, &next)
            .await
            .err()
            .unwrap()
            .current_context()
            .code,
        CoordinationCode::UnknownOutcome
    );
    fs::write(child.join("mode"), "normal").unwrap();
    let committed = adapter.read().await.unwrap();
    assert!(committed.state.lease.is_none());
    assert_eq!(committed.state.active.unwrap().phase, RunPhase::Unresolved);
    drop(adapter);
    drop(server);
    let (server, url) = start();
    let reader = KubernetesCoordinator::fixture(registration(), url, owner_id).unwrap();
    let after_restart = reader.read().await.unwrap().state;
    assert!(after_restart.lease.is_none());
    assert_eq!(after_restart.active.unwrap().owner, owner());
    assert_eq!(after_restart.highest_intent.unwrap().generation, 1);
    let writes = fs::read_to_string(child.join("writes")).unwrap();
    assert_eq!(writes, "2"); // no implicit retry after a committed PUT lost its response
    drop(reader);
    drop(server);
    assert_eq!(child.parent(), Some(root.as_path()));
    fs::remove_dir_all(child).unwrap();
}
