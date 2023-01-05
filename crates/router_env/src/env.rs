//!
//! Current environment related stuff.
//!

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Parent dir where Cargo.toml is stored
pub const CARGO_MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";
/// Env variable that sets Development/Production env
pub const RUN_ENV: &str = "RUN_ENV";

///
/// Current environment.
///

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, Display, EnumString)]
pub enum Env {
    /// Development environment.
    #[default]
    Development,
    /// Sandbox environment.
    Sandbox,
    /// Production environment.
    Production,
}

/// Name of current environment. Either "Development", "Sandbox" or "Production".
pub fn which() -> Env {
    #[cfg(debug_assertions)]
    let default_env = Env::Development;
    #[cfg(not(debug_assertions))]
    let default_env = Env::Production;

    std::env::var(RUN_ENV).map_or_else(|_| default_env, |v| v.parse().unwrap_or(default_env))
}

///
/// Base path to look for config and logs directories.
/// Application expects to find `./config/` and `./logs/` relative this directories.
///
/// Using workspace and splitting into monolith into crates introduce introduce extra level of complexity.
/// We can't rely on current working directory anymore because there are several ways of running applications.
///
/// Developer can run application from root of repository:
/// ```bash
/// cargo run
/// ```
///
/// Alternatively, developer can run from directory of crate:
/// ```bash
/// cd crates/router
/// cargo run
/// ```
///
/// Config and log files are located at root. No matter application is run it should work properly.
/// `router_log::env::workspace_path` takes care of the problem returning path tho workspace relative which should other paths be calculated.
///

pub fn workspace_path() -> PathBuf {
    // for (key, value) in std::env::vars() {
    //     println!("{key} : {value}");
    // }
    if let Ok(manifest_dir) = std::env::var(CARGO_MANIFEST_DIR) {
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
/// - Semantic Version from the latest git tag. If tags are not present in the repository, crate
///   version from the crate manifest is used instead.
/// - Short hash of the latest git commit.
/// - Timestamp of the latest git commit.
///
/// Example: `0.1.0-abcd012-2038-01-19T03:14:08Z`.
#[macro_export]
macro_rules! version {
    () => {
        concat!(
            env!("VERGEN_GIT_SEMVER"),
            "-",
            env!("VERGEN_GIT_SHA_SHORT"),
            "-",
            env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        )
    };
}

///
/// A string uniquely idendify build of the service.
///
/// Consists of combination of
/// - Version defined in the crate file
/// - Timestamp of commit
/// - Hash of the commit
/// - Version of rust compiler
/// - Target triple
///
/// Example: `0.1.0-f5f383e-2022-09-04T11:39:37Z-1.63.0-x86_64-unknown-linux-gnu`
///

#[macro_export]
macro_rules! build {
    () => {
        concat!(
            env!("CARGO_PKG_VERSION"),
            "-",
            env!("VERGEN_GIT_SHA_SHORT"),
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

///
/// Full hash of the current commit.
///
/// Example: `f5f383ee7e36214d60ce3c6353b57db03ff0ceb1`.
///

#[macro_export]
macro_rules! commit {
    () => {
        env!("VERGEN_GIT_SHA")
    };
}

// ///
// /// Information about the platform on which service was built, including:
// /// - Information about OS
// /// - Information about CPU
// ///
// /// Example: ``.
// ///
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

///
/// Service name deduced from name of the crate.
///
/// Example: `router`.
///

#[macro_export]
macro_rules! service_name {
    () => {
        env!("CARGO_CRATE_NAME")
    };
}

///
/// Build profile, either debug or release.
///
/// Example: `release`.
///

#[macro_export]
macro_rules! profile {
    () => {
        env!("VERGEN_CARGO_PROFILE")
    };
}
