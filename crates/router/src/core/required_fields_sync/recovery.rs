//! Observation-first recovery. No replay, CLI entry point, reset, implicit approval or hosted adapter.
use serde::Serialize;

#[path = "recovery_restore.rs"]
mod restoration;
pub use restoration::{RestorationDraft, RestoreOrigin};

use super::{
    apply_contract::{checked, fail, hash, ApplyCode, ApplyResult, CertifiedMutationTarget},
    apply_observation::{retained_images, RequestDisposition, World},
    authority::{self, RecoveryIntent},
    coordination::{
        CoordinationClock, CoordinatorStore, ExecutionSession, Operation, SafetyControls,
    },
    journal::{
        self, ArtifactReference, CertifiedJournalStore, ImmutableJournalStore, JournalReader,
        RecoveredRun, RestrictedJson, Retention, RunLocator, SecretGuard, UnresolvedRunRecord,
    },
    ownership::OwnershipManifest,
    plan::{self, PlanCandidate, ReadOnlyPlan},
    planner_types::{
        PlanIntent, PlanLimits, PlannedAction, ProjectionContract, SemanticApproval,
        VerifiedDesired,
    },
    preflight::PrerequisitePolicy,
    raw_json::Json,
    recovery_data::{self as data, Images},
    types::{CompleteInventory, EvidenceScope},
    Digest, Name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestState {
    NotDispatched,
    Before,
    After,
    Divergent,
}
#[derive(Debug, Clone, Serialize)]
pub struct RequestAssessment {
    pub request: Digest,
    pub state: RequestState,
    pub durable_prior_resolution: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct RecoveryReport {
    pub schema_version: u16,
    pub source_run: Digest,
    pub source_snapshot: Digest,
    pub first_adoption: Digest,
    pub journal_head: Digest,
    pub current_inventory: Digest,
    pub requests: Vec<RequestAssessment>,
    pub requires_executor_and_request_exclusion: bool,
    pub hosted_apply: &'static str,
}

/// No unchecked/deserializable constructor. Always scans durable storage without executor memory.
pub struct RecoverySource {
    run: RecoveredRun,
}
impl RecoverySource {
    pub async fn load<R: JournalReader>(
        reader: &R,
        certificate: &CertifiedJournalStore,
        locator: &RunLocator,
        anchor: Option<&UnresolvedRunRecord>,
    ) -> ApplyResult<Self> {
        Ok(Self {
            run: journal::recover(reader, certificate, locator, anchor)
                .await
                .map_err(|_| fail(ApplyCode::JournalUnavailable))?,
        })
    }
    pub fn observe(
        self,
        current: &CompleteInventory,
        target: &CertifiedMutationTarget,
        guard: &SecretGuard,
    ) -> ApplyResult<RecoveryObservation> {
        target.validate()?;
        target.inventory(current)?;
        let run = &self.run;
        let identity = &run.snapshot.data.identity;
        if identity.target != target.registration.target
            || identity.endpoint_identity != target.registration.endpoint_identity
            || identity.coordinator != target.registration.coordinator.object
            || run.report().store_contract != target.registration.journal_contract
            || run.unresolved.pending_requests.len() > 1
        {
            return Err(fail(ApplyCode::TargetMismatch));
        }
        let actions = data::actions(&run.snapshot)?;
        let compiled = super::mutation::compile(&actions, target, guard)?;
        if checked(serde_json::to_vec(&compiled))?
            != checked(serde_json::to_vec(run.snapshot.requests()))?
        {
            return Err(fail(ApplyCode::JournalUnavailable));
        }
        let contents = checked(Json::parse(run.snapshot.contents().bytes()))?;
        let initial = data::before(&run.snapshot, guard)?;
        let original_inventory = data::decode(checked(
            checked(contents.get("proof"))?.get("inventory_hash"),
        )?)?;
        Images::parse(&initial)?.validate(target, &original_inventory)?;
        let mut before = World::archived(&initial)?;
        let actual = World::read(current, target)?;
        let live_images = retained_images(current, guard)?;
        let mut assessments = Vec::new();
        let mut applied = Vec::new();
        let mut encountered_unresolved = false;
        for (ordinal, request) in run.snapshot.requests().iter().enumerate() {
            let request_hash = checked(request.hash())?;
            let expected = before.advance(&actions, &request.action_indices)?;
            let resolution = run.resolutions.iter().find(|r| r.request == request_hash);
            let (state, images) = if let Some(resolved) = resolution {
                if encountered_unresolved
                    || ordinal >= run.intended_requests
                    || resolved.contract != target.id
                {
                    return Err(fail(ApplyCode::JournalUnavailable));
                }
                Images::parse(&resolved.images)?.validate(target, &resolved.inventory)?;
                let saved = World::archived(&resolved.images)?;
                let committed = resolved.disposition == RequestDisposition::Committed;
                if !(if committed { &expected } else { &before }).matches(&saved, target)? {
                    return Err(fail(ApplyCode::JournalUnavailable));
                }
                before = saved;
                (
                    if committed {
                        RequestState::After
                    } else {
                        RequestState::Before
                    },
                    Some(resolved.images.clone()),
                )
            } else if run.unresolved.pending_requests.contains(&request_hash) {
                if encountered_unresolved || ordinal + 1 != run.intended_requests {
                    return Err(fail(ApplyCode::JournalUnavailable));
                }
                encountered_unresolved = true;
                let state = if expected.matches(&actual, target)? {
                    RequestState::After
                } else if before.matches(&actual, target)? {
                    RequestState::Before
                } else {
                    RequestState::Divergent
                };
                (state, Some(live_images.clone()))
            } else {
                if ordinal < run.intended_requests {
                    return Err(fail(ApplyCode::JournalUnavailable));
                }
                (RequestState::NotDispatched, None)
            };
            if state == RequestState::After {
                let images = images.ok_or_else(|| fail(ApplyCode::JournalUnavailable))?;
                for index in &request.action_indices {
                    applied.push((
                        actions
                            .get(*index)
                            .ok_or_else(|| fail(ApplyCode::InvalidInput))?
                            .clone(),
                        images.clone(),
                    ));
                }
            }
            assessments.push(RequestAssessment {
                request: request_hash,
                state,
                durable_prior_resolution: resolution.is_some(),
            });
        }
        let report = RecoveryReport {
            schema_version: 1,
            source_run: run.unresolved.locator.run().clone(),
            source_snapshot: run.unresolved.locator.artifact().clone(),
            first_adoption: run.unresolved.first_adoption.hash().clone(),
            journal_head: checked(authority::hash(&Some(&run.unresolved)))?,
            current_inventory: current.proof().inventory_hash.clone(),
            requests: assessments,
            requires_executor_and_request_exclusion: true,
            hosted_apply: "disabled",
        };
        Ok(RecoveryObservation {
            source: self,
            report,
            applied,
            live_images,
        })
    }
}
pub struct RecoveryObservation {
    source: RecoverySource,
    report: RecoveryReport,
    pub(super) applied: Vec<(PlannedAction, RestrictedJson)>,
    live_images: RestrictedJson,
}
impl RecoveryObservation {
    pub fn report(&self) -> &RecoveryReport {
        &self.report
    }
    pub fn intent(
        &self,
        readers: &std::collections::BTreeMap<Name, authority::ReaderBuild>,
    ) -> ApplyResult<RecoveryIntent> {
        Ok(RecoveryIntent {
            from_run: self.report.source_run.clone(),
            snapshot: self.report.source_snapshot.clone(),
            first_adoption: self.report.first_adoption.clone(),
            reader_builds: checked(authority::hash(readers))?,
        })
    }
    pub(super) fn current(&self, inventory: &CompleteInventory) -> ApplyResult<()> {
        if inventory.proof().target != self.source.run.snapshot.data.identity.target
            || inventory.proof().inventory_hash != self.report.current_inventory
        {
            return Err(fail(ApplyCode::ConcurrentChange));
        }
        if self
            .report
            .requests
            .iter()
            .any(|r| r.state == RequestState::Divergent)
        {
            return Err(fail(ApplyCode::CommitUnknown));
        }
        Ok(())
    }
    pub fn prepare_resume(&self, inputs: RecoveryPlanning<'_>) -> ApplyResult<RecoveryCandidate> {
        self.current(inputs.inventory)?;
        self.new_generation(&inputs.intent)?;
        let candidate = checked(plan::prepare(
            inputs.inventory,
            inputs.desired,
            inputs.ownership,
            inputs.prerequisites,
            inputs.intent,
            inputs.contract,
            inputs.limits,
        ))?;
        let residuals = self
            .applied
            .iter()
            .filter(|(action, _)| {
                matches!(
                    action.kind,
                    super::planner_types::ActionKind::CreateDefault
                        | super::planner_types::ActionKind::ExtendDimension
                )
            })
            .map(|(action, _)| checked(super::planner_types::digest(&checked(action.summary())?)))
            .collect::<ApplyResult<Vec<_>>>()?;
        self.bind(candidate, Operation::Resume, None, residuals)
    }
    pub(super) fn new_generation(&self, intent: &PlanIntent) -> ApplyResult<()> {
        let contents = checked(Json::parse(self.source.run.snapshot.contents().bytes()))?;
        let old = checked(checked(checked(contents.get("plan"))?.get("inputs"))?.get("intent"))?;
        if intent.generation <= checked(checked(old.get("generation"))?.unsigned())?
            || intent.authority != data::decode::<Name>(checked(old.get("authority"))?)?
        {
            return Err(fail(ApplyCode::NotAuthoritative));
        }
        Ok(())
    }
    pub(super) fn bind(
        &self,
        mut candidate: PlanCandidate,
        operation: Operation,
        bundle: Option<RestrictedJson>,
        residuals: Vec<Digest>,
    ) -> ApplyResult<RecoveryCandidate> {
        let binding = checked(super::planner_types::digest(&(
            &self.report,
            operation,
            &residuals,
        )))?;
        checked(plan::bind_recovery(&mut candidate, &binding))?;
        Ok(RecoveryCandidate {
            candidate,
            operation,
            binding,
            observation: hash(&self.report)?,
            bundle,
            residuals,
        })
    }
}
pub struct RecoveryPlanning<'a> {
    pub inventory: &'a CompleteInventory,
    pub desired: &'a VerifiedDesired,
    pub ownership: &'a OwnershipManifest,
    pub prerequisites: &'a PrerequisitePolicy,
    pub intent: PlanIntent,
    pub contract: &'a ProjectionContract,
    pub limits: PlanLimits,
}
pub struct RecoveryCandidate {
    candidate: PlanCandidate,
    operation: Operation,
    binding: Digest,
    observation: Digest,
    bundle: Option<RestrictedJson>,
    residuals: Vec<Digest>,
}
impl RecoveryCandidate {
    pub fn candidate(&self) -> &PlanCandidate {
        &self.candidate
    }
    pub fn retained_prerequisites(&self) -> &[Digest] {
        &self.residuals
    }
    pub fn bundle(&self) -> Option<&RestrictedJson> {
        self.bundle.as_ref()
    }
    pub fn accept(
        self,
        contract: &ProjectionContract,
        reports: &[&[u8]],
        approval: Option<&SemanticApproval>,
        now: u64,
    ) -> ApplyResult<AcceptedRecoveryPlan> {
        let plan = checked(plan::accept(
            self.candidate,
            contract,
            reports,
            approval,
            now,
        ))?;
        Ok(AcceptedRecoveryPlan {
            plan,
            operation: self.operation,
            binding: self.binding,
            observation: self.observation,
            bundle: self.bundle,
            residuals: self.residuals,
        })
    }
}
pub struct AcceptedRecoveryPlan {
    plan: ReadOnlyPlan,
    operation: Operation,
    binding: Digest,
    observation: Digest,
    bundle: Option<RestrictedJson>,
    residuals: Vec<Digest>,
}
impl AcceptedRecoveryPlan {
    pub fn plan(&self) -> &ReadOnlyPlan {
        &self.plan
    }
    pub fn bundle(&self) -> Option<&RestrictedJson> {
        self.bundle.as_ref()
    }
}

/// Platform registry must certify digest availability, access and retention. No user flag can
/// manufacture this evidence; the selected recovery image is not built/deployed by this module.
pub struct RetainedRecoveryImage {
    pub(super) image: Digest,
    pub(super) receipt: Digest,
    pub(super) scope: EvidenceScope,
    pub(super) retained_until_unix: u64,
}
impl RetainedRecoveryImage {
    pub fn from_registry(_: &Name) -> ApplyResult<Self> {
        Err(fail(ApplyCode::UnsupportedServerContract))
    }
}
/// Only exact source re-read + current exclusion + immutable decision read-back constructs this.
pub struct RecoveryAuthorization {
    pub(super) reference: ArtifactReference,
    pub(super) baseline: ArtifactReference,
    pub(super) plan: Digest,
    pub(super) inventory: Digest,
    pub(super) intent: Digest,
    pub(super) run: Digest,
    pub(super) operation: Operation,
    pub(super) image: Digest,
}
impl RecoveryObservation {
    #[allow(clippy::too_many_arguments)]
    pub async fn record<
        S: CoordinatorStore,
        G: SafetyControls,
        C: CoordinationClock,
        J: ImmutableJournalStore,
    >(
        &self,
        accepted: &AcceptedRecoveryPlan,
        session: &ExecutionSession<'_, S, G, C>,
        store: &J,
        certificate: &CertifiedJournalStore,
        image: &RetainedRecoveryImage,
        retention: &Retention,
        guard: &SecretGuard,
    ) -> ApplyResult<RecoveryAuthorization> {
        let work = async {
            let execution = session.execution();
            let intent = &execution.intent;
            let source = &self.source.run;
            let now = session
                .now_for_apply()
                .map_err(|_| fail(ApplyCode::LeaseLost))?
                / 1000;
            if accepted.observation != hash(&self.report)?
                || accepted.operation != execution.operation
                || accepted.plan.report().plan_hash.as_ref() != Some(&execution.plan)
                || intent.recovery.as_ref() != Some(&self.intent(&intent.readers)?)
                || image.image != source.snapshot.data.identity.retained_recovery_image
                || image.image != intent.executor_image
                || image.scope != accepted.plan.report().scope
                || image.retained_until_unix < retention.retain_until_unix
                || retention.created_at_unix > now
                || retention.retain_until_unix <= now
                || self
                    .report
                    .requests
                    .iter()
                    .any(|r| r.state == RequestState::Divergent)
                || certificate.contract_id() != &session.registration().journal_contract
            {
                return Err(fail(ApplyCode::NotAuthoritative));
            }
            checked(retention.validate())?;
            let recovered = journal::recover(
                store,
                certificate,
                &source.unresolved.locator,
                Some(&source.unresolved),
            )
            .await
            .map_err(|_| fail(ApplyCode::JournalUnavailable))?;
            if checked(authority::hash(&Some(&recovered.unresolved)))? != self.report.journal_head {
                return Err(fail(ApplyCode::CommitUnknown));
            }
            let exclusion = session
                .recovery_exclusion(&source.unresolved, &self.report.current_inventory)
                .await
                .map_err(|_| fail(ApplyCode::NotAuthoritative))?;
            let decision = checked(serde_json::to_vec(&serde_json::json!({
                "schema_version":1, "assessment":self.report, "accepted_plan":execution.plan,
                "binding":accepted.binding, "operation":execution.operation, "new_intent":intent,
                "exclusion":exclusion, "image_retention_receipt":image.receipt,
                "source":source.unresolved, "current_images":self.live_images,
                "retained_additive_prerequisites":accepted.residuals,
            })))?;
            let contents = checked(RestrictedJson::parse(&decision, guard))?;
            let reference = journal::persist_recovery(
                store,
                certificate,
                &intent.target,
                &execution.owner.run,
                &contents,
                retention,
            )
            .await
            .map_err(|_| fail(ApplyCode::JournalUnavailable))?;
            session
                .recovery_exclusion(&source.unresolved, &self.report.current_inventory)
                .await
                .map_err(|_| fail(ApplyCode::NotAuthoritative))?;
            Ok(RecoveryAuthorization {
                reference,
                baseline: source.unresolved.first_adoption.clone(),
                plan: execution.plan.clone(),
                inventory: self.report.current_inventory.clone(),
                intent: checked(authority::hash(intent))?,
                run: execution.owner.run.clone(),
                operation: execution.operation,
                image: image.image.clone(),
            })
        };
        tokio::pin!(work);
        let result = tokio::select! {
            result = &mut work => result,
            _ = session.renew_until_stopped() => { session.stop(); work.await },
        };
        if result.is_err() {
            session.stop();
        }
        result
    }
}
