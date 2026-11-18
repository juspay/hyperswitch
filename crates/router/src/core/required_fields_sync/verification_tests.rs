//! Synthetic read orchestration is not evidence of pod access, server exports or backup publication.
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{
    super::super::{
        apply_contract::{self as ac, ApplyVerifier},
        journal,
        verification::*,
        verification_provider::*,
    },
    *,
};

struct Source(Fixture);
impl Source {
    fn export(&self) -> Vec<u8> {
        let mut config = self.0.export();
        config["dimensions"] = self
            .0
            .dimensions
            .iter()
            .map(|r| {
                let mut row = serde_json::Map::new();
                for key in [
                    "schema",
                    "position",
                    "dimension_type",
                    "dependency_graph",
                    "value_compute_function_name",
                ] {
                    row.insert(key.into(), r[key].clone());
                }
                (
                    r["dimension"].as_str().unwrap().to_owned(),
                    Value::Object(row),
                )
            })
            .collect::<serde_json::Map<_, _>>()
            .into();
        serde_json::to_vec(&config).unwrap()
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
impl ReadSource for Source {
    async fn read(
        &self,
        request: ReadRequest,
        timeout: Duration,
        cap: usize,
    ) -> InventoryResult<RawResponse> {
        let mut response = self.0.read(request, timeout, cap).await?;
        if matches!(request, ReadRequest::VersionHead) {
            let mut body: Value = serde_json::from_slice(&response.body).unwrap();
            body["data"][0]["config"] = serde_json::from_slice(&self.export()).unwrap();
            response.body = serde_json::to_vec(&body).unwrap();
        }
        Ok(response)
    }
}
#[derive(Clone, Copy, Debug)]
enum Fault {
    None,
    RawRow,
    CachedExport,
    OldVersion,
    Dimension,
    DuplicateJson,
    StalePod,
    WrongPod,
    AbsentPod,
    Scaled,
    MissingMetadata,
    MissingBrowser,
    WrongPhase,
    WrongInput,
    WrongAssets,
    MissingCheckout,
    Credentials,
    Confirm,
    MissingBackup,
    WrongBackupTarget,
    StaleBackup,
    OldReader,
    Deadline,
}
struct Reads {
    source: Source,
    population: Population,
    calls: AtomicUsize,
    fault: Fault,
}
impl VerificationReads for Reads {
    fn now_ms(&self) -> u64 {
        if matches!(self.fault, Fault::Deadline) {
            1000
        } else {
            100
        }
    }
    async fn population(&self) -> ac::ApplyResult<Population> {
        let mut p = self.population.clone();
        if matches!(self.fault, Fault::AbsentPod) {
            p.pods.pop();
        }
        if matches!(self.fault, Fault::Scaled) && self.calls.fetch_add(1, Ordering::SeqCst) > 0 {
            p.membership = d("scaled");
        }
        if matches!(self.fault, Fault::OldReader) {
            p.pods[0].reader = d("old-unregistered-reader");
        }
        Ok(p)
    }
    async fn persisted(&self) -> ac::ApplyResult<ExportRead> {
        let mut source = Source(self.source.0.clone());
        if matches!(self.fault, Fault::RawRow) {
            source.0.contexts[0]["future_metadata"] = v!({"lost":true});
        }
        let mut latest = source.export();
        if matches!(self.fault, Fault::CachedExport) {
            let mut v: Value = serde_json::from_slice(&latest).unwrap();
            v["default_configs"][OPTIONS] = v!(["B", "A"]);
            latest = serde_json::to_vec(&v).unwrap();
        }
        if matches!(self.fault, Fault::DuplicateJson) {
            latest = br#"{"contexts":[],"contexts":[]}"#.to_vec();
        }
        if matches!(self.fault, Fault::Dimension) {
            let mut v: Value = serde_json::from_slice(&latest).unwrap();
            v["dimensions"] = v!({});
            latest = serde_json::to_vec(&v).unwrap();
        }
        Ok(ExportRead {
            inventory: source.inventory().await,
            latest,
            versioned: source.export(),
            requested_version: n(if matches!(self.fault, Fault::OldVersion) {
                "old"
            } else {
                "version-1"
            }),
        })
    }
    async fn pod(&self, key: &ProbeKey) -> ac::ApplyResult<PodRead> {
        let body = if matches!(self.fault, Fault::StalePod) {
            br#"{"raw_configs":{},"payment_methods":[]}"#.to_vec()
        } else if matches!(self.fault, Fault::MissingMetadata) {
            br#"{"raw_configs":{}}"#.to_vec()
        } else {
            sdk_response()
        };
        Ok(PodRead {
            key: key.clone(),
            addressed_pod: if matches!(self.fault, Fault::WrongPod) {
                n("load-balancer")
            } else {
                key.pod_uid.clone()
            },
            status: 200,
            body,
        })
    }
    async fn browser(&self, binding: &BrowserBinding, body: &[u8]) -> ac::ApplyResult<BrowserRead> {
        assert_eq!(bytes_hash(body).unwrap(), binding.response);
        if matches!(self.fault, Fault::MissingBrowser) {
            return Err(ac::fail(ac::ApplyCode::VerificationFailed));
        }
        let mut binding = binding.clone();
        if matches!(self.fault, Fault::WrongPhase) {
            binding.phase = BrowserPhase::ProjectedBefore;
        }
        if matches!(self.fault, Fault::WrongInput) {
            binding.input = d("wrong");
        }
        if matches!(self.fault, Fault::WrongAssets) {
            binding.artifacts = d("wrong");
        }
        Ok(BrowserRead {
            binding,
            report: journal::RestrictedJson::parse(
                br#"{"synthetic_validator_only":true}"#,
                &journal::SecretGuard::default(),
            )
            .unwrap(),
            components_passed: true,
            checkout_passed: !matches!(self.fault, Fault::MissingCheckout),
            retained_snapshot_passed: true,
            assets_passed: true,
            credentials_absent: !matches!(self.fault, Fault::Credentials),
            confirm_intercepted: !matches!(self.fault, Fault::Confirm),
        })
    }
    async fn backup(&self, reader: &Digest) -> ac::ApplyResult<BackupRead> {
        if matches!(self.fault, Fault::MissingBackup) {
            return Err(ac::fail(ac::ApplyCode::VerificationFailed));
        }
        let content = self.source.export();
        let mut target = self.population.target.clone();
        if matches!(self.fault, Fault::WrongBackupTarget) {
            target.workspace = n("wrong-target");
        }
        Ok(BackupRead {
            target,
            reader: reader.clone(),
            version: n("version-1"),
            published_ms: if matches!(self.fault, Fault::StaleBackup) {
                1
            } else {
                90
            },
            publication: d("synthetic-publication"),
            startup: StartupReceipt {
                content: bytes_hash(&content).unwrap(),
                config: bytes_hash(
                    &raw_json::Json::parse(&content)
                        .unwrap()
                        .canonical()
                        .unwrap(),
                )
                .unwrap(),
            },
            content,
        })
    }
}
fn sdk_response() -> Vec<u8> {
    br#"{"raw_configs":{"contexts":[],"overrides":{},"default_configs":{"dynamic_fields.choice._dropdown_options":["A","B"]}},"payment_methods":[]}"#.to_vec()
}
async fn verifier(
    fault: Fault,
) -> (
    ServingVerifier<Reads>,
    ac::VerificationBinding,
    Vec<ExpectedProbe>,
) {
    let source = Source(Fixture::new());
    let inventory = source.inventory().await;
    let binding = ac::VerificationBinding {
        target: inventory.proof().target.clone(),
        plan: d("plan"),
        inventory: inventory.proof().inventory_hash.clone(),
        release: d("release"),
    };
    let population = Population {
        target: binding.target.clone(),
        release: binding.release.clone(),
        membership: d("population"),
        pods: vec![
            Pod {
                uid: n("pod-a"),
                reader: d("reader"),
                build: d("build"),
                effective_inputs: d("inputs"),
            },
            Pod {
                uid: n("pod-b"),
                reader: d("reader"),
                build: d("build"),
                effective_inputs: d("inputs"),
            },
        ],
        observed_ms: 80,
        valid_until_ms: 2000,
        complete: true,
    };
    let probes: Vec<_> = population
        .pods
        .iter()
        .flat_map(|p| {
            [SdkRoute::Payment, SdkRoute::Profile].map(|route| ExpectedProbe {
                request: d("request"),
                key: ProbeKey {
                    pod_uid: p.uid.clone(),
                    reader: p.reader.clone(),
                    route,
                    input: d("input"),
                },
                response: sdk_response(),
                cases: d("cases"),
                artifacts: d("assets"),
            })
        })
        .collect();
    let contract = VerificationContract {
        scope: EvidenceScope::SyntheticFixture,
        run: d("run"),
        binding: binding.clone(),
        population: population.clone(),
        probes: probes.clone(),
        read_contract: inventory.proof().evidence.contract_id.clone(),
        deadline_ms: 1000,
        publication_after_ms: 50,
        guard: journal::SecretGuard::default(),
    };
    (
        ServingVerifier::new(
            contract,
            Reads {
                source,
                population,
                calls: AtomicUsize::new(0),
                fault,
            },
        ),
        binding,
        probes,
    )
}
#[tokio::test]
async fn verification_all_levels_and_both_projected_phases_bind_exact_inputs() {
    let (v, b, p) = verifier(Fault::None).await;
    assert_eq!(
        v.preflight(BrowserPhase::ProjectedBefore, &p)
            .await
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        v.preflight(BrowserPhase::ProjectedAfter, &p)
            .await
            .unwrap()
            .len(),
        4
    );
    assert!(v.preflight(BrowserPhase::PostApply, &p).await.is_err());
    let receipt = v.verify(&b).await.unwrap();
    assert_eq!(receipt.scope, EvidenceScope::SyntheticFixture);
    assert_eq!(receipt.binding, ac::hash(&b).unwrap());
    assert!(!String::from_utf8_lossy(receipt.evidence.bytes()).contains("dropdown_options"));
}

#[tokio::test]
async fn verification_export_checks_metadata_even_when_both_exports_and_head_agree() {
    for fault in 0..8 {
        let source = Source(Fixture::new());
        let mut inventory = source.inventory().await;
        let mut export: Value = serde_json::from_slice(&source.export()).unwrap();
        match fault {
            0 => export["default_configs"][OPTIONS] = v!(["B", "A"]),
            1 => export["overrides"]["override-server-A"][OPTIONS] = v!(["B", "A"]),
            2 => export["contexts"][0]["condition"]["connector"] = v!("unowned"),
            3 => export["contexts"][0]["override_with_keys"] = v!(["wrong-id"]),
            4 => export["dimensions"]["connector"]["schema"] = v!({"type":"number"}),
            5 => {
                export["dimensions"]["connector"]
                    .as_object_mut()
                    .unwrap()
                    .remove("schema");
            }
            6 => {
                export["dimensions"]["connector"]
                    .as_object_mut()
                    .unwrap()
                    .remove("value_compute_function_name");
            }
            _ => export["overrides"]["unknown"] = v!({"preserved":false}),
        }
        let mut head: Value = serde_json::from_str(inventory.version_pages[0].body.raw()).unwrap();
        head["data"][0]["config"] = export.clone();
        inventory.version_pages[0].body =
            raw_json::Json::parse(&serde_json::to_vec(&head).unwrap()).unwrap();
        let bytes = serde_json::to_vec(&export).unwrap();
        let result = verify_exports(&ExportRead {
            inventory,
            latest: bytes.clone(),
            versioned: bytes,
            requested_version: n("version-1"),
        });
        assert_eq!(result.is_ok(), fault == 6, "fault {fault}");
    }
}
#[tokio::test]
async fn verification_refuses_stale_partial_forged_missing_and_scaled_evidence() {
    for fault in [
        Fault::RawRow,
        Fault::CachedExport,
        Fault::OldVersion,
        Fault::Dimension,
        Fault::DuplicateJson,
        Fault::StalePod,
        Fault::WrongPod,
        Fault::AbsentPod,
        Fault::Scaled,
        Fault::MissingMetadata,
        Fault::MissingBrowser,
        Fault::WrongPhase,
        Fault::WrongInput,
        Fault::WrongAssets,
        Fault::MissingCheckout,
        Fault::Credentials,
        Fault::Confirm,
        Fault::MissingBackup,
        Fault::WrongBackupTarget,
        Fault::StaleBackup,
        Fault::OldReader,
        Fault::Deadline,
    ] {
        let (v, b, _) = verifier(fault).await;
        assert!(v.verify(&b).await.is_err(), "{fault:?}");
    }
    assert!(VerificationContract::from_registry(&n("unregistered")).is_err());
}
#[tokio::test]
#[ignore = "requires explicitly registered RF_VERIFICATION_FIXTURE_ROOT; no network"]
async fn verification_actual_provider_startup_and_last_good_are_separate() {
    use superposition_provider::{
        data_source::file::FileDataSource, EvaluationContext, LocalResolutionProvider,
        RefreshStrategy, SuperpositionDataSource,
    };
    let root =
        std::fs::canonicalize(std::env::var("RF_VERIFICATION_FIXTURE_ROOT").unwrap()).unwrap();
    let owner: Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(owner["purpose"], "Step 11 isolated verification checks");
    assert_eq!(owner["root"], root.to_str().unwrap());
    assert_eq!(owner["cleanup_scope"], owner["root"]);
    assert!(owner["hosted_target"].is_null());
    let dir = (0..32)
        .find_map(|i| {
            let dir = root.join(format!("provider-{i}"));
            match std::fs::create_dir(&dir) {
                Ok(()) => Some(dir),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(e) => panic!("owned fixture creation failed: {e}"),
            }
        })
        .expect("owned fixture slot available");
    std::fs::write(
        dir.join("owner.json"),
        serde_json::to_vec(&v!({"root":dir,"owner":owner["owner"]})).unwrap(),
    )
    .unwrap();
    let backup = dir.join("backup.json");
    let primary = dir.join("primary.json");
    let config=br#"{"contexts":[],"overrides":{},"default_configs":{"synthetic.choice":["B","A"]},"dimensions":{}}"#;
    let file = br#"{"default-configs":{"synthetic.choice":{"value":["B","A"],"schema":{"type":"array","items":{"type":"string"}}}},"dimensions":{},"overrides":[]}"#;
    std::fs::write(&backup, file).unwrap();
    let digest = bytes_hash(file).unwrap();
    let wrong=br#"{"contexts":[],"overrides":{},"default_configs":{"synthetic.choice":["A","B"]},"dimensions":{}}"#;
    assert!(startup_canary(&backup, &primary, &digest, wrong)
        .await
        .is_err());
    startup_canary(&backup, &primary, &digest, config)
        .await
        .unwrap();
    assert!(startup_canary(&backup, &primary, &d("wrong"), config)
        .await
        .is_err());
    std::fs::write(&backup, b"not JSON").unwrap();
    assert!(
        startup_canary(&backup, &primary, &bytes_hash(b"not JSON").unwrap(), config)
            .await
            .is_err()
    );
    let toml_backup = dir.join("backup.toml");
    let toml_file=b"[default-configs.\"synthetic.choice\"]\nvalue = [\"B\",\"A\"]\nschema = { type = \"array\", items = { type = \"string\" } }\n[dimensions]\n";
    std::fs::write(&toml_backup, toml_file).unwrap();
    startup_canary(
        &toml_backup,
        &primary,
        &bytes_hash(toml_file).unwrap(),
        config,
    )
    .await
    .unwrap();
    std::fs::write(&backup, file).unwrap();
    std::fs::write(&primary, file).unwrap();
    assert!(startup_canary(&backup, &primary, &digest, config)
        .await
        .is_err());
    let running = LocalResolutionProvider::new(
        Box::new(FileDataSource::new(primary.clone()).unwrap()),
        Some(Box::new(FileDataSource::new(backup.clone()).unwrap())),
        RefreshStrategy::Manual,
    );
    running.init(EvaluationContext::default()).await.unwrap();
    std::fs::remove_file(&primary).unwrap();
    std::fs::remove_file(&backup).unwrap();
    assert!(running.refresh().await.is_err());
    assert_eq!(
        serde_json::to_value(
            running
                .fetch_config(None)
                .await
                .unwrap()
                .into_data()
                .unwrap()
                .data
        )
        .unwrap()["default_configs"]["synthetic.choice"],
        v!(["B", "A"])
    );
    assert!(startup_canary(&backup, &primary, &digest, config)
        .await
        .is_err());
    running.close_provider().await.unwrap();
    let marker: Value =
        serde_json::from_slice(&std::fs::read(dir.join("owner.json")).unwrap()).unwrap();
    assert_eq!(marker["owner"], owner["owner"]);
    assert_eq!(marker["root"], dir.to_str().unwrap());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "requires owned Step 11 browser execution record, never a synthetic successful report"]
fn verification_reads_actual_companion_receipt_and_rejects_rebinding() {
    let root =
        std::fs::canonicalize(std::env::var("RF_VERIFICATION_FIXTURE_ROOT").unwrap()).unwrap();
    let owner: Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(owner["purpose"], "Step 11 isolated verification checks");
    assert_eq!(owner["root"], root.to_str().unwrap());
    let execution: Value =
        serde_json::from_slice(&std::fs::read(root.join("browser-execution.json")).unwrap())
            .unwrap();
    let owned_read = |key: &str| {
        let path = std::fs::canonicalize(execution[key].as_str().unwrap()).unwrap();
        assert!(path.starts_with(&root));
        std::fs::read(path).unwrap()
    };
    let request_bytes = owned_read("input");
    let input: Value = serde_json::from_slice(&request_bytes).unwrap();
    let registration: Value = serde_json::from_slice(&owned_read("registration")).unwrap();
    let report = owned_read("report");
    let mut pack: Value = serde_json::from_str(include_str!(
        "../../../../../docs/required_fields_sync_contracts/sdk-v1/cases.json"
    ))
    .unwrap();
    let reader = bytes_hash(
        &raw_json::Json::parse(&serde_json::to_vec(&pack["provenance"]).unwrap())
            .unwrap()
            .canonical()
            .unwrap(),
    )
    .unwrap();
    pack["browser_case_ids"] = v!([
        "B-response-0",
        "E-response-0",
        "B-response-1",
        "E-response-1",
        "B-response-2",
        "E-response-2",
        "E-retained"
    ]);
    let plain = |v: &Value| -> Digest {
        format!("sha256:{}", v.as_str().unwrap())
            .try_into()
            .unwrap()
    };
    for (i, phase) in [
        BrowserPhase::ProjectedBefore,
        BrowserPhase::ProjectedAfter,
        BrowserPhase::PostApply,
    ]
    .into_iter()
    .enumerate()
    {
        let c = &input["cases"][i];
        let binding = BrowserBinding {
            request: bytes_hash(&request_bytes).unwrap(),
            phase,
            run: plain(&input["run"]),
            target: plain(&input["target"]),
            plan: plain(&input["plan"]),
            response: bytes_hash(c["response_utf8"].as_str().unwrap().as_bytes()).unwrap(),
            input: bytes_hash(
                &raw_json::Json::parse(&serde_json::to_vec(&c["input"]).unwrap())
                    .unwrap()
                    .canonical()
                    .unwrap(),
            )
            .unwrap(),
            cases: bytes_hash(
                &raw_json::Json::parse(&serde_json::to_vec(&pack).unwrap())
                    .unwrap()
                    .canonical()
                    .unwrap(),
            )
            .unwrap(),
            artifacts: plain(&registration["artifact_sha256"]),
            reader: reader.clone(),
        };
        BrowserRead::from_companion(
            binding.clone(),
            &report,
            EvidenceScope::SyntheticFixture,
            &journal::SecretGuard::default(),
        )
        .unwrap();
        assert!(BrowserRead::from_companion(
            binding.clone(),
            &report,
            EvidenceScope::CertifiedServer,
            &journal::SecretGuard::default()
        )
        .is_err());
        for change in 0..7 {
            let mut bad = binding.clone();
            match change {
                0 => bad.response = d("wrong"),
                1 => bad.input = d("wrong"),
                2 => bad.artifacts = d("wrong"),
                3 => bad.request = d("wrong"),
                4 => bad.cases = d("wrong"),
                5 => bad.reader = d("wrong"),
                _ => bad.run = d("wrong"),
            };
            assert!(BrowserRead::from_companion(
                bad,
                &report,
                EvidenceScope::SyntheticFixture,
                &journal::SecretGuard::default()
            )
            .is_err());
        }
        let mut bad: Value = serde_json::from_slice(&report).unwrap();
        bad["reports"][1]["results"][0]["status"] = v!("not_run");
        assert!(BrowserRead::from_companion(
            binding,
            &serde_json::to_vec(&bad).unwrap(),
            EvidenceScope::SyntheticFixture,
            &journal::SecretGuard::default()
        )
        .is_err());
    }
}
