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
