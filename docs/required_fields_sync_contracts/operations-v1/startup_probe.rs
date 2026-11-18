//! Read-only test executable linked to the actual built router library, never a shipped command.
//! This avoids the unrelated default router unit-test email feature compilation failure.
use std::{fs, io::Write as _, path::PathBuf};

use router::{
    configs::settings::{RequiredFields, SecuredSecret, Settings},
    core::required_fields_sync::{command_runtime::startup_attestation, operation::Mode},
};

fn put(path: PathBuf, value: &serde_json::Value) {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path).unwrap();
    file.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
    file.sync_all().unwrap();
}

fn main() {
    let root = PathBuf::from(std::env::args_os().nth(1).expect("owned fixture required"));
    assert!(root.is_absolute());
    let owner: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(owner["purpose"], "Step 13 isolated command checks");
    assert_eq!(owner["root"], serde_json::json!(root));
    assert_eq!(owner["cleanup_scope"], serde_json::json!(root));
    assert!(owner["hosted_target"].is_null());
    let work = root.join("startup-library-probe");
    fs::create_dir(&work).unwrap();
    let mut conf = Settings::<SecuredSecret>::default();
    conf.required_fields = RequiredFields::new(&conf.bank_config);
    assert_eq!(conf.required_fields_sync.mode, Mode::Disabled);
    let baseline = startup_attestation(&conf).unwrap();
    assert_eq!(
        baseline.effective_settings,
        conf.required_fields_inputs()
            .unwrap()
            .settings_hash()
            .unwrap()
    );
    assert_eq!(
        baseline.retained_rules,
        conf.required_fields_inputs()
            .unwrap()
            .retained_rules_hash()
            .unwrap()
    );
    assert_eq!(baseline.desired_state, "profile_not_configured");
    assert!(baseline.desired.is_none());
    let expected = serde_json::to_value(&baseline.effective_settings).unwrap();
    let mut reports = vec![serde_json::to_value(baseline).unwrap()];
    let mut profile: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("seed-profile.json")).unwrap()).unwrap();
    profile["profile_id"] = serde_json::json!("synthetic-sensitive-never-log");
    profile["inputs"] = serde_json::json!({"kind":"effective_settings", "settings_file":"not-reloaded.toml", "expected_settings_hash":expected});
    conf.required_fields_sync.mode = Mode::Apply; // Serving still cannot execute synchronization.
    for (name, state) in [
        ("matching", "observed"),
        ("drift", "refused"),
        ("seed", "refused"),
        ("missing", "refused"),
    ] {
        if name == "drift" {
            profile["inputs"]["expected_settings_hash"] =
                serde_json::json!(format!("sha256:{}", "0".repeat(64)));
        }
        if name == "seed" {
            profile["inputs"] = serde_json::json!({"kind":"seed", "stripe_eps_banks":null});
        }
        let path = work.join(format!("{name}.json"));
        if name != "missing" {
            put(path.clone(), &profile);
        }
        conf.required_fields_sync.attestation_profile = Some(path);
        let observation = startup_attestation(&conf).unwrap();
        assert_eq!(observation.desired_state, state);
        assert_eq!(observation.desired.is_some(), name == "matching");
        assert_eq!(observation.bundle.is_some(), name == "matching");
        assert_eq!(
            observation.image_provenance,
            "pending_platform_verification"
        );
        assert_eq!(observation.application_rollout, "not_asserted");
        assert_eq!(observation.synchronization, "not_executed");
        assert_eq!(observation.verification, "not_executed");
        assert_eq!(
            serde_json::to_value(&observation.effective_settings).unwrap(),
            expected
        );
        let report = serde_json::to_value(observation).unwrap();
        let text = report.to_string();
        for forbidden in [
            "synthetic-sensitive-never-log",
            "not-reloaded",
            "master_database",
            "prefill",
            root.to_str().unwrap(),
        ] {
            assert!(!text.contains(forbidden));
        }
        reports.push(report);
    }
    put(
        root.join("startup-probe-results.json"),
        &serde_json::json!({"cases_passed":reports.len(),
        "scope":"actual_library_startup_observation_only_not_platform_attestation", "observations":reports}),
    );
    println!("{{\"cases_passed\":5,\"scope\":\"actual_router_library\"}}");
}
