//! Shared orchestration for certified adapters. All mutations converge on ApplyRuntime.
use std::time::Duration;

use super::{
    apply::{ApplyLimits, ApplyRuntime, Cancellation, PreparedApply},
    apply_contract::{
        checked, fail, ApplyCode, ApplyReader, ApplyResult, ApplyVerifier, CertifiedMutationTarget,
    },
    authority::{CertifiedExecutionTarget, ReleaseIntent},
    canonical_hash,
    coordination::{
        self, CoordinationClock, CoordinatorStore, ExecutionRequest, ExecutionSession, Operation,
        Owner, SafetyControls,
    },
    journal::{
        self, Capture, CertifiedJournalStore, ImmutableJournalStore, RecoveryIdentity,
        RestrictedJson, Retention, SecretGuard,
    },
    mutation::MutationWriter,
    operation::{Configuration, Invocation, Kind, ResultBody},
    plan,
    planner_types::{self, PlanIntent, ProjectionContract, VerifiedDesired},
    recovery::{RecoveryPlanning, RecoverySource, RestoreOrigin, RetainedRecoveryImage},
    types::CompleteInventory,
    Digest, HashDomain,
};

/// Observations supplied by a certified image/immutable-config adapter, not command JSON.
/// The executable digest and compiled feature closure must be independently matched by that
/// adapter to the platform's exact image digest. The production registry is intentionally empty.
pub struct ExecutionIdentity {
    pub(super) image: Digest,
    pub(super) executable: Digest,
    pub(super) build: Digest,
    pub(super) immutable_profile: Digest,
    pub(super) feature_keys: Vec<super::Name>,
}
/// Process observations cannot be deserialized from a claimed release manifest.
pub struct ObservedExecutable {
    pub(super) digest: Digest,
    pub(super) features: Vec<super::Name>,
}
impl ObservedExecutable {
    pub(super) fn capture(features: Vec<super::Name>) -> ApplyResult<Self> {
        use std::io::Read as _;

        use sha2::{Digest as _, Sha256};
        let file = checked(std::fs::File::open(checked(std::env::current_exe())?))?;
        let before = checked(file.metadata())?;
        if !before.is_file() || before.len() == 0 || before.len() > 2 * 1024 * 1024 * 1024 {
            return Err(fail(ApplyCode::InvalidInput));
        }
        let mut bounded = file.take(before.len() + 1);
        let mut hasher = Sha256::new();
        let bytes = checked(std::io::copy(&mut bounded, &mut hasher))?;
        if bytes != before.len() {
            return Err(fail(ApplyCode::ConcurrentChange));
        }
        let digest = checked(format!("sha256:{:x}", hasher.finalize()).try_into())?;
        Ok(Self { digest, features })
    }
}
impl ExecutionIdentity {
    pub fn from_registry(_: &super::Name) -> ApplyResult<Self> {
        Err(fail(ApplyCode::NotAuthoritative))
    }
    pub fn validate(
        &self,
        intent: &ReleaseIntent,
        desired: &VerifiedDesired,
        profile: &Digest,
        observed: &ObservedExecutable,
    ) -> ApplyResult<()> {
        let manifest = &desired.desired.input_manifest;
        let build = checked(planner_types::json(&manifest.build))?;
        let release = checked(build.get("release"))?;
        let artifact = checked(release.get("artifact"))?;
        let declared_features: Vec<super::Name> = checked(serde_json::from_str(
            checked(build.get("router_feature_keys"))?.raw(),
        ))?;
        let platforms: std::collections::BTreeMap<String, Digest> = checked(serde_json::from_str(
            checked(artifact.get("platform_digests"))?.raw(),
        ))?;
        if self.image != intent.executor_image
            || self.executable != observed.digest
            || self.build != intent.build
            || self.immutable_profile != *profile
            || self.feature_keys != observed.features
            || intent.feature_keys != observed.features
            || declared_features != observed.features
            || !platforms.values().any(|v| v == &self.image)
            || checked(artifact.get("kind"))?
                .string()
                .map_err(|_| fail(ApplyCode::NotAuthoritative))?
                != "image"
            || checked(release.get("application_revision"))?.raw()
                != checked(planner_types::json(&intent.source))?.raw()
            || checked(planner_types::hash(&build))? != intent.build
            || checked(canonical_hash(HashDomain::InputManifest, manifest))?
                != intent.input_manifest
            || manifest.mapping_hash != intent.policy
            || manifest.catalog_hash != intent.catalog
            || manifest.effective_settings_hash != intent.settings
            || desired.release_hash() != &intent.release
        {
            return Err(fail(ApplyCode::NotAuthoritative));
        }
        Ok(())
    }
}

pub struct Inputs<'a> {
    pub desired: &'a VerifiedDesired,
    pub bundle: RestrictedJson,
    pub profile_hash: Digest,
    pub observed: &'a ObservedExecutable,
    pub projection: &'a ProjectionContract,
    pub browser_reports: Vec<Vec<u8>>,
    pub identity: &'a ExecutionIdentity,
    pub retained_image: &'a RetainedRecoveryImage,
    pub retention: Retention,
}
pub struct Executor<'a, S, G, C, J, W, R, V> {
    pub target: &'a CertifiedExecutionTarget,
    pub mutation: &'a CertifiedMutationTarget,
    pub coordinator: &'a S,
    pub controls: &'a G,
    pub clock: &'a C,
    pub journal: &'a J,
    pub certificate: &'a CertifiedJournalStore,
    pub reader: &'a R,
    pub writer: &'a W,
    pub verifier: &'a V,
    pub guard: &'a SecretGuard,
    pub cancellation: &'a Cancellation,
    pub owner: Owner,
    pub limits: ApplyLimits,
}

/// Narrow status reader: cannot call coordinator, mutation, verification or journal append APIs.
pub async fn status<R: journal::JournalReader>(
    reader: &R,
    certificate: &CertifiedJournalStore,
    target: &super::types::TargetIdentity,
    run: &Digest,
) -> ApplyResult<ResultBody> {
    let recovered = journal::recover_run(reader, certificate, target, run, None)
        .await
        .map_err(|_| fail(ApplyCode::JournalUnavailable))?;
    Ok(ResultBody::Status {
        journal: recovered.report(),
    })
}

pub struct RestoreInputs<'a> {
    pub desired: &'a VerifiedDesired,
    pub projection: &'a ProjectionContract,
    pub mutation_contract: &'a CertifiedMutationTarget,
    pub guard: &'a SecretGuard,
    pub read_ms: u64,
}

/// Provisional inverse planning only needs read capabilities. It must not depend on a writer
/// credential, available executor image, barrier acquisition or collector access.
pub async fn restore_plan<R: ApplyReader, J: journal::JournalReader>(
    reader: &R,
    journal: &J,
    certificate: &CertifiedJournalStore,
    invocation: &Invocation,
    config: &Configuration,
    inputs: RestoreInputs<'_>,
) -> ApplyResult<ResultBody> {
    checked(invocation.validate(config))?;
    let planning = config
        .planning
        .as_ref()
        .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
    let target = inputs.mutation_contract;
    target.validate()?;
    let manifest = &inputs.desired.desired.input_manifest;
    if planning.intent.release != *inputs.desired.release_hash()
        || planning.intent.input_manifest
            != checked(canonical_hash(HashDomain::InputManifest, manifest))?
        || planning.intent.build != checked(planner_types::digest(&manifest.build))?
        || planning.intent.settings != manifest.effective_settings_hash
        || planning.intent.policy != manifest.mapping_hash
        || planning.intent.catalog != manifest.catalog_hash
    {
        return Err(fail(ApplyCode::NotAuthoritative));
    }
    if invocation.kind != Kind::RestorePlan
        || inputs.read_ms == 0
        || inputs.read_ms > 30_000
        || target.registration.target != config.target
        || certificate.contract_id() != &target.registration.journal_contract
        || inputs.projection.target != config.target
        || inputs.projection.id != planning.projection_contract
    {
        return Err(fail(ApplyCode::TargetMismatch));
    }
    let current = tokio::time::timeout(Duration::from_millis(inputs.read_ms), reader.complete())
        .await
        .map_err(|_| fail(ApplyCode::ReadIncomplete))??;
    target.inventory(&current)?;
    let run = invocation
        .source_run
        .as_ref()
        .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
    let recovered = journal::recover_run(journal, certificate, &config.target, run, None)
        .await
        .map_err(|e| {
            e.change_context(super::apply_contract::ApplyError {
                code: ApplyCode::JournalUnavailable,
            })
        })?;
    let observation =
        RecoverySource::from_recovered(recovered).observe(&current, target, inputs.guard)?;
    let input = || RecoveryPlanning {
        inventory: &current,
        desired: inputs.desired,
        ownership: &planning.ownership,
        prerequisites: &planning.prerequisites,
        intent: PlanIntent {
            authority: planning.intent.authority.clone(),
            release_hash: inputs.desired.release_hash().clone(),
            generation: planning.intent.generation,
        },
        contract: inputs.projection,
        limits: planning.limits.clone(),
    };
    let candidate = observation
        .restore_draft(input(), RestoreOrigin::RunBefore, inputs.guard)?
        .prepare(&observation, input())?;
    Ok(ResultBody::RestorePlan {
        recovery: observation.report().clone(),
        plan: Box::new(candidate.candidate().report().clone()),
        retained_prerequisites: candidate.retained_prerequisites().to_vec(),
    })
}
impl<
        S: CoordinatorStore,
        G: SafetyControls,
        C: CoordinationClock,
        J: ImmutableJournalStore,
        W: MutationWriter,
        R: ApplyReader,
        V: ApplyVerifier,
    > Executor<'_, S, G, C, J, W, R, V>
{
    fn registration(&self, invocation: &Invocation, config: &Configuration) -> ApplyResult<()> {
        invocation.validate(config).map_err(|error| {
            let code = match error.current_context().code {
                super::command::CommandCode::TargetMismatch => ApplyCode::TargetMismatch,
                super::command::CommandCode::NotAuthoritative => ApplyCode::NotAuthoritative,
                _ => ApplyCode::InvalidInput,
            };
            error.change_context(super::apply_contract::ApplyError { code })
        })?;
        self.mutation.validate()?;
        if checked(super::authority::hash(self.target.registration()))?
            != checked(super::authority::hash(&self.mutation.registration))?
            || self.target.registration().target != config.target
            || self.certificate.contract_id() != &self.target.registration().journal_contract
        {
            return Err(fail(ApplyCode::TargetMismatch));
        }
        Ok(())
    }
    async fn inventory(&self) -> ApplyResult<CompleteInventory> {
        self.limits.validate()?;
        let current = tokio::time::timeout(
            Duration::from_millis(self.limits.read_ms),
            self.reader.complete(),
        )
        .await
        .map_err(|_| fail(ApplyCode::ReadIncomplete))??;
        self.mutation.inventory(&current)?;
        Ok(current)
    }
    /// Status has no inventory read, lease, barrier, writer, approval or verification action.
    pub async fn status(
        &self,
        invocation: &Invocation,
        config: &Configuration,
    ) -> ApplyResult<ResultBody> {
        checked(invocation.validate(config))?;
        if self.target.registration().target != config.target
            || self.certificate.contract_id() != &self.target.registration().journal_contract
        {
            return Err(fail(ApplyCode::TargetMismatch));
        }
        if invocation.kind != Kind::Status {
            return Err(fail(ApplyCode::InvalidInput));
        }
        let run = invocation
            .source_run
            .as_ref()
            .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
        status(self.journal, self.certificate, &config.target, run).await
    }
    pub async fn execute(
        &self,
        invocation: &Invocation,
        config: &Configuration,
        inputs: Inputs<'_>,
    ) -> ApplyResult<ResultBody> {
        self.registration(invocation, config)?;
        if invocation.kind == Kind::RestorePlan {
            return restore_plan(
                self.reader,
                self.journal,
                self.certificate,
                invocation,
                config,
                RestoreInputs {
                    desired: inputs.desired,
                    projection: inputs.projection,
                    mutation_contract: self.mutation,
                    guard: self.guard,
                    read_ms: self.limits.read_ms,
                },
            )
            .await;
        }
        let planning = config
            .planning
            .as_ref()
            .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
        let intent = &planning.intent;
        checked(intent.validate(self.target.registration()))?;
        inputs.identity.validate(
            intent,
            inputs.desired,
            &inputs.profile_hash,
            inputs.observed,
        )?;
        if inputs.projection.target != config.target
            || inputs.projection.id != planning.projection_contract
            || inputs.retained_image.scope != self.mutation.scope
            || inputs.retained_image.retained_until_unix < inputs.retention.retain_until_unix
            || inputs.browser_reports.len() > 16
            || inputs
                .browser_reports
                .iter()
                .any(|r| r.len() > 4 * 1024 * 1024)
        {
            return Err(fail(ApplyCode::NotAuthoritative));
        }
        // A claimed intent must equal authenticated current authority, even for inverse planning.
        let current_intent =
            tokio::time::timeout(Duration::from_secs(10), self.controls.current_intent())
                .await
                .map_err(|_| fail(ApplyCode::NotAuthoritative))?
                .map_err(|_| fail(ApplyCode::NotAuthoritative))?;
        if &current_intent != intent {
            return Err(fail(ApplyCode::NotAuthoritative));
        }
        let operation = match invocation.kind {
            Kind::Apply => Operation::Apply,
            Kind::Resume => Operation::Resume,
            Kind::RestorePlan | Kind::RestoreApply => Operation::Restore,
            Kind::Status => return Err(fail(ApplyCode::InvalidInput)),
        };
        // Recovery evidence is read before takeover; it does not authorize takeover on its own.
        let prior = if invocation.source_run.is_some() {
            Some(
                tokio::time::timeout(Duration::from_secs(10), self.coordinator.read())
                    .await
                    .map_err(|_| fail(ApplyCode::NotAuthoritative))?
                    .map_err(|_| fail(ApplyCode::NotAuthoritative))?,
            )
        } else {
            None
        };
        let evidence = if let Some(active) = prior.as_ref().and_then(|p| p.state.active.as_ref()) {
            if invocation.source_run.as_ref() != Some(&active.owner.run) {
                return Err(fail(ApplyCode::ConcurrentChange));
            }
            Some(
                coordination::inspect_recovery(self.journal, self.certificate, active)
                    .await
                    .map_err(|_| fail(ApplyCode::JournalUnavailable))?,
            )
        } else {
            None
        };
        let session = if invocation.kind.mutates() {
            Some(super::apply::coordinate(
                ExecutionSession::acquire(
                    self.coordinator,
                    self.controls,
                    self.clock,
                    self.target,
                    ExecutionRequest {
                        owner: self.owner.clone(),
                        operation,
                        intent: intent.clone(),
                        plan: invocation
                            .plan_hash
                            .clone()
                            .ok_or_else(|| fail(ApplyCode::InvalidInput))?,
                    },
                    evidence.as_ref(),
                )
                .await,
            )?)
        } else {
            None
        };
        // Poll renewal during planning/recovery I/O too. Never drop a dispatched request on timeout.
        let work = async {
            let current = self.inventory().await?;
            let observation = if let Some(run) = &invocation.source_run {
                let recovered = journal::recover_run(
                    self.journal,
                    self.certificate,
                    &config.target,
                    run,
                    prior
                        .as_ref()
                        .and_then(|p| p.state.active.as_ref())
                        .and_then(|a| a.journal.as_ref()),
                )
                .await
                .map_err(|_| fail(ApplyCode::JournalUnavailable))?;
                Some(RecoverySource::from_recovered(recovered).observe(
                    &current,
                    self.mutation,
                    self.guard,
                )?)
            } else {
                None
            };
            let plan_intent = PlanIntent {
                authority: intent.authority.clone(),
                release_hash: intent.release.clone(),
                generation: intent.generation,
            };
            let recovery_inputs = || RecoveryPlanning {
                inventory: &current,
                desired: inputs.desired,
                ownership: &planning.ownership,
                prerequisites: &planning.prerequisites,
                intent: plan_intent.clone(),
                contract: inputs.projection,
                limits: planning.limits.clone(),
            };
            let recovery = match &observation {
                Some(observation) if operation == Operation::Restore => Some(
                    observation
                        .restore_draft(recovery_inputs(), RestoreOrigin::RunBefore, self.guard)?
                        .prepare(observation, recovery_inputs())?,
                ),
                Some(observation) => Some(observation.prepare_resume(recovery_inputs())?),
                None => None,
            };
            let now = self
                .clock
                .now_ms()
                .map_err(|_| fail(ApplyCode::NotAuthoritative))?
                / 1000;
            let reports: Vec<_> = inputs.browser_reports.iter().map(Vec::as_slice).collect();
            let accepted_recovery = recovery
                .map(|candidate| {
                    candidate.accept(inputs.projection, &reports, planning.approval.as_ref(), now)
                })
                .transpose()?;
            let accepted_apply = if accepted_recovery.is_none() {
                Some(checked(plan::accept(
                    checked(plan::prepare(
                        &current,
                        inputs.desired,
                        &planning.ownership,
                        &planning.prerequisites,
                        plan_intent,
                        inputs.projection,
                        planning.limits.clone(),
                    ))?,
                    inputs.projection,
                    &reports,
                    planning.approval.as_ref(),
                    now,
                ))?)
            } else {
                None
            };
            let accepted = accepted_recovery
                .as_ref()
                .map(|r| r.plan())
                .or(accepted_apply.as_ref())
                .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
            if accepted.report().plan_hash != invocation.plan_hash
                || accepted.report().inputs.bundle_hash != intent.bundle
            {
                return Err(fail(ApplyCode::NotAuthoritative));
            }
            let session = session
                .as_ref()
                .ok_or_else(|| fail(ApplyCode::NotAuthoritative))?;
            let authorization =
                if let (Some(observation), Some(accepted)) = (&observation, &accepted_recovery) {
                    Some(
                        observation
                            .record(
                                accepted,
                                session,
                                self.journal,
                                self.certificate,
                                inputs.retained_image,
                                &inputs.retention,
                                self.guard,
                            )
                            .await?,
                    )
                } else {
                    None
                };
            let baseline = authorization.as_ref().map(|a| a.baseline.clone());
            let capture = Capture {
                plan: accepted,
                inventory: &current,
                ownership: &planning.ownership,
                prerequisites: &planning.prerequisites,
                projection: inputs.projection,
                bundle: accepted_recovery
                    .as_ref()
                    .and_then(|r| r.bundle())
                    .cloned()
                    .unwrap_or(inputs.bundle),
                identity: RecoveryIdentity {
                    target: config.target.clone(),
                    endpoint_identity: self.target.registration().endpoint_identity.clone(),
                    coordinator: self.target.registration().coordinator.object.clone(),
                    executor_image: intent.executor_image.clone(),
                    retained_recovery_image: inputs.retained_image.image.clone(),
                },
                retention: inputs.retention,
                requests: vec![],
                unreadable_attributes: vec![],
                server_managed_attributes: self
                    .mutation
                    .context_audit
                    .iter()
                    .chain(&self.mutation.default_audit)
                    .chain(&self.mutation.dimension_audit)
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .map(|v| checked(v.clone().try_into()))
                    .collect::<ApplyResult<_>>()?,
            };
            let prepared = match authorization {
                Some(authorization) => PreparedApply::prepare_recovery(
                    capture,
                    self.mutation,
                    self.guard,
                    authorization,
                )?,
                None => PreparedApply::prepare(capture, self.mutation, self.guard)?,
            };
            let engine = ApplyRuntime {
                session,
                target: self.mutation,
                journal_store: self.journal,
                journal_certificate: self.certificate,
                guard: self.guard,
                writer: self.writer,
                reader: self.reader,
                verifier: self.verifier,
                cancellation: self.cancellation,
                limits: self.limits.clone(),
            };
            Ok(ResultBody::Execution {
                report: engine.execute_inner(prepared, baseline.as_ref()).await,
            })
        };
        let result = if let Some(session) = &session {
            tokio::pin!(work);
            tokio::select! { result = &mut work => result,
            _ = session.renew_until_stopped() => { session.stop(); work.await } }
        } else {
            work.await
        };
        if let Some(session) = &session {
            session.stop();
            if let Err(error) = result {
                let code = error.current_context().code;
                let (outcome, exit_code) =
                    super::apply_contract::classify(super::apply_contract::OutcomeFacts {
                        planned: 1,
                        dispatched: 0,
                        committed: 0,
                        rejected: 0,
                        unknown: 0,
                        preflight: false,
                        verified: false,
                        reason: Some(code),
                    });
                return Ok(ResultBody::Execution {
                    report: super::apply_contract::ApplyReport {
                        schema_version: 1,
                        run: self.owner.run.clone(),
                        intent_generation: intent.generation,
                        input_manifest: intent.input_manifest.clone(),
                        bundle: intent.bundle.clone(),
                        executor_image: intent.executor_image.clone(),
                        leases_acquired: 1,
                        target: checked(super::apply_contract::hash(self.target.registration()))?,
                        plan: session.execution().plan.clone(),
                        outcome,
                        exit_code,
                        dispatched: 0,
                        committed: vec![],
                        rejected: vec![],
                        unknown: vec![],
                        reason: Some(code),
                        recovery_artifact: None,
                        verification: None,
                        requests: vec![],
                        actions: std::collections::BTreeMap::new(),
                        evidence_scope: self.mutation.scope,
                        hosted_apply: "disabled",
                    },
                });
            }
        }
        result
    }
}
