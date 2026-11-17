#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn hosted_apply_has_no_enabled_variant() {
    assert_eq!(
        serde_json::to_value(HostedApply::default()).unwrap(),
        "disabled"
    );
    assert!(serde_json::from_str::<HostedApply>("\"apply\"").is_err());
}

/// Run explicitly with RF_FOUNDATION_FIXTURE_ROOT pointing at a registered owned directory.
/// Each child has a cleared environment; no process-global environment mutation races other tests.
#[test]
#[ignore = "requires an explicitly registered disposable filesystem fixture"]
fn required_fields_inputs_settings_precedence() {
    if let Ok(expected) = std::env::var("RF_FOUNDATION_EXPECTED_BANK") {
        let captured = load_effective_inputs(None).unwrap();
        let json = serde_json::to_value(captured).unwrap();
        assert_eq!(
            json["banks"]["stripe_eps_banks"],
            serde_json::json!([expected])
        );
        assert!(!json.to_string().contains("synthetic-database-password"));
        assert!(!json.to_string().contains("master_database"));
        return;
    }
    use std::io::Write as _;
    let root = PathBuf::from(
        std::env::var("RF_FOUNDATION_FIXTURE_ROOT")
            .expect("register fixture before running this test"),
    );
    assert!(root.join("owner.json").is_file());
    let owned = root.join(format!("settings-precedence-{}", std::process::id()));
    std::fs::create_dir(&owned).unwrap();
    let path = owned.join("development.toml");
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.write_all(b"[bank_config.eps]\nstripe = {banks = 'bank_austria'}\n[master_database]\npassword = 'synthetic-database-password'\n").unwrap();
    drop(file);
    let mut results = Vec::new();
    for override_bank in [None, Some("abn_amro")] {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .env_clear()
            .args([
                "--exact",
                "core::required_fields_sync::tests::required_fields_inputs_settings_precedence",
                "--ignored",
            ])
            .env("RUN_ENV", "development")
            .env("RUST_MIN_STACK", "11534336")
            .env("CONFIG_DIR", &owned)
            .env(
                "RF_FOUNDATION_EXPECTED_BANK",
                override_bank.unwrap_or("bank_austria"),
            );
        if let Some(bank) = override_bank {
            child.env("ROUTER__BANK_CONFIG__EPS__STRIPE__BANKS", bank);
        }
        results.push(child.output().unwrap());
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(owned).unwrap();
    for result in results {
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
    }
}
