//! One command protocol for status, apply, resume and both restore modes.
//! Configuration is a request, never a certificate. No endpoint or secret is accepted here.
use std::{path::Path, time::Instant};

use serde::{Deserialize, Serialize};

use super::{
    apply_contract::{ApplyCode, ApplyOutcome, ApplyReport},
    authority::ReleaseIntent,
    command::{self, fail, CommandCode, CommandResult, Phase},
    ownership::OwnershipManifest,
    planner_types::{PlanLimits, SemanticApproval},
    preflight::PrerequisitePolicy,
    types::TargetIdentity,
    Digest, Name,
};
use crate::configs::cli::{OperationArgs, RequiredFieldsCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Disabled,
    Plan,
    Apply,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Status,
    Apply,
    Resume,
    RestorePlan,
    RestoreApply,
}
impl Kind {
    pub fn mutates(self) -> bool {
        matches!(self, Self::Apply | Self::Resume | Self::RestoreApply)
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Planning {
    pub input_profile: std::path::PathBuf,
    pub projection_contract: Name,
    pub ownership: OwnershipManifest,
    pub prerequisites: PrerequisitePolicy,
    pub intent: ReleaseIntent,
    pub limits: PlanLimits,
    pub browser_reports: Vec<std::path::PathBuf>,
    pub approval: Option<SemanticApproval>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub schema_version: u16,
    #[serde(default)]
    pub mode: Mode,
    pub target: TargetIdentity,
    pub journal_store: Name,
    pub planning: Option<Planning>,
}
#[derive(Clone)]
pub struct Invocation {
    pub kind: Kind,
    pub target: Name,
    pub source_run: Option<Digest>,
    pub generation: Option<u64>,
    pub plan_hash: Option<Digest>,
}
impl Invocation {
    pub(super) fn arguments(
        command: &RequiredFieldsCommand,
    ) -> CommandResult<(&OperationArgs, Self)> {
        let (args, kind, run, generation, plan) = match command {
            RequiredFieldsCommand::Status { args, run } => {
                (args, Kind::Status, Some(run), None, None)
            }
            RequiredFieldsCommand::Apply {
                args,
                intent,
                plan_hash,
            } => (args, Kind::Apply, None, Some(*intent), Some(plan_hash)),
            RequiredFieldsCommand::Resume {
                args,
                run,
                intent,
                plan_hash,
            } => (
                args,
                Kind::Resume,
                Some(run),
                Some(*intent),
                Some(plan_hash),
            ),
            RequiredFieldsCommand::Restore {
                args,
                from_run,
                intent,
                plan,
                plan_hash,
            } if *plan != plan_hash.is_some() => (
                args,
                if *plan {
                    Kind::RestorePlan
                } else {
                    Kind::RestoreApply
                },
                Some(from_run),
                Some(*intent),
                plan_hash.as_ref(),
            ),
            _ => return Err(fail(CommandCode::InputInvalid, Phase::Arguments)),
        };
        let digest = |v: &String| {
            v.clone()
                .try_into()
                .map_err(|_| fail(CommandCode::InputInvalid, Phase::Arguments))
        };
        Ok((
            args,
            Self {
                kind,
                target: args
                    .target
                    .clone()
                    .try_into()
                    .map_err(|_| fail(CommandCode::TargetMismatch, Phase::Arguments))?,
                source_run: run.map(digest).transpose()?,
                generation,
                plan_hash: plan.map(digest).transpose()?,
            },
        ))
    }
    pub fn validate(&self, config: &Configuration) -> CommandResult<()> {
        if config.schema_version != 1
            || config.target.target_id != self.target
            || config.target.instance_generation == 0
        {
            return Err(fail(CommandCode::TargetMismatch, Phase::Target));
        }
        if self.kind == Kind::Status {
            if self.source_run.is_none()
                || self.generation.is_some()
                || self.plan_hash.is_some()
                || config.planning.is_some()
            {
                return Err(fail(CommandCode::InputInvalid, Phase::Arguments));
            }
            return Ok(());
        }
        if config.mode == Mode::Disabled || (self.kind.mutates() && config.mode != Mode::Apply) {
            return Err(fail(CommandCode::Disabled, Phase::Target));
        }
        let planning = config
            .planning
            .as_ref()
            .ok_or_else(|| fail(CommandCode::InputInvalid, Phase::Input))?;
        if self.generation != Some(planning.intent.generation)
            || planning.intent.generation == 0
            || planning.intent.target != config.target
            || planning.ownership.target != config.target
            || planning.browser_reports.len() > 16
            || self.kind.mutates() != self.plan_hash.is_some()
            || (self.kind != Kind::Apply) != self.source_run.is_some()
            || (self.kind != Kind::Apply) != planning.intent.recovery.is_some()
            || planning
                .intent
                .recovery
                .as_ref()
                .is_some_and(|r| Some(&r.from_run) != self.source_run.as_ref())
        {
            return Err(fail(CommandCode::NotAuthoritative, Phase::Target));
        }
        planning.limits.validate().map_err(command::plan_error)
    }
}

/// Typed output from the shared executor. No raw rows, profile paths, error strings or case values.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResultBody {
    Status {
        journal: super::journal::JournalReport,
    },
    RestorePlan {
        recovery: super::recovery::RecoveryReport,
        plan: Box<super::planner_types::PlanReport>,
        retained_prerequisites: Vec<Digest>,
    },
    Execution {
        report: ApplyReport,
    },
}
impl ResultBody {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Status { journal } if journal.pending_requests > 0 => 5,
            Self::Status { journal } if journal.blocks_new_execution => 3,
            Self::Status { .. } | Self::RestorePlan { .. } => 0,
            Self::Execution { report } => report.exit_code,
        }
    }
}

/// Fixed labels only. A collector can ingest this final event after the process exits.
/// Durations/counts are values; target IDs, hashes and user text are never metric dimensions.
#[derive(Serialize)]
pub struct Metrics {
    pub operation: Kind,
    pub target_class: &'static str,
    pub phase: &'static str,
    pub outcome: &'static str,
    pub refusal: Option<ApplyCode>,
    pub elapsed_ms: u64,
    pub requests_dispatched: usize,
    pub requests_committed: usize,
    pub requests_unknown: usize,
    pub lease_lost: bool,
    pub preservation_conflict: bool,
    pub actions: std::collections::BTreeMap<super::planner_types::ActionKind, usize>,
    pub request_durations: Vec<(&'static str, u64)>,
    pub verification: [(&'static str, &'static str); 4],
}
impl Metrics {
    pub fn of(kind: Kind, body: &ResultBody, elapsed_ms: u64) -> Self {
        let mut value = Self {
            operation: kind,
            target_class: "unregistered_or_fixture",
            phase: "complete",
            outcome: "observed",
            refusal: None,
            elapsed_ms,
            requests_dispatched: 0,
            requests_committed: 0,
            requests_unknown: 0,
            lease_lost: false,
            preservation_conflict: false,
            actions: std::collections::BTreeMap::new(),
            request_durations: vec![],
            verification: [
                ("persisted", "pending"),
                ("router", "pending"),
                ("sdk", "pending"),
                ("fallback", "pending"),
            ],
        };
        match body {
            ResultBody::Status { journal } => {
                value.outcome = if journal.pending_requests > 0 {
                    "commit_unknown"
                } else if journal.blocks_new_execution {
                    "recovery_required"
                } else {
                    "observed"
                };
                value.requests_unknown = journal.pending_requests;
            }
            ResultBody::RestorePlan { .. } => value.outcome = "planned_provisional",
            ResultBody::Execution { report } => {
                value.target_class = match report.evidence_scope {
                    #[cfg(test)]
                    super::types::EvidenceScope::SyntheticFixture => "synthetic_fixture",
                    super::types::EvidenceScope::CertifiedServer => "certified_server",
                };
                value.outcome = match report.outcome {
                    ApplyOutcome::NoChangeVerified => "no_change_verified",
                    ApplyOutcome::NoChangeUnverified => "no_change_unverified",
                    ApplyOutcome::Refused => "refused",
                    ApplyOutcome::RejectedNoChange => "rejected_no_change",
                    ApplyOutcome::AppliedVerified => "applied_verified",
                    ApplyOutcome::PartialApplied => "partial_applied",
                    ApplyOutcome::CommitUnknown => "commit_unknown",
                    ApplyOutcome::AppliedUnverified => "applied_unverified",
                };
                value.requests_dispatched = report.dispatched;
                value.requests_committed = report.committed.len();
                value.requests_unknown = report.unknown.len();
                value.refusal = report.reason;
                value.lease_lost = report.reason == Some(ApplyCode::LeaseLost);
                value.preservation_conflict = report.reason == Some(ApplyCode::ConcurrentChange);
                value.actions = report.actions.clone();
                value.request_durations = report
                    .requests
                    .iter()
                    .map(|r| (r.kind, r.transport_ms))
                    .collect();
                // A receipt is produced only after the existing verifier checks all four levels.
                if report.verification.is_some()
                    && matches!(
                        report.outcome,
                        ApplyOutcome::NoChangeVerified | ApplyOutcome::AppliedVerified
                    )
                {
                    for (_, state) in &mut value.verification {
                        *state = "verified";
                    }
                }
            }
        }
        value
    }
}

/// The production adapter must supply certified adapters, not deserialize capabilities from JSON.
pub(super) trait Runtime {
    async fn run(
        &self,
        invocation: &Invocation,
        config: &Configuration,
        directory: &Path,
    ) -> CommandResult<ResultBody>;
}

/// Report output is reserved before dispatch. A post-dispatch delivery error preserves the
/// actual engine outcome; it must never be mislabeled as a zero-write Refused result.
pub(super) async fn run(
    command: RequiredFieldsCommand,
    runtime: &impl Runtime,
) -> CommandResult<(serde_json::Value, u8)> {
    let started = Instant::now();
    let (args, invocation) = Invocation::arguments(&command)?;
    command::output_available(&args.output)?;
    let config: Configuration = command::parse(&command::read_file(
        &args.command_config,
        command::MAX_PROFILE_BYTES,
        Phase::Input,
    )?)?;
    invocation.validate(&config)?;
    let directory = args
        .command_config
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut output = options
        .open(&args.output)
        .map_err(|_| fail(CommandCode::OutputUnavailable, Phase::Output))?;
    output
        .sync_all()
        .map_err(|_| fail(CommandCode::OutputUnavailable, Phase::Output))?;
    let parent = args
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::File::open(parent)
        .and_then(|d| d.sync_all())
        .map_err(|_| fail(CommandCode::OutputUnavailable, Phase::Output))?;
    let result = runtime.run(&invocation, &config, directory).await;
    let (report, exit) = match result {
        Ok(body) => {
            let exit = body.exit_code();
            let metrics = Metrics::of(
                invocation.kind,
                &body,
                u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            );
            (
                serde_json::json!({"schema_version":1, "operation":invocation.kind,
                "result":body, "exit_code":exit, "metrics":metrics,
                "application_rollout":"not_asserted", "collector_delivery":"pending_platform_certification",
                "hosted_apply":"disabled"}),
                exit,
            )
        }
        Err(error) => {
            let exit = error.current_context().exit_code();
            (command::refusal_json(error.current_context()), exit)
        }
    };
    Ok(deliver(report, exit, |bytes| {
        use std::io::Write as _;
        output
            .write_all(bytes)
            .and_then(|_| output.write_all(b"\n"))
            .and_then(|_| output.sync_all())
    }))
}

fn deliver(
    mut report: serde_json::Value,
    exit: u8,
    write: impl FnOnce(&[u8]) -> std::io::Result<()>,
) -> (serde_json::Value, u8) {
    let delivery = serde_json::to_vec(&report)
        .ok()
        .filter(|v| v.len() <= 4 * 1024 * 1024)
        .is_some_and(|bytes| write(&bytes).is_ok());
    if !delivery {
        let exit = if exit == 0 { 6 } else { exit };
        if let Some(envelope) = report.as_object_mut() {
            envelope.insert("report_delivery".to_owned(), serde_json::json!("failed"));
            envelope.insert("exit_code".to_owned(), serde_json::json!(exit));
        }
        return (report, exit);
    }
    (report, exit)
}

#[cfg(test)]
mod delivery_tests {
    #[test]
    fn failed_final_output_preserves_unknown_partial_and_applied_facts() {
        for (outcome, exit, expected) in [
            ("CommitUnknown", 5, 5),
            ("PartialApplied", 4, 4),
            ("AppliedUnverified", 6, 6),
            ("AppliedVerified", 0, 6),
            ("Refused", 3, 3),
        ] {
            let (report, actual) = super::deliver(
                serde_json::json!({"outcome":outcome,"committed":1}),
                exit,
                |_| Err(std::io::Error::other("synthetic-private-error")),
            );
            assert_eq!(actual, expected);
            assert_eq!(report["outcome"], outcome);
            assert_eq!(report["committed"], 1);
            assert_eq!(report["report_delivery"], "failed");
            assert!(!report.to_string().contains("synthetic-private-error"));
        }
    }
}
