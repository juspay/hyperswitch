//! Synthetic fault tests only; neither these stores nor receipts certify hosted durability.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::{collections::BTreeMap, sync::Mutex};

use super::{super::journal_snapshot::SnapshotData, *};

fn name(value: &str) -> Name {
    value.to_owned().try_into().unwrap()
}
fn hash(value: &str) -> Digest {
    checksum(value.as_bytes()).unwrap()
}
pub(crate) fn certificate() -> CertifiedJournalStore {
    CertifiedJournalStore {
        id: hash("synthetic-only-store-contract"),
        synthetic: true,
    }
}
fn retention() -> Retention {
    Retention {
        created_at_unix: 1,
        retain_until_unix: MIN_RETENTION_SECONDS + 1,
        rollback_window_seconds: 86400,
        protected_hold: true,
    }
}
fn target() -> TargetIdentity {
    TargetIdentity {
        target_id: name("synthetic-journal"),
        organisation: name("fixture-org"),
        workspace: name("fixture-workspace"),
        instance_generation: 1,
    }
}
fn raw(value: &str) -> RestrictedJson {
    RestrictedJson::parse(value.as_bytes(), &SecretGuard::default()).unwrap()
}
fn request(indices: Vec<usize>) -> PreparedRequest {
    PreparedRequest::new(
        Method::Put,
        "/synthetic-only/bulk".to_owned(),
        BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]),
        raw(" {\"ordered\":[\"B\",\"A\",\"A\"],\"decimal\":1.2300} "),
        indices,
        hash("uncertified-synthetic-wire"),
    )
    .unwrap()
}
fn snapshot() -> PreparedSnapshot {
    PreparedSnapshot {
        data: SnapshotData {
            recovery: None,
            schema_version: 1,
            identity: RecoveryIdentity {
                target: target(),
                endpoint_identity: hash("fixture-endpoint"),
                coordinator: name("fixture-future-coordinator"),
                executor_image: hash("fixture-image"),
                retained_recovery_image: hash("fixture-recovery-image"),
            },
            retention: retention(),
            requests: vec![request(vec![0])],
            pending_create_ownership: vec![hash("fixture-pending-create")],
            contents: raw(
                r#"{"actions":[{"before":"{\"unowned\":[\"B\",\"A\"]}","after":"{\"unowned\":[\"B\",\"A\"],\"managed\":true}"}],"first_adoption":"fixture-original"}"#,
            ),
        },
    }
}
#[derive(Clone, Default)]
enum Fault {
    #[default]
    None,
    BeforeCreate,
    AfterCreate,
    Denied,
    Corrupt,
    WrongVersion,
    MissingHold,
}
#[derive(Default)]
pub(crate) struct MemoryStore {
    objects: Mutex<BTreeMap<String, StoredObject>>,
    fault: Mutex<Fault>,
}
impl MemoryStore {
    /// Export only synthetic fixture objects for a separate read-only recovery process.
    pub(crate) fn fixture_objects(&self) -> serde_json::Value {
        serde_json::Value::Object(self.objects.lock().unwrap().iter().map(|(key, object)|
            (key.clone(), serde_json::json!({"bytes":object.bytes,"version":object.version,"retention":object.retention}))
        ).collect())
    }
    fn fault(&self, value: Fault) {
        *self.fault.lock().unwrap() = value;
    }
}
impl JournalReader for MemoryStore {
    async fn read(&self, key: &str, max_bytes: usize) -> JournalResult<Option<StoredObject>> {
        let fault = self.fault.lock().unwrap().clone();
        if matches!(fault, Fault::Denied) {
            return Err(fail(JournalCode::AccessDenied));
        }
        let objects = self.objects.lock().unwrap();
        let Some(value) = objects.get(key) else {
            return Ok(None);
        };
        if value.bytes.len() > max_bytes {
            return Err(fail(JournalCode::LimitExceeded));
        }
        let mut bytes = value.bytes.clone();
        if matches!(fault, Fault::Corrupt) {
            bytes.pop();
        }
        let mut retention = value.retention.clone();
        if matches!(fault, Fault::MissingHold) {
            retention.protected_hold = false;
        }
        Ok(Some(StoredObject {
            bytes,
            version: if matches!(fault, Fault::WrongVersion) {
                name("wrong")
            } else {
                value.version.clone()
            },
            retention,
        }))
    }
}
impl ImmutableJournalStore for MemoryStore {
    async fn create(&self, key: &str, bytes: &[u8], retention: &Retention) -> JournalResult<Name> {
        let fault = self.fault.lock().unwrap().clone();
        if matches!(fault, Fault::BeforeCreate) {
            return Err(fail(JournalCode::AccessDenied));
        }
        let mut objects = self.objects.lock().unwrap();
        if objects.contains_key(key) {
            return Err(fail(JournalCode::Exists));
        }
        let version = name(&format!("fixture-version-{}", objects.len()));
        objects.insert(
            key.to_owned(),
            StoredObject {
                bytes: bytes.to_vec(),
                version: version.clone(),
                retention: retention.clone(),
            },
        );
        if matches!(fault, Fault::AfterCreate) {
            return Err(fail(JournalCode::UnknownStorageOutcome));
        }
        Ok(version)
    }
}
fn code<T>(result: JournalResult<T>) -> JournalCode {
    match result {
        Ok(_) => panic!("expected refusal"),
        Err(e) => e.current_context().code,
    }
}

#[test]
fn production_registry_is_closed_and_every_storage_guarantee_is_required() {
    assert_eq!(
        code(CertifiedJournalStore::from_registry(&name("anything"))),
        JournalCode::CertificationPending
    );
    for missing in 0..8 {
        let mut flags = [true; 8];
        flags[missing] = false;
        let requirements = StorageRequirements {
            conditional_create: flags[0],
            immutable_versions: flags[1],
            strong_complete_reads: flags[2],
            deny_overwrite_delete: flags[3],
            encrypted_restricted_access: flags[4],
            access_logged: flags[5],
            protected_retention: flags[6],
            independent_recovery_access: flags[7],
        };
        assert_eq!(code(requirements.validate()), JournalCode::MissingGuarantee);
    }
}
#[test]
fn retention_floor_rollback_window_and_holds_cannot_be_omitted() {
    let mut policy = retention();
    policy.retain_until_unix -= 1;
    assert_eq!(code(policy.validate()), JournalCode::MissingGuarantee);
    policy = retention();
    policy.rollback_window_seconds = MIN_RETENTION_SECONDS + 1;
    assert!(policy.validate().is_err());
    policy = retention();
    policy.protected_hold = false;
    assert!(policy.validate().is_err());
    policy = retention();
    policy.created_at_unix = u64::MAX;
    assert!(policy.validate().is_err());
}
#[test]
fn exact_request_bytes_order_and_explicit_values_survive_roundtrip() {
    let request = request(vec![0]);
    let restored: PreparedRequest = decode(&encode(&request).unwrap()).unwrap();
    assert_eq!(
        restored.body(),
        b" {\"ordered\":[\"B\",\"A\",\"A\"],\"decimal\":1.2300} "
    );
    assert_eq!(restored.hash().unwrap(), request.hash().unwrap());
    let mut changed = restored;
    changed.body = raw(r#"{"ordered":["A","B"],"decimal":1.23}"#);
    assert_ne!(changed.hash().unwrap(), request.hash().unwrap());
    for path in [
        "//host/path",
        "/../path",
        "/path?credential=value",
        "https://host/path",
        "/%2e/path",
    ] {
        let mut bad = request.clone();
        bad.path = path.to_owned();
        assert!(bad.validate().is_err());
    }
    let mut bad = request;
    bad.headers
        .insert("authorization".to_owned(), "fixture-value".to_owned());
    assert!(bad.validate().is_err());
}

#[tokio::test]
async fn new_reader_accepts_legacy_intents_but_not_new_event_kinds_under_legacy_version() {
    for legacy_intent in [true, false] {
        let store = MemoryStore::default();
        let certificate = certificate();
        let guard = SecretGuard::default();
        let (mut run, _) = RunJournal::persist_prepared(
            &store,
            &certificate,
            &guard,
            hash("legacy-fixture"),
            snapshot(),
            None,
        )
        .await
        .unwrap();
        if legacy_intent {
            run.record_intent(0).await.unwrap();
        } else {
            run.record_run_report(raw(r#"{"synthetic":"report"}"#))
                .await
                .unwrap();
        }
        let locator = run.unresolved().locator;
        // Deliberate isolated format fixture, not a journal API that can overwrite artifacts.
        let key = event_key(&locator, 0).unwrap();
        {
            let mut objects = store.objects.lock().unwrap();
            let object = objects.get_mut(&key).unwrap();
            let mut event: Event = decode(&object.bytes).unwrap();
            event.schema_version = 1;
            object.bytes = encode(&event).unwrap();
        }
        let result = recover(&store, &certificate, &locator, None).await;
        if legacy_intent {
            assert_eq!(result.unwrap().unresolved.pending_requests.len(), 1);
        } else {
            assert_eq!(code(result), JournalCode::InvalidTransition);
        }
    }
}
#[test]
fn credentials_are_rejected_without_leaking_diagnostics_or_rewriting_images() {
    let marker = "disposable-private-sentinel";
    let guard = SecretGuard::new(vec![marker.to_owned()]).unwrap();
    for bytes in [
        r#"{"password":"synthetic"}"#,
        r#"{"nested":{"api-key":"synthetic"}}"#,
        r#"{"authorization":"synthetic"}"#,
        r#"{"raw":"{\"access_token\":\"synthetic\"}"}"#,
        r#"{"x":"Bearer synthetic"}"#,
        r#"{"x":"disposable-private-sentinel"}"#,
    ] {
        let error = RestrictedJson::parse(bytes.as_bytes(), &guard).unwrap_err();
        assert_eq!(
            error.current_context().code,
            JournalCode::CredentialMaterial
        );
        assert!(!format!("{error:?}").contains(marker));
    }
    assert!(
        !format!("{:?}", raw(r#"{"tenant_rule":"sensitive-value"}"#)).contains("sensitive-value")
    );
}
#[tokio::test]
async fn invalid_or_secret_snapshot_never_reaches_storage() {
    let store = MemoryStore::default();
    let cert = certificate();
    let guard = SecretGuard::default();
    let mut bad = snapshot();
    bad.data.requests[0].action_indices = vec![1];
    assert!(
        RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), bad, None)
            .await
            .is_err()
    );
    let mut bad = snapshot();
    bad.data.contents = RestrictedJson(r#"{"actions":[{}],"password":"fixture"}"#.to_owned());
    assert_eq!(
        code(RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), bad, None).await),
        JournalCode::CredentialMaterial
    );
    assert!(store.objects.lock().unwrap().is_empty());
}
#[tokio::test]
async fn faults_during_preparation_never_issue_a_receipt() {
    for fault in [
        Fault::BeforeCreate,
        Fault::AfterCreate,
        Fault::Denied,
        Fault::Corrupt,
        Fault::WrongVersion,
        Fault::MissingHold,
    ] {
        let store = MemoryStore::default();
        store.fault(fault);
        assert!(RunJournal::persist_prepared(
            &store,
            &certificate(),
            &SecretGuard::default(),
            hash("run"),
            snapshot(),
            None
        )
        .await
        .is_err());
    }
}
#[tokio::test]
async fn intent_readback_failures_never_issue_a_dispatch_receipt() {
    for fault in [
        Fault::Denied,
        Fault::Corrupt,
        Fault::WrongVersion,
        Fault::MissingHold,
    ] {
        let store = MemoryStore::default();
        let cert = certificate();
        let guard = SecretGuard::default();
        let (mut writer, _) =
            RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
                .await
                .unwrap();
        store.fault(fault);
        assert!(writer.record_intent(0).await.is_err());
        store.fault(Fault::None);
        assert_eq!(
            code(writer.record_intent(0).await),
            JournalCode::UnknownStorageOutcome
        );
        let recovered = recover_run(&store, &cert, &target(), &hash("run"), None)
            .await
            .unwrap();
        assert_eq!(recovered.unresolved.pending_requests.len(), 1);
    }
}
#[tokio::test]
async fn missing_prepared_marker_or_baseline_cannot_be_recovered_as_complete() {
    for removed in ["prepared", "unresolved", "baseline"] {
        let store = MemoryStore::default();
        let cert = certificate();
        let guard = SecretGuard::default();
        let (writer, _) =
            RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
                .await
                .unwrap();
        let anchor = writer.unresolved();
        let key = if removed == "baseline" {
            anchor.first_adoption.key.clone()
        } else {
            format!("{}/{removed}", prefix(&target(), &hash("run")).unwrap())
        };
        store.objects.lock().unwrap().remove(&key);
        assert_eq!(
            code(recover_run(&store, &cert, &target(), &hash("run"), None).await),
            JournalCode::MissingArtifact
        );
    }
}
#[tokio::test]
async fn invalid_observations_and_aggregate_byte_exhaustion_do_not_append() {
    let store = MemoryStore::default();
    let cert = certificate();
    let guard = SecretGuard::default();
    let (mut writer, _) =
        RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
            .await
            .unwrap();
    let observation = Observation::Transport {
        request: request(vec![0]).hash().unwrap(),
        status: Some(200),
        response_body_hash: None,
        version: None,
        request_id: None,
        outcome: TransportOutcome::Response,
    };
    assert_eq!(
        code(writer.record_observation(observation.clone()).await),
        JournalCode::InvalidTransition
    );
    writer.record_intent(0).await.unwrap();
    assert_eq!(
        code(writer.record_observation(observation).await),
        JournalCode::InvalidInput
    );
    let before = store.objects.lock().unwrap().len();
    writer.bytes = MAX_RECOVERY_BYTES;
    assert_eq!(
        code(
            writer
                .record_observation(Observation::Transport {
                    request: request(vec![0]).hash().unwrap(),
                    status: None,
                    response_body_hash: None,
                    version: None,
                    request_id: None,
                    outcome: TransportOutcome::Timeout
                })
                .await
        ),
        JournalCode::LimitExceeded
    );
    assert_eq!(store.objects.lock().unwrap().len(), before);
}
#[tokio::test]
async fn before_intent_failure_is_empty_and_handle_must_not_retry() {
    let store = MemoryStore::default();
    let cert = certificate();
    let guard = SecretGuard::default();
    let (mut writer, receipt) =
        RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
            .await
            .unwrap();
    assert!(!receipt.is_certified());
    let anchor = writer.unresolved();
    store.fault(Fault::BeforeCreate);
    assert_eq!(
        code(writer.record_intent(0).await),
        JournalCode::AccessDenied
    );
    store.fault(Fault::None);
    assert_eq!(
        code(writer.record_intent(0).await),
        JournalCode::UnknownStorageOutcome
    );
    let recovered = recover_run(&store, &cert, &target(), &hash("run"), Some(&anchor))
        .await
        .unwrap();
    assert!(recovered.unresolved.pending_requests.is_empty());
    assert!(recovered.unresolved.blocks_new_execution);
}
#[tokio::test]
async fn after_intent_before_dispatch_and_lost_storage_ack_both_remain_unknown() {
    for lost_ack in [false, true] {
        let store = MemoryStore::default();
        let cert = certificate();
        let guard = SecretGuard::default();
        let (mut writer, _) =
            RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
                .await
                .unwrap();
        if lost_ack {
            store.fault(Fault::AfterCreate);
        }
        let intent = writer.record_intent(0).await;
        assert_eq!(intent.is_err(), lost_ack);
        store.fault(Fault::None);
        drop(writer);
        let recovered = recover_run(&store, &cert, &target(), &hash("run"), None)
            .await
            .unwrap();
        assert_eq!(
            recovered.unresolved.pending_requests,
            vec![request(vec![0]).hash().unwrap()]
        );
        assert!(recovered.observations.is_empty());
        assert_eq!(
            recovered.snapshot.requests()[0].body(),
            request(vec![0]).body()
        );
        assert_eq!(
            recovered.unresolved.pending_create_ownership,
            vec![hash("fixture-pending-create")]
        );
    }
}
#[tokio::test]
async fn responses_confirmed_images_and_verification_are_durable_but_never_authorize_retry() {
    for status in [200, 512] {
        let store = MemoryStore::default();
        let cert = certificate();
        let guard = SecretGuard::default();
        let (mut writer, _) =
            RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
                .await
                .unwrap();
        let intent = writer.record_intent(0).await.unwrap();
        assert_eq!(
            code(writer.record_intent(0).await),
            JournalCode::InvalidTransition
        );
        let request = intent.request().hash().unwrap();
        writer
            .record_observation(Observation::Transport {
                request: request.clone(),
                status: Some(status),
                response_body_hash: Some(hash("response-body")),
                version: Some(name("v2")),
                request_id: Some(name("request-1")),
                outcome: TransportOutcome::Response,
            })
            .await
            .unwrap();
        writer.record_observation(Observation::DirectState { request: request.clone(), inventory_hash: hash("observed-state"),
            images: raw(r#"{"server_id":"observed-create-id","full_row":{"unowned":["B","A"],"managed":true}}"#), evidence: hash("observation-contract") }).await.unwrap();
        writer
            .record_observation(Observation::Verification {
                request,
                evidence: raw(r#"{"v1":"pending","v2":"pending","v3":"pending","v4":"pending"}"#),
            })
            .await
            .unwrap();
        let anchor = writer.unresolved();
        drop(writer);
        let recovered = recover_run(&store, &cert, &target(), &hash("run"), Some(&anchor))
            .await
            .unwrap();
        assert_eq!(recovered.observations.len(), 3);
        assert_eq!(recovered.unresolved.pending_requests.len(), 1);
        let report = String::from_utf8(encode(&recovered.report()).unwrap()).unwrap();
        for private in [
            "observed-create-id",
            "unowned",
            "fixture-org",
            "fixture-workspace",
            "request-1",
        ] {
            assert!(!report.contains(private));
        }
        assert!(report.contains("synthetic_fixture"));
        assert!(report.contains("disabled"));
    }
}
#[tokio::test]
async fn observation_lost_ack_is_recovered_and_missing_or_corrupt_events_refuse() {
    let store = MemoryStore::default();
    let cert = certificate();
    let guard = SecretGuard::default();
    let (mut writer, _) =
        RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
            .await
            .unwrap();
    let request = writer
        .record_intent(0)
        .await
        .unwrap()
        .request()
        .hash()
        .unwrap();
    let intent_anchor = writer.unresolved();
    store.fault(Fault::AfterCreate);
    assert!(writer
        .record_observation(Observation::Transport {
            request,
            status: None,
            response_body_hash: None,
            version: None,
            request_id: None,
            outcome: TransportOutcome::ConnectionLost
        })
        .await
        .is_err());
    store.fault(Fault::None);
    let recovered = recover_run(&store, &cert, &target(), &hash("run"), Some(&intent_anchor))
        .await
        .unwrap();
    assert_eq!(recovered.observations.len(), 1);
    let tail_anchor = recovered.unresolved;
    let tail_key = event_key(&tail_anchor.locator, 1).unwrap();
    let tail = store.objects.lock().unwrap().remove(&tail_key).unwrap();
    assert_eq!(
        code(recover_run(&store, &cert, &target(), &hash("run"), Some(&tail_anchor)).await),
        JournalCode::IncompleteHistory
    );
    store.objects.lock().unwrap().insert(tail_key, tail);
    store
        .objects
        .lock()
        .unwrap()
        .remove(&event_key(&tail_anchor.locator, 0).unwrap());
    assert_eq!(
        code(recover_run(&store, &cert, &target(), &hash("run"), None).await),
        JournalCode::IncompleteHistory
    );
}
#[tokio::test]
async fn first_adoption_is_not_replaced_and_later_runs_bind_its_exact_reference() {
    let store = MemoryStore::default();
    let cert = certificate();
    let guard = SecretGuard::default();
    let (writer, _) =
        RunJournal::persist_prepared(&store, &cert, &guard, hash("run-1"), snapshot(), None)
            .await
            .unwrap();
    let original = writer.unresolved().first_adoption;
    assert_eq!(
        code(
            RunJournal::persist_prepared(&store, &cert, &guard, hash("run-2"), snapshot(), None)
                .await
        ),
        JournalCode::Exists
    );
    let mut later = snapshot();
    later.data.contents = raw(r#"{"actions":[{}],"first_adoption":"later-change"}"#);
    RunJournal::persist_prepared(&store, &cert, &guard, hash("run-2"), later, Some(&original))
        .await
        .unwrap();
    let recovered = recover_run(&store, &cert, &target(), &hash("run-2"), None)
        .await
        .unwrap();
    assert_eq!(
        recovered.first_adoption.contents().bytes(),
        snapshot().contents().bytes()
    );
    assert_ne!(
        recovered.first_adoption.contents().bytes(),
        recovered.snapshot.contents().bytes()
    );
    let mut wrong = original;
    wrong.hash = hash("wrong");
    assert_eq!(
        code(
            RunJournal::persist_prepared(
                &store,
                &cert,
                &guard,
                hash("run-3"),
                snapshot(),
                Some(&wrong)
            )
            .await
        ),
        JournalCode::CorruptArtifact
    );
}
#[tokio::test]
async fn read_denial_corruption_and_version_changes_never_become_absence() {
    let store = MemoryStore::default();
    let cert = certificate();
    let guard = SecretGuard::default();
    let (writer, _) =
        RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
            .await
            .unwrap();
    let anchor = writer.unresolved();
    for fault in [
        Fault::Denied,
        Fault::Corrupt,
        Fault::WrongVersion,
        Fault::MissingHold,
    ] {
        store.fault(fault);
        assert!(recover(&store, &cert, &anchor.locator, Some(&anchor))
            .await
            .is_err());
    }
    store.fault(Fault::None);
    let mut inconsistent = anchor.clone();
    inconsistent
        .pending_requests
        .push(hash("no recorded intent"));
    assert_eq!(
        code(recover(&store, &cert, &anchor.locator, Some(&inconsistent)).await),
        JournalCode::BindingMismatch
    );
    let mut wrong = anchor.locator.clone();
    wrong.target.instance_generation += 1;
    assert_eq!(
        code(recover(&store, &cert, &wrong, None).await),
        JournalCode::BindingMismatch
    );
}
#[tokio::test]
async fn append_collisions_and_limits_refuse_without_overwriting() {
    let store = MemoryStore::default();
    let cert = certificate();
    let guard = SecretGuard::default();
    let (mut writer, _) =
        RunJournal::persist_prepared(&store, &cert, &guard, hash("run"), snapshot(), None)
            .await
            .unwrap();
    let key = event_key(&writer.locator, 0).unwrap();
    store
        .create(&key, b"owned collision sentinel", &retention())
        .await
        .unwrap();
    assert_eq!(code(writer.record_intent(0).await), JournalCode::Exists);
    assert_eq!(
        store.objects.lock().unwrap()[&key].bytes,
        b"owned collision sentinel"
    );
    let mut oversized = snapshot();
    oversized.data.requests = vec![request(vec![0]); MAX_REQUESTS + 1];
    assert_eq!(code(oversized.validate()), JournalCode::LimitExceeded);
    writer.poisoned = false;
    writer.next_sequence = MAX_EVENTS;
    assert_eq!(
        code(writer.record_intent(0).await),
        JournalCode::LimitExceeded
    );
}

#[path = "journal_fixture_tests.rs"]
mod fixture;
