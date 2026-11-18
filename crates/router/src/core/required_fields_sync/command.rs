//! Bounded filesystem/command orchestration. The backend interface has no mutation or lease API.

use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{
    ownership::OwnershipManifest,
    plan,
    planner_types::{self, PlanIntent, PlanLimits, ProjectionContract, VerifiedDesired},
    preflight::PrerequisitePolicy,
    raw_json::Json,
    types::{CertifiedReadTarget, CompleteInventory, InventoryLimits, TargetIdentity},
    ArtifactIdentity, Contract, Digest, ExclusionPolicy, FieldContractCatalog, MappingPolicy, Name,
    ReleaseIdentity,
};
use crate::configs::cli::{ArtifactArgs, RequiredFieldsCommand};

// Profiles can contain explicit exclusions for every compiled rule. They are bounded independently
// of the smaller individual policy/approval interfaces; this does not raise any API/action limit.
pub(super) const MAX_PROFILE_BYTES: usize = 2 * 1024 * 1024;
pub(super) const MAX_PLAN_BYTES: usize = 128 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandCode {
    InputInvalid,
    UnsupportedBuild,
    Disabled,
    MappingUnresolved,
    SettingsMismatch,
    UnsupportedServerContract,
    TargetMismatch,
    CredentialInvalid,
    ReadIncomplete,
    PlanRefused,
    OutputUnavailable,
    CertificationPending,
    NotAuthoritative,
    JournalUnavailable,
    SeedDrift,
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Arguments,
    Input,
    Render,
    Target,
    Credential,
    Discovery,
    Plan,
    Output,
    Status,
    Execute,
    Seed,
}

#[derive(Debug, thiserror::Error, Serialize)]
#[error("required-fields command refused: {code:?}")]
pub struct CommandError {
    pub code: CommandCode,
    pub phase: Phase,
    pub detail: Option<String>,
    pub diagnostic_counts: BTreeMap<String, usize>,
}
impl CommandError {
    pub fn exit_code(&self) -> u8 {
        if matches!(
            self.code,
            CommandCode::NotAuthoritative | CommandCode::TargetMismatch
        ) {
            3
        } else {
            2
        }
    }
}
pub(super) type CommandResult<T> = error_stack::Result<T, CommandError>;
pub(super) fn fail(code: CommandCode, phase: Phase) -> error_stack::Report<CommandError> {
    CommandError {
        code,
        phase,
        detail: None,
        diagnostic_counts: BTreeMap::new(),
    }
    .into()
}
pub(super) fn plan_error(
    error: error_stack::Report<planner_types::PlanError>,
) -> error_stack::Report<CommandError> {
    CommandError {
        code: CommandCode::PlanRefused,
        phase: Phase::Plan,
        detail: Some(format!("{:?}", error.current_context().code)),
        diagnostic_counts: BTreeMap::new(),
    }
    .into()
}
pub(super) fn inventory_error(
    error: error_stack::Report<super::types::InventoryError>,
    phase: Phase,
) -> error_stack::Report<CommandError> {
    CommandError {
        code: if matches!(phase, Phase::Target) {
            CommandCode::UnsupportedServerContract
        } else {
            CommandCode::ReadIncomplete
        },
        phase,
        detail: Some(format!("{:?}", error.current_context().kind)),
        diagnostic_counts: BTreeMap::new(),
    }
    .into()
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum InputSource {
    /// This is the one current RequiredFields constructor dependency, explicitly supplied.
    Seed { stripe_eps_banks: Option<Vec<Name>> },
    /// Uses the ordinary Settings source precedence, but deserializes only bank inputs.
    EffectiveSettings {
        settings_file: PathBuf,
        expected_settings_hash: Digest,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InputProfile {
    pub schema_version: u16,
    pub profile_id: Name,
    pub inputs: InputSource,
    pub release: ReleaseIdentity,
    pub policy: MappingPolicy,
    pub catalog: FieldContractCatalog,
    pub exclusions: ExclusionPolicy,
}
impl InputProfile {
    pub(super) fn validate(&self) -> CommandResult<()> {
        if self.schema_version != 1 || !matches!(self.release.artifact, ArtifactIdentity::Offline) {
            return Err(fail(CommandCode::InputInvalid, Phase::Input));
        }
        // These commands do not attest a deployed image. Later release integration must supply
        // an authenticated artifact identity instead of accepting a user-written image claim.
        self.policy
            .validate()
            .and_then(|_| self.catalog.validate())
            .and_then(|_| self.exclusions.validate())
            .map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?;
        if let InputSource::Seed {
            stripe_eps_banks: Some(banks),
        } = &self.inputs
        {
            if banks.len() > 256
                || banks
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != banks.len()
            {
                return Err(fail(CommandCode::InputInvalid, Phase::Input));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PlanMode {
    #[default]
    Disabled,
    Plan,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PlanConfiguration {
    pub schema_version: u16,
    #[serde(default)]
    pub mode: PlanMode,
    pub target: TargetIdentity,
    pub projection_contract: Name,
    /// Explicit mounted reader token file, never a CLI token or ambient router credential.
    pub reader_credential_file: PathBuf,
    pub ownership: OwnershipManifest,
    pub prerequisites: PrerequisitePolicy,
    pub intent: PlanIntent,
    pub limits: PlanLimits,
}

pub(super) struct RenderedInput {
    pub bundle: Json,
    pub desired: VerifiedDesired,
}
pub(super) struct Registration {
    pub read: CertifiedReadTarget,
    pub projection: ProjectionContract,
}
pub(super) trait Runtime {
    fn render(&self, profile: &InputProfile, directory: &Path) -> CommandResult<RenderedInput>;
    fn register(&self, target: &Name, projection: &Name) -> CommandResult<Registration>;
    async fn inventory(
        &self,
        target: &Name,
        credential: &Path,
        limits: InventoryLimits,
    ) -> CommandResult<CompleteInventory>;
}

#[derive(Serialize)]
pub struct CommandSummary {
    pub schema_version: u16,
    pub outcome: &'static str,
    pub profile_hash: Option<Digest>,
    pub bundle_hash: Option<Digest>,
    pub candidate_hash: Option<Digest>,
    pub provisional: bool,
    pub sdk_verification: &'static str,
    pub superposition_mutations_dispatched: u32,
    pub leases_acquired: u32,
    pub hosted_apply: &'static str,
}
impl CommandSummary {
    fn new(outcome: &'static str, profile_hash: Digest, bundle_hash: Digest) -> Self {
        Self {
            schema_version: 1,
            outcome,
            profile_hash: Some(profile_hash),
            bundle_hash: Some(bundle_hash),
            candidate_hash: None,
            provisional: true,
            sdk_verification: "not_executed",
            superposition_mutations_dispatched: 0,
            leases_acquired: 0,
            hosted_apply: "disabled",
        }
    }
}

/// Never includes paths, input values, endpoint/headers, credentials or source error strings.
pub fn refusal_json(error: &CommandError) -> serde_json::Value {
    serde_json::json!({"schema_version":1,"outcome":"refused","code":error.code,"phase":error.phase,"exit_code":error.exit_code(),
        "detail":error.detail,"diagnostic_counts":error.diagnostic_counts,"superposition_mutations_dispatched":0,
        "leases_acquired":0,"hosted_apply":"disabled", "application_rollout":"not_asserted",
        "collector_delivery":"pending_platform_certification",
        "metrics":{"outcome":"refused","phase":error.phase,"refusal":error.code,
            "requests_dispatched":0,"target_class":"unregistered_or_fixture"}})
}

pub(super) fn read_file(path: &Path, cap: usize, phase: Phase) -> CommandResult<Vec<u8>> {
    let read_cap = u64::try_from(cap).map_err(|_| fail(CommandCode::InputInvalid, phase))?;
    let read_limit = read_cap
        .checked_add(1)
        .ok_or_else(|| fail(CommandCode::InputInvalid, phase))?;
    let before =
        std::fs::symlink_metadata(path).map_err(|_| fail(CommandCode::InputInvalid, phase))?;
    if !before.is_file() || before.len() > read_cap {
        return Err(fail(CommandCode::InputInvalid, phase));
    }
    let file = File::open(path).map_err(|_| fail(CommandCode::InputInvalid, phase))?;
    let opened = file
        .metadata()
        .map_err(|_| fail(CommandCode::InputInvalid, phase))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if before.dev() != opened.dev() || before.ino() != opened.ino() {
            return Err(fail(CommandCode::InputInvalid, phase));
        }
    }
    if !opened.is_file() {
        return Err(fail(CommandCode::InputInvalid, phase));
    }
    let mut bytes = Vec::new();
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|_| fail(CommandCode::InputInvalid, phase))?;
    if bytes.len() > cap {
        return Err(fail(CommandCode::InputInvalid, phase));
    }
    Ok(bytes)
}
pub(super) fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> CommandResult<T> {
    Json::parse(bytes).map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))?;
    serde_json::from_slice(bytes).map_err(|_| fail(CommandCode::InputInvalid, Phase::Input))
}
pub(super) fn output_available(path: &Path) -> CommandResult<()> {
    if path.file_name().is_none()
        || std::fs::symlink_metadata(path).is_ok()
        || !path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .is_dir()
    {
        return Err(fail(CommandCode::OutputUnavailable, Phase::Output));
    }
    Ok(())
}
pub(super) fn write_artifact(path: &Path, value: &serde_json::Value) -> CommandResult<()> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|_| fail(CommandCode::OutputUnavailable, Phase::Output))?;
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(fail(CommandCode::OutputUnavailable, Phase::Output));
    }
    bytes.push(b'\n');
    write_new(path, &bytes)
}
pub(super) fn write_bytes(path: &Path, bytes: &[u8]) -> CommandResult<()> {
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(fail(CommandCode::OutputUnavailable, Phase::Output));
    }
    write_new(path, bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> CommandResult<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| fail(CommandCode::OutputUnavailable, Phase::Output))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| fail(CommandCode::OutputUnavailable, Phase::Output))
}
pub(super) fn relative_to(directory: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        directory.join(path)
    }
}

pub(super) async fn run(
    command: RequiredFieldsCommand,
    runtime: &impl Runtime,
) -> CommandResult<CommandSummary> {
    let artifacts: &ArtifactArgs = match &command {
        RequiredFieldsCommand::Render(args) => args,
        RequiredFieldsCommand::Plan { artifacts, .. } => artifacts,
        _ => return Err(fail(CommandCode::InputInvalid, Phase::Arguments)),
    };
    output_available(&artifacts.output)?;
    let bytes = read_file(&artifacts.input_profile, MAX_PROFILE_BYTES, Phase::Input)?;
    let profile: InputProfile = parse(&bytes)?;
    profile.validate()?;
    let profile_hash = planner_types::digest(&profile).map_err(plan_error)?;
    let directory = artifacts
        .input_profile
        .parent()
        .unwrap_or_else(|| Path::new("."));
    // Validate all plan/target authority before opening settings or a credential file.
    let configuration = if let RequiredFieldsCommand::Plan {
        target,
        plan_config,
        ..
    } = &command
    {
        let target: Name = target
            .clone()
            .try_into()
            .map_err(|_| fail(CommandCode::TargetMismatch, Phase::Target))?;
        let config: PlanConfiguration =
            parse(&read_file(plan_config, MAX_PLAN_BYTES, Phase::Input)?)?;
        if config.schema_version != 1
            || config.target.target_id != target
            || config.ownership.target != config.target
        {
            return Err(fail(CommandCode::TargetMismatch, Phase::Target));
        }
        if !matches!(config.mode, PlanMode::Plan) {
            return Err(fail(CommandCode::Disabled, Phase::Target));
        }
        config.limits.validate().map_err(plan_error)?;
        let registration = runtime.register(&target, &config.projection_contract)?;
        if registration.read.identity != config.target
            || registration.projection.target != config.target
        {
            return Err(fail(CommandCode::TargetMismatch, Phase::Target));
        }
        Some((
            config,
            registration,
            plan_config
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_owned(),
        ))
    } else {
        None
    };
    let rendered = match runtime.render(&profile, directory) {
        Ok(value) => value,
        Err(error) => {
            // Known renderer refusals get a sanitized artifact; malformed inputs/I/O never get
            // rewritten as a successful empty bundle. Both stderr and artifact carry failure.
            if matches!(error.current_context().phase, Phase::Render) {
                write_artifact(&artifacts.output, &refusal_json(error.current_context()))?;
            }
            return Err(error);
        }
    };
    if let Some((config, registration, config_directory)) = configuration {
        if config.intent.release_hash != *rendered.desired.release_hash() {
            return Err(fail(CommandCode::TargetMismatch, Phase::Target));
        }
        let inventory = runtime
            .inventory(
                &config.target.target_id,
                &relative_to(&config_directory, &config.reader_credential_file),
                InventoryLimits::default(),
            )
            .await?;
        let candidate = plan::prepare(
            &inventory,
            &rendered.desired,
            &config.ownership,
            &config.prerequisites,
            config.intent,
            &registration.projection,
            config.limits,
        )
        .map_err(plan_error)?;
        let mut summary = CommandSummary::new(
            "planned_provisional",
            profile_hash,
            rendered.desired.bundle_hash().clone(),
        );
        summary.candidate_hash = Some(candidate.report().candidate_hash.clone());
        // Only sanitized report data is exported here. Raw live rows/case bodies are restricted
        // artifacts for a later journal/verified companion boundary, never CLI stdout.
        write_artifact(
            &artifacts.output,
            &serde_json::json!({"summary":summary,"plan":candidate.report()}),
        )?;
        Ok(summary)
    } else {
        let summary = CommandSummary::new(
            "rendered_offline",
            profile_hash,
            rendered.desired.bundle_hash().clone(),
        );
        let bundle: serde_json::Value = serde_json::from_str(rendered.bundle.raw())
            .map_err(|_| fail(CommandCode::InputInvalid, Phase::Render))?;
        write_artifact(
            &artifacts.output,
            &serde_json::json!({"summary":summary,"bundle":bundle}),
        )?;
        Ok(summary)
    }
}
