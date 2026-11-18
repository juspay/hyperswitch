//! Uses the actual accepted planner with synthetic receipts, never a server/SDK certification.
use super::{super::super::journal::*, *};

#[tokio::test]
async fn capture_binds_plan_inventory_grants_bundle_and_retains_raw_metadata() {
    for creating in [false, true] {
        let mut fixture = Fixture::new();
        if creating {
            fixture.contexts.clear();
        }
        let inventory = fixture.inventory().await;
        let mut desired = desired();
        desired
            .desired
            .contexts
            .get_mut(&condition("a"))
            .unwrap()
            .remove(&key(OPTIONS));
        // Explicit synthetic bundle for the pure binding test; no claim of a compiled renderer profile.
        let bundle =
            v!({"synthetic_only":true,"desired":desired.desired,"catalog":desired.catalog});
        desired.bundle_hash = canonical_hash(HashDomain::Bundle, &bundle).unwrap();
        let mut ownership = grants(&inventory);
        if creating {
            ownership.creations.push(CreationGrant {
                intent_id: n("fixture-create-intent"),
                condition: condition("a"),
                attributes: BTreeSet::from([key(REQUIRED), key(OPTIONS)]),
                approver: n("fixture-owner"),
                review: n("fixture-create-review"),
            });
        }
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
        let guard = SecretGuard::default();
        let raw = |bytes: &[u8]| RestrictedJson::parse(bytes, &guard).unwrap();
        let make_input = || Capture {
            plan: &plan,
            inventory: &inventory,
            ownership: &ownership,
            prerequisites: &prerequisites,
            projection: &projection,
            bundle: raw(&serde_json::to_vec(&bundle).unwrap()),
            identity: RecoveryIdentity {
                target: inventory.proof().target.clone(),
                endpoint_identity: d("endpoint"),
                coordinator: n("fixture-coordinator"),
                executor_image: d("image"),
                retained_recovery_image: d("recovery-image"),
            },
            retention: Retention {
                created_at_unix: 1,
                retain_until_unix: 1 + MIN_RETENTION_SECONDS,
                rollback_window_seconds: 100,
                protected_hold: true,
            },
            requests: vec![PreparedRequest::new(
                Method::Put,
                "/synthetic-only/bulk".to_owned(),
                BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]),
                raw(br#"[{"synthetic_wire":"unvalidated by this step"}]"#),
                (0..plan.actions().len()).collect(),
                d("uncertified-wire"),
            )
            .unwrap()],
            unreadable_attributes: vec![n("fixture-unreadable-attribute")],
            server_managed_attributes: vec![n("last_modified_at")],
        };
        let snapshot = PreparedSnapshot::capture(make_input(), &guard).unwrap();
        assert_eq!(
            snapshot.pending_create_ownership().len(),
            usize::from(creating)
        );
        if creating {
            assert_eq!(
                snapshot.pending_create_ownership().first(),
                plan.actions()[0].ownership_intent()
            );
            assert!(plan.actions()[0].server_id().is_none());
        }
        let contents: Value = serde_json::from_slice(snapshot.contents().bytes()).unwrap();
        for (id, row) in inventory.contexts() {
            assert!(
                contents["rows"]["contexts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v.as_str() == Some(row.raw_json())),
                "missing row {id:?}"
            );
        }
        assert_eq!(
            contents["workspace"].as_str(),
            Some(inventory.workspace().raw_json())
        );
        assert_eq!(contents["accepted_reports"].as_array().unwrap().len(), 8);
        assert_eq!(contents["accepted_approval"], v!(approval));
        assert_eq!(
            contents["actions"][0]["before"].as_str(),
            plan.actions()[0].before_json()
        );
        assert_eq!(
            contents["actions"][0]["after"].as_str(),
            plan.actions()[0].after_json()
        );
        assert_eq!(
            contents["case_request"].as_str(),
            Some(plan.case_request_json())
        );
        assert!(contents["pages"].as_array().unwrap().len() >= inventory.raw_pages().len());
        let mut bad = make_input();
        bad.identity.target.instance_generation += 1;
        assert!(PreparedSnapshot::capture(bad, &guard).is_err());
        let mut bad = make_input();
        bad.bundle = raw(br#"{"synthetic_only":"drift"}"#);
        assert!(PreparedSnapshot::capture(bad, &guard).is_err());
        let mut changed_grants = ownership.clone();
        changed_grants.revision = n("changed");
        let mut bad = make_input();
        bad.ownership = &changed_grants;
        assert!(PreparedSnapshot::capture(bad, &guard).is_err());
        let mut changed_prereqs = prerequisites.clone();
        changed_prereqs.contract_id = n("changed");
        let mut bad = make_input();
        bad.prerequisites = &changed_prereqs;
        assert!(PreparedSnapshot::capture(bad, &guard).is_err());
    }
}
