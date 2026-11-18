//! Orchestration tests with explicit synthetic backends. Not production render/server evidence.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::{
    cell::Cell,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use super::{
    super::super::command::{self, *},
    *,
};
use crate::configs::cli::{ArtifactArgs, RequiredFieldsCommand};

fn owned_root() -> PathBuf {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let root = PathBuf::from(std::env::var("RF_CLI_FIXTURE_ROOT").expect("register fixture first"));
    let owner: Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert!(root.is_absolute());
    assert_eq!(owner["purpose"], "Step 7 offline CLI checks");
    assert_eq!(owner["root"], v!(root));
    assert_eq!(owner["cleanup_scope"], v!(root));
    loop {
        let child = root.join(format!(
            "unit-cli-{}",
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&child) {
            Ok(()) => return child,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("cannot create owned test directory: {error}"),
        }
    }
}
fn profile() -> InputProfile {
    let desired = desired();
    let build = serde_json::to_value(&desired.desired.input_manifest.build).unwrap();
    InputProfile {
        schema_version: 1,
        profile_id: n("synthetic-command-fixture"),
        inputs: InputSource::Seed {
            stripe_eps_banks: Some(vec![n("abn_amro")]),
        },
        release: serde_json::from_value(build["release"].clone()).unwrap(),
        policy: MappingPolicy {
            schema_version: 1,
            contract_id: n("synthetic-policy"),
            connectors: BTreeMap::new(),
            payment_methods: BTreeMap::new(),
            payment_method_types: BTreeMap::new(),
            bindings: vec![],
        },
        catalog: desired.catalog,
        exclusions: ExclusionPolicy {
            schema_version: 1,
            contract_id: n("fixture-exclusions"),
            sources: vec![],
        },
    }
}
fn put(path: &std::path::Path, value: &impl Serialize) {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .unwrap();
    file.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
}
struct SyntheticRuntime {
    fixture: Fixture,
    snapshot: CompleteInventory,
    render_calls: Cell<u32>,
    register_calls: Cell<u32>,
    read_calls: Cell<u32>,
    deny_registration: bool,
    deny_render: bool,
    deny_read: bool,
}
impl SyntheticRuntime {
    async fn new() -> Self {
        let mut fixture = Fixture::new();
        fixture.contexts[0]["override"]["manual.owner"] = v!("synthetic-sensitive-live-value");
        let snapshot = fixture.inventory().await;
        Self {
            fixture,
            snapshot,
            render_calls: Cell::new(0),
            register_calls: Cell::new(0),
            read_calls: Cell::new(0),
            deny_registration: false,
            deny_render: false,
            deny_read: false,
        }
    }
    fn configuration(&self) -> PlanConfiguration {
        let desired = desired();
        PlanConfiguration {
            schema_version: 1,
            mode: PlanMode::Plan,
            target: self.snapshot.proof().target.clone(),
            projection_contract: n("synthetic-projection"),
            reader_credential_file: PathBuf::from("not-opened-by-synthetic-backend"),
            ownership: grants(&self.snapshot),
            prerequisites: prereqs(&desired),
            intent: intent(&desired),
            limits: PlanLimits::default(),
        }
    }
}
impl Runtime for SyntheticRuntime {
    fn render(&self, _: &InputProfile, _: &std::path::Path) -> CommandResult<RenderedInput> {
        self.render_calls.set(self.render_calls.get() + 1);
        if self.deny_render {
            return Err(CommandError {
                code: CommandCode::MappingUnresolved,
                phase: Phase::Render,
                detail: None,
                diagnostic_counts: BTreeMap::from([("MissingBinding".to_owned(), 1)]),
            }
            .into());
        }
        Ok(RenderedInput {
            bundle: Json::parse(br#"{"scope":"synthetic_command_backend_only"}"#).unwrap(),
            desired: desired(),
        })
    }
    fn register(&self, target: &Name, _: &Name) -> CommandResult<Registration> {
        self.register_calls.set(self.register_calls.get() + 1);
        if self.deny_registration || target != &self.snapshot.proof().target.target_id {
            return Err(fail(CommandCode::UnsupportedServerContract, Phase::Target));
        }
        Ok(Registration {
            read: inventory::tests::target(),
            projection: contract(&self.snapshot, &desired()),
        })
    }
    async fn inventory(
        &self,
        _: &Name,
        _: &std::path::Path,
        _: InventoryLimits,
    ) -> CommandResult<CompleteInventory> {
        self.read_calls.set(self.read_calls.get() + 1);
        if self.deny_read {
            return Err(inventory_error(
                types::failure(InventoryErrorKind::PermissionDenied),
                Phase::Discovery,
            ));
        }
        Ok(self.fixture.inventory().await)
    }
}
fn artifacts(root: &std::path::Path) -> ArtifactArgs {
    ArtifactArgs {
        input_profile: root.join("profile.json"),
        output: root.join("output.json"),
    }
}
fn plan_args(root: &std::path::Path) -> RequiredFieldsCommand {
    RequiredFieldsCommand::Plan {
        artifacts: artifacts(root),
        target: "synthetic-inventory".to_owned(),
        plan_config: root.join("plan.json"),
    }
}
fn refusal<T>(result: CommandResult<T>) -> Value {
    match result {
        Err(error) => command::refusal_json(error.current_context()),
        Ok(_) => panic!("expected refusal"),
    }
}

#[tokio::test]
#[ignore = "requires RF_CLI_FIXTURE_ROOT ownership registration"]
async fn cli_orchestration_never_dispatches_mutations_or_exports_live_values() {
    let root = owned_root();
    let runtime = SyntheticRuntime::new().await;
    put(&root.join("profile.json"), &profile());
    put(&root.join("plan.json"), &runtime.configuration());
    let summary = command::run(plan_args(&root), &runtime).await.unwrap();
    assert_eq!(summary.outcome, "planned_provisional");
    assert!(summary.provisional);
    assert_eq!(summary.superposition_mutations_dispatched, 0);
    assert_eq!(summary.leases_acquired, 0);
    assert_eq!(runtime.read_calls.get(), 1);
    let text = std::fs::read_to_string(root.join("output.json")).unwrap();
    assert!(!text.contains("synthetic-sensitive-live-value"));
    assert!(!text.contains("manual.owner"));
    assert!(!text.contains("server-A"));
    let result: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(result["plan"]["scope"], "synthetic_fixture");
    assert_eq!(result["summary"]["sdk_verification"], "not_executed");
    assert!(result["plan"]["plan_hash"].is_null());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(root.join("output.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
    let root = owned_root();
    put(&root.join("profile.json"), &profile());
    let runtime = SyntheticRuntime::new().await;
    let summary = command::run(RequiredFieldsCommand::Render(artifacts(&root)), &runtime)
        .await
        .unwrap();
    assert_eq!(summary.outcome, "rendered_offline");
    assert_eq!(runtime.register_calls.get(), 0);
    assert_eq!(runtime.read_calls.get(), 0);
}

#[tokio::test]
#[ignore = "requires RF_CLI_FIXTURE_ROOT ownership registration"]
async fn cli_disabled_unregistered_and_mismatched_targets_refuse_before_settings_or_credentials() {
    for case in 0..4 {
        let root = owned_root();
        let mut runtime = SyntheticRuntime::new().await;
        let mut profile = profile();
        profile.inputs = InputSource::EffectiveSettings {
            settings_file: PathBuf::from("missing-settings-file"),
            expected_settings_hash: d("unread"),
        };
        put(&root.join("profile.json"), &profile);
        let mut config = serde_json::to_value(runtime.configuration()).unwrap();
        match case {
            0 => {
                config.as_object_mut().unwrap().remove("mode");
            }
            1 => runtime.deny_registration = true,
            2 => {
                config["target"]["instance_generation"] = v!(2);
                config["ownership"]["target"]["instance_generation"] = v!(2);
            }
            _ => config["target"]["workspace"] = v!("other-workspace"),
        };
        put(&root.join("plan.json"), &config);
        let error = refusal(command::run(plan_args(&root), &runtime).await);
        assert_eq!(error["phase"], "target");
        assert_eq!(runtime.render_calls.get(), 0);
        assert_eq!(runtime.read_calls.get(), 0);
        assert!(!root.join("output.json").exists());
    }
}

#[tokio::test]
#[ignore = "requires RF_CLI_FIXTURE_ROOT ownership registration"]
async fn cli_invalid_inputs_output_collisions_and_renderer_refusals_are_redacted() {
    for case in 0..7 {
        let root = owned_root();
        let runtime = SyntheticRuntime::new().await;
        let mut profile = serde_json::to_value(profile()).unwrap();
        match case {
            0 => profile["unknown"] = v!("synthetic-sensitive-input"),
            1 => profile["inputs"]["stripe_eps_banks"] = v!(["abn_amro", "abn_amro"]),
            2 => {
                profile["release"]["artifact"] =
                    v!({"kind":"image","index_digest":d("self-asserted"),"platform_digests":{}})
            }
            3 => {
                profile["inputs"]["kind"] = v!("remote_secret_provider");
            }
            4 => put(&root.join("output.json"), &v!("preserve-existing-output")),
            _ => {}
        }
        if case == 5 {
            std::fs::write(
                root.join("profile.json"),
                br#"{"schema_version":1,"schema_version":2,"secret":"synthetic-sensitive-input"}"#,
            )
            .unwrap();
        } else if case == 6 {
            std::fs::write(root.join("profile.json"), vec![b' '; MAX_PROFILE_BYTES + 1]).unwrap();
        } else {
            put(&root.join("profile.json"), &profile);
        }
        let error =
            refusal(command::run(RequiredFieldsCommand::Render(artifacts(&root)), &runtime).await);
        assert!(!error.to_string().contains("synthetic-sensitive-input"));
        assert_eq!(runtime.render_calls.get(), 0);
        assert_eq!(runtime.read_calls.get(), 0);
        if case == 4 {
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(root.join("output.json")).unwrap())
                    .unwrap(),
                v!("preserve-existing-output")
            );
        }
    }
    let root = owned_root();
    let mut runtime = SyntheticRuntime::new().await;
    runtime.deny_render = true;
    put(&root.join("profile.json"), &profile());
    let error =
        refusal(command::run(RequiredFieldsCommand::Render(artifacts(&root)), &runtime).await);
    assert_eq!(error["code"], "mapping_unresolved");
    let output: Value =
        serde_json::from_slice(&std::fs::read(root.join("output.json")).unwrap()).unwrap();
    assert_eq!(output["outcome"], "refused");
    assert!(output.get("bundle").is_none());
}

#[tokio::test]
#[ignore = "requires RF_CLI_FIXTURE_ROOT ownership registration"]
async fn cli_discovery_failure_cannot_become_a_successful_empty_plan() {
    let root = owned_root();
    let mut runtime = SyntheticRuntime::new().await;
    runtime.deny_read = true;
    put(&root.join("profile.json"), &profile());
    put(&root.join("plan.json"), &runtime.configuration());
    let result = refusal(command::run(plan_args(&root), &runtime).await);
    assert_eq!(result["code"], "read_incomplete");
    assert_eq!(result["detail"], "PermissionDenied");
    assert!(!root.join("output.json").exists());
    assert_eq!(result["superposition_mutations_dispatched"], 0);
    #[cfg(unix)]
    {
        let root = owned_root();
        let runtime = SyntheticRuntime::new().await;
        put(&root.join("original.json"), &profile());
        std::os::unix::fs::symlink(root.join("original.json"), root.join("profile.json")).unwrap();
        assert!(
            command::run(RequiredFieldsCommand::Render(artifacts(&root)), &runtime)
                .await
                .is_err()
        );
        assert_eq!(runtime.render_calls.get(), 0);
    }
}
