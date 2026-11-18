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
    /// Required-fields executor. Uncertified targets and hosted mutation remain disabled.
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
#[derive(clap::Args, Clone)]
pub struct OperationArgs {
    /// Registered identity; endpoint, headers and credentials cannot be supplied here.
    #[arg(long)]
    pub target: String,
    /// Strict operation configuration. Missing mode means disabled.
    #[arg(long)]
    pub command_config: PathBuf,
    /// New private final report, never an existing file or symlink.
    #[arg(long)]
    pub output: PathBuf,
}

#[cfg(feature = "required_fields_sync")]
#[derive(clap::Subcommand, Clone)]
pub enum RequiredFieldsCommand {
    /// Render compiled required fields without initializing a service or HTTP client.
    Render(ArtifactArgs),
    /// Generate or check an explicitly granted local seed subset without uploading anything.
    Seed {
        #[arg(long)]
        input_profile: PathBuf,
        #[arg(long)]
        seed_policy: PathBuf,
        #[arg(long)]
        template: PathBuf,
        /// Compare in memory; never writes the template or an output file.
        #[arg(long, conflicts_with = "output", required_unless_present = "output")]
        check: bool,
        #[arg(long, conflicts_with = "check", required_unless_present = "check")]
        output: Option<PathBuf>,
    },
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
    /// Read durable run evidence without acquiring a lease or changing state.
    Status {
        #[command(flatten)]
        args: OperationArgs,
        #[arg(long)]
        run: String,
    },
    /// Execute an exact accepted plan through all certified safety gates.
    Apply {
        #[command(flatten)]
        args: OperationArgs,
        #[arg(long)]
        intent: u64,
        #[arg(long)]
        plan_hash: String,
    },
    /// Observe a prior run and replan under a new authorized intent; never replay blindly.
    Resume {
        #[command(flatten)]
        args: OperationArgs,
        #[arg(long)]
        run: String,
        #[arg(long)]
        intent: u64,
        #[arg(long)]
        plan_hash: String,
    },
    /// Plan an inverse without writes, or apply its exact accepted hash with every apply gate.
    Restore {
        #[command(flatten)]
        args: OperationArgs,
        #[arg(long)]
        from_run: String,
        #[arg(long)]
        intent: u64,
        #[arg(
            long,
            conflicts_with = "plan_hash",
            required_unless_present = "plan_hash"
        )]
        plan: bool,
        #[arg(long, conflicts_with = "plan", required_unless_present = "plan")]
        plan_hash: Option<String>,
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
