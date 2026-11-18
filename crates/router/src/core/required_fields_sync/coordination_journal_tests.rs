//! End-to-end type binding with actual accepted planner, journal and coordinator. All external
//! contracts are explicitly synthetic. No renderer profile, server or hosted control is certified.
use std::sync::atomic::AtomicU64;

use super::{
    super::super::{authority as a, coordination as c, journal as j},
    *,
};

#[tokio::test]
async fn accepted_plan_journal_receipt_and_active_record_bind_before_dispatch_check() {
    let inventory = Fixture::new().inventory().await;
    let mut desired = desired();
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .remove(&key(OPTIONS));
    let release = ReleaseIdentity {
        application_revision: "4".repeat(40).try_into().unwrap(),
        source_tree_hash: d("fixture-tree"),
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
    let ownership = grants(&inventory);
    let prerequisites = prereqs(&desired);
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
    let mut registration = c::tests::registration();
    registration.target = inventory.proof().target.clone();
    registration.authority = projection.authority.clone();
    let reader = registration.readers.remove(&n("main")).unwrap();
    registration
        .readers
        .insert(registration.authority.clone(), reader);
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
    for operation in [c::Operation::Apply, c::Operation::Restore] {
        let owner = c::tests::owner();
        let store = j::tests::MemoryStore::default();
        let certificate = j::tests::certificate();
        let guard = j::SecretGuard::default();
        let raw = |bytes: &[u8]| j::RestrictedJson::parse(bytes, &guard).unwrap();
        let snapshot = j::PreparedSnapshot::capture(
            j::Capture {
                plan: &plan,
                inventory: &inventory,
                ownership: &ownership,
                prerequisites: &prerequisites,
                projection: &projection,
                bundle: raw(&serde_json::to_vec(&bundle).unwrap()),
                identity: j::RecoveryIdentity {
                    target: registration.target.clone(),
                    endpoint_identity: registration.endpoint_identity.clone(),
                    coordinator: registration.coordinator.object.clone(),
                    executor_image: intent.executor_image.clone(),
                    retained_recovery_image: d("recovery-image"),
                },
                retention: j::Retention {
                    created_at_unix: 1,
                    retain_until_unix: 1 + j::MIN_RETENTION_SECONDS,
                    rollback_window_seconds: 86400,
                    protected_hold: true,
                },
                requests: vec![j::PreparedRequest::new(
                    j::Method::Put,
                    "/synthetic-only/bulk".to_owned(),
                    BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]),
                    raw(br#"[{"synthetic_request":true}]"#),
                    (0..plan.actions().len()).collect(),
                    d("synthetic-wire"),
                )
                .unwrap()],
                unreadable_attributes: vec![],
                server_managed_attributes: vec![n("last_modified_at")],
            },
            &guard,
        )
        .unwrap();
        let (mut journal, prepared) = j::RunJournal::persist_prepared(
            &store,
            &certificate,
            &guard,
            owner.run.clone(),
            snapshot,
            None,
        )
        .await
        .unwrap();
        registration.journal_contract = prepared.store_contract().clone();
        let target = c::tests::certificate(registration.clone());
        let coordinator = c::tests::Memory::new(&registration);
        let clock = c::tests::Clock(AtomicU64::new(1000));
        let controls = c::tests::Controls::new(&registration, &intent, &owner, 1000);
        let session = c::ExecutionSession::acquire(
            &coordinator,
            &controls,
            &clock,
            &target,
            c::ExecutionRequest {
                owner: owner.clone(),
                operation,
                intent: intent.clone(),
                plan: plan.report().plan_hash.clone().unwrap(),
            },
            None,
        )
        .await
        .unwrap();
        session
            .bind_journal(&journal.checkpoint().unwrap())
            .await
            .unwrap();
        assert_eq!(
            coordinator
                .row
                .lock()
                .unwrap()
                .1
                .active
                .as_ref()
                .unwrap()
                .phase,
            c::RunPhase::Prepared
        );
        let recorded = journal.record_intent(0).await.unwrap();
        let request = recorded.request().hash().unwrap();
        session
            .reserve_request(&journal.checkpoint().unwrap(), &recorded)
            .await
            .unwrap();
        session.recheck_before_dispatch(&request).await.unwrap();
        assert_eq!(
            coordinator
                .row
                .lock()
                .unwrap()
                .1
                .active
                .as_ref()
                .unwrap()
                .request_in_flight
                .as_ref(),
            Some(&request)
        );
        session.release_to_recovery().await.unwrap();
        let active = coordinator.row.lock().unwrap().1.active.clone().unwrap();
        let recovery = c::inspect_recovery(&store, &certificate, &active)
            .await
            .unwrap();
        assert_eq!(
            recovery.head(),
            &a::hash(&Some(journal.unresolved())).unwrap()
        );
        assert!(active.journal.unwrap().blocks_new_execution);
        // Exact plan mismatch is rejected by the sealed journal bridge, not a caller flag.
        assert!(journal
            .checkpoint()
            .unwrap()
            .matches(&registration, &owner.run, &intent, &d("wrong-plan"), true)
            .is_err());
        let mut drift = intent.clone();
        drift.policy = d("wrong-policy");
        assert!(journal
            .checkpoint()
            .unwrap()
            .matches(
                &registration,
                &owner.run,
                &drift,
                plan.report().plan_hash.as_ref().unwrap(),
                true
            )
            .is_err());
    }
}
