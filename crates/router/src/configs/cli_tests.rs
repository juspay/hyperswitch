#![allow(clippy::unwrap_used, clippy::expect_used)]
use clap::Parser as _;

use super::*;

#[test]
fn serving_and_scheduler_keep_the_original_argument_contract() {
    let default = RouterCmdLineConf::try_parse_from(["router"]).unwrap();
    assert!(default.serving.config_path.is_none());
    let explicit = RouterCmdLineConf::try_parse_from(["router", "-f", "required-fields"]).unwrap();
    assert_eq!(
        explicit.serving.config_path,
        Some(PathBuf::from("required-fields"))
    );
    let scheduler =
        CmdLineConf::try_parse_from(["scheduler", "--config-path", "local.toml"]).unwrap();
    assert_eq!(scheduler.config_path, Some(PathBuf::from("local.toml")));
    assert!(CmdLineConf::try_parse_from(["scheduler", "required-fields", "render"]).is_err());
}
#[cfg(feature = "required_fields_sync")]
#[test]
fn only_explicit_render_and_plan_arguments_are_accepted() {
    assert!(RouterCmdLineConf::try_parse_from([
        "router",
        "required-fields",
        "render",
        "--input-profile",
        "profile.json",
        "--output",
        "bundle.json"
    ])
    .is_ok());
    assert!(RouterCmdLineConf::try_parse_from([
        "router",
        "required-fields",
        "plan",
        "--target",
        "registered-id",
        "--input-profile",
        "profile.json",
        "--plan-config",
        "plan.json",
        "--output",
        "report.json"
    ])
    .is_ok());
    for command in ["apply", "seed", "status", "restore", "resume", "reset"] {
        assert!(RouterCmdLineConf::try_parse_from(["router", "required-fields", command]).is_err());
    }
    for flag in [
        "--force",
        "--host",
        "--endpoint",
        "--org",
        "--workspace",
        "--authorization",
        "--token",
    ] {
        assert!(RouterCmdLineConf::try_parse_from([
            "router",
            "required-fields",
            "render",
            "--input-profile",
            "profile.json",
            "--output",
            "bundle.json",
            flag,
            "synthetic-sensitive-value"
        ])
        .is_err());
    }
    assert!(RouterCmdLineConf::try_parse_from([
        "router",
        "required-fields",
        "render",
        "--output",
        "bundle.json"
    ])
    .is_err());
    assert!(RouterCmdLineConf::try_parse_from([
        "router",
        "required-fields",
        "plan",
        "--input-profile",
        "profile.json",
        "--output",
        "report.json"
    ])
    .is_err());
}

#[cfg(feature = "required_fields_sync")]
#[test]
fn execution_arguments_require_exact_intent_and_restore_choice() {
    let base = ["router", "required-fields"];
    let common = [
        "--target",
        "synthetic-target",
        "--command-config",
        "config.json",
        "--output",
        "report.json",
    ];
    for (kind, tail) in [
        ("status", vec!["--run", "run"]),
        ("apply", vec!["--intent", "2", "--plan-hash", "hash"]),
        (
            "resume",
            vec!["--run", "run", "--intent", "2", "--plan-hash", "hash"],
        ),
        (
            "restore",
            vec!["--from-run", "run", "--intent", "2", "--plan"],
        ),
        (
            "restore",
            vec!["--from-run", "run", "--intent", "2", "--plan-hash", "hash"],
        ),
    ] {
        let args: Vec<_> = base
            .into_iter()
            .chain([kind])
            .chain(common)
            .chain(tail)
            .collect();
        assert!(RouterCmdLineConf::try_parse_from(&args).is_ok(), "{kind}");
        for flag in [
            "--force",
            "--host",
            "--workspace",
            "--token",
            "--skip-verification",
        ] {
            let mut denied = args.clone();
            denied.push(flag);
            denied.push("synthetic-private-value");
            assert!(RouterCmdLineConf::try_parse_from(denied).is_err());
        }
    }
    for tail in [vec![], vec!["--plan", "--plan-hash", "hash"]] {
        assert!(RouterCmdLineConf::try_parse_from(
            base.into_iter()
                .chain(["restore"])
                .chain(common)
                .chain(["--from-run", "run", "--intent", "2"])
                .chain(tail)
        )
        .is_err());
    }
}
#[cfg(not(feature = "required_fields_sync"))]
#[test]
fn disabled_build_has_no_required_fields_command() {
    assert!(RouterCmdLineConf::try_parse_from([
        "router",
        "required-fields",
        "render",
        "--input-profile",
        "profile.json",
        "--output",
        "bundle.json"
    ])
    .is_err());
}

#[cfg(feature = "required_fields_sync")]
#[test]
fn seed_requires_an_explicit_profile_policy_template_and_exclusive_output_mode() {
    let base = [
        "router",
        "required-fields",
        "seed",
        "--input-profile",
        "local.json",
        "--seed-policy",
        "policy.json",
        "--template",
        "seed.toml",
    ];
    for tail in [vec!["--check"], vec!["--output", "new.toml"]] {
        assert!(RouterCmdLineConf::try_parse_from(base.into_iter().chain(tail)).is_ok());
    }
    for tail in [
        vec![],
        vec!["--check", "--output", "new.toml"],
        vec!["--check", "--upload"],
        vec!["--check", "--force"],
    ] {
        assert!(RouterCmdLineConf::try_parse_from(base.into_iter().chain(tail)).is_err());
    }
}
