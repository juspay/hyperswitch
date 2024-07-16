//! Information about the current environment.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Environment variables accessed by the application. This module aims to be the source of truth
/// containing all environment variable that the application accesses.
pub mod vars {
    /// Parent directory where `Cargo.toml` is stored.
    pub const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";

    /// Environment variable that sets development/sandbox/production environment.
    pub const RUN_ENV: &str = "RUN_ENV";

    /// Directory of config TOML files. Default is `config`.
    pub const CONFIG_DIR: &str = "CONFIG_DIR";
}

/// Current environment.
#[derive(
    Debug, Default, Deserialize, Serialize, Clone, Copy, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Env {
    /// Development environment.
    #[default]
    Development,
    /// Sandbox environment.
    Sandbox,
    /// Production environment.
    Production,
}

/// Name of current environment. Either "development", "sandbox" or "production".
pub fn which() -> Env {
    #[cfg(debug_assertions)]
    let default_env = Env::Development;
    #[cfg(not(debug_assertions))]
    let default_env = Env::Production;

    std::env::var(vars::RUN_ENV).map_or_else(|_| default_env, |v| v.parse().unwrap_or(default_env))
}

/// Three letter (lowercase) prefix corresponding to the current environment.
/// Either `dev`, `snd` or `prd`.
pub fn prefix_for_env() -> &'static str {
    match which() {
        Env::Development => "dev",
        Env::Sandbox => "snd",
        Env::Production => "prd",
    }
}

/// Path to the root directory of the cargo workspace.
/// It is recommended that this be used by the application as the base path to build other paths
/// such as configuration and logs directories.
pub fn workspace_path() -> PathBuf {
    if let Ok(manifest_dir) = std::env::var(vars::CARGO_MANIFEST_DIR) {
        let mut path = PathBuf::from(manifest_dir);
        path.pop();
        path.pop();
        path
    } else {
        PathBuf::from(".")
    }
}

/// Version of the crate containing the following information:
///
/// - The latest git tag. If tags are not present in the repository, the short commit hash is used
///   instead.
/// - Short hash of the latest git commit.
/// - Timestamp of the latest git commit.
///
/// Example: `0.1.0-abcd012-2038-01-19T03:14:08Z`.
#[cfg(feature = "vergen")]
#[macro_export]
macro_rules! version {
    () => {
        concat!(
            env!("VERGEN_GIT_DESCRIBE"),
            "-",
            env!("VERGEN_GIT_SHA"),
            "-",
            env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        )
    };
}

/// A string uniquely identifying the application build.
///
/// Consists of a combination of:
/// - Version defined in the crate file
/// - Timestamp of commit
/// - Hash of the commit
/// - Version of rust compiler
/// - Target triple
///
/// Example: `0.1.0-f5f383e-2022-09-04T11:39:37Z-1.63.0-x86_64-unknown-linux-gnu`
#[cfg(feature = "vergen")]
#[macro_export]
macro_rules! build {
    () => {
        concat!(
            env!("CARGO_PKG_VERSION"),
            "-",
            env!("VERGEN_GIT_SHA"),
            "-",
            env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
            "-",
            env!("VERGEN_RUSTC_SEMVER"),
            "-",
            $crate::profile!(),
            "-",
            env!("VERGEN_CARGO_TARGET_TRIPLE"),
        )
    };
}

/// Short hash of the current commit.
///
/// Example: `f5f383e`.
#[cfg(feature = "vergen")]
#[macro_export]
macro_rules! commit {
    () => {
        env!("VERGEN_GIT_SHA")
    };
}

// /// Information about the platform on which service was built, including:
// /// - Information about OS
// /// - Information about CPU
// ///
// /// Example: ``.
// #[macro_export]
// macro_rules! platform {
//     (
//     ) => {
//         concat!(
//             env!("VERGEN_SYSINFO_OS_VERSION"),
//             " - ",
//             env!("VERGEN_SYSINFO_CPU_BRAND"),
//         )
//     };
// }

/// Service name deduced from name of the binary.
/// This macro must be called within binaries only.
///
/// Example: `router`.
#[macro_export]
macro_rules! service_name {
    () => {
        env!("CARGO_BIN_NAME")
    };
}

/// Build profile, either debug or release.
///
/// Example: `release`.
#[macro_export]
macro_rules! profile {
    () => {
        env!("CARGO_PROFILE")
    };
}

/// The latest git tag. If tags are not present in the repository, the short commit hash is used
/// instead. Refer to the [`git describe`](https://git-scm.com/docs/git-describe) documentation for
/// more details.
#[macro_export]
macro_rules! git_tag {
    () => {
        env!("VERGEN_GIT_DESCRIBE")
    };
}
