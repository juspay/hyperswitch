//! Router argument shapes only. The scheduler retains the original serving-only parser.

use std::path::PathBuf;

#[derive(clap::Parser, Default)]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,
}

#[derive(clap::Parser, Default)]
#[command(name = "router")]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct RouterCmdLineConf {
    #[command(flatten)]
    pub serving: CmdLineConf,
    #[cfg(feature = "required_fields_sync")]
    #[command(subcommand)]
    pub command: Option<RouterCommand>,
}

#[cfg(feature = "required_fields_sync")]
#[derive(clap::Subcommand)]
pub enum RouterCommand {
    /// Offline rendering and provisional read-only planning. Hosted apply remains disabled.
    RequiredFields(RequiredFieldsArgs),
}

#[cfg(feature = "required_fields_sync")]
#[derive(clap::Args)]
pub struct RequiredFieldsArgs {
    #[command(subcommand)]
    pub command: RequiredFieldsCommand,
}

#[cfg(feature = "required_fields_sync")]
#[derive(clap::Args, Clone)]
pub struct ArtifactArgs {
    /// Explicit bounded JSON profile; seed profiles ignore ambient bank settings.
    #[arg(long, value_name = "PROFILE.json")]
    pub input_profile: PathBuf,
    /// New private artifact file; existing files and symlinks are never replaced.
    #[arg(long, value_name = "OUTPUT.json")]
    pub output: PathBuf,
}

#[cfg(feature = "required_fields_sync")]
#[derive(clap::Subcommand, Clone)]
pub enum RequiredFieldsCommand {
    /// Render compiled required fields without initializing a service or HTTP client.
    Render(ArtifactArgs),
    /// Discover a registered read-only target and emit a provisional, unverified candidate.
    Plan {
        #[command(flatten)]
        artifacts: ArtifactArgs,
        /// Certified allowlisted identity, never an endpoint or org/workspace override.
        #[arg(long, value_name = "REGISTERED_ID")]
        target: String,
        /// Explicit plan-mode configuration, grants, prerequisites and release intent.
        #[arg(long, value_name = "PLAN.json")]
        plan_config: PathBuf,
    },
}

/// Clap diagnostics may echo supplied argument values. Required-fields failures use a fixed
/// envelope even when the feature is absent; serving-only argument diagnostics remain unchanged.
#[allow(
    clippy::print_stderr,
    reason = "CLI argument refusal precedes application logging"
)]
pub fn rejected_required_fields_arguments() -> std::process::ExitCode {
    let code = if cfg!(feature = "required_fields_sync") {
        "input_invalid"
    } else {
        "unsupported_build"
    };
    eprintln!("{{\"schema_version\":1,\"outcome\":\"refused\",\"code\":\"{code}\",\"phase\":\"arguments\",\"superposition_mutations_dispatched\":0,\"leases_acquired\":0,\"hosted_apply\":\"disabled\"}}");
    std::process::ExitCode::from(2)
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
