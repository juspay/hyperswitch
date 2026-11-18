//! Guarded engine shared by command execution. No registered hosted activation or automatic replay.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use super::{
    apply_contract::*,
    apply_observation::{self, RequestDisposition, World},
    coordination::{CoordinationClock, CoordinatorStore, ExecutionSession, SafetyControls},
    journal::{
        self, ArtifactReference, Capture, CertifiedJournalStore, ImmutableJournalStore,
        Observation, PreparedSnapshot, RestrictedJson, RunJournal, SecretGuard, TransportOutcome,
    },
    mutation::{self, Dispatch, MutationWriter, WireReply},
    planner_types::PlannedAction,
    types::CompleteInventory,
    Digest,
};

/// No serde/unchecked constructor: every request is compiled from an accepted whole plan.
pub struct PreparedApply {
    snapshot: PreparedSnapshot,
    actions: Vec<PlannedAction>,
    before: World,
    inventory: Digest,
    plan: Digest,
    binding: Digest,
    approval_expiry: Option<u64>,
    recovery: Option<super::recovery::RecoveryAuthorization>,
}
impl PreparedApply {
    pub fn prepare_recovery(
        capture: Capture<'_>,
        target: &CertifiedMutationTarget,
        guard: &SecretGuard,
        authorization: super::recovery::RecoveryAuthorization,
    ) -> ApplyResult<Self> {
        if capture.plan.report().plan_hash.as_ref() != Some(&authorization.plan)
            || capture.inventory.proof().inventory_hash != authorization.inventory
            || &capture.retention != authorization.reference.retention()
            || capture.identity.executor_image != authorization.image
            || capture.identity.retained_recovery_image != authorization.image
        {
            return Err(fail(ApplyCode::NotAuthoritative));
        }
        let mut prepared = Self::prepare(capture, target, guard)?;
        prepared.snapshot.data.recovery = Some(authorization.reference.clone());
        prepared.snapshot.data.schema_version = 2;
        checked(prepared.snapshot.validate())?;
        prepared.recovery = Some(authorization);
        Ok(prepared)
    }
    pub fn prepare(
        mut capture: Capture<'_>,
        target: &CertifiedMutationTarget,
        guard: &SecretGuard,
    ) -> ApplyResult<Self> {
        target.validate()?;
        target.inventory(capture.inventory)?;
        let declared: std::collections::BTreeSet<_> = capture
            .server_managed_attributes
            .iter()
            .map(|n| n.as_str())
            .collect();
        let certified: std::collections::BTreeSet<_> = target
            .context_audit
            .iter()
            .chain(&target.default_audit)
            .chain(&target.dimension_audit)
            .map(String::as_str)
            .collect();
        if !capture.unreadable_attributes.is_empty()
            || declared != certified
            || declared.len() != capture.server_managed_attributes.len()
        {
            return Err(fail(ApplyCode::UnsupportedServerContract));
        }
        if !capture.requests.is_empty()
            || capture.identity.target != target.registration.target
            || capture.identity.endpoint_identity != target.registration.endpoint_identity
            || capture.identity.coordinator != target.registration.coordinator.object
            || capture.plan.report().scope != target.scope
        {
            return Err(fail(ApplyCode::TargetMismatch));
        }
        let before = World::read(capture.inventory, target)?;
        if apply_observation::retained_images(capture.inventory, guard)?
            .bytes()
            .len()
            > 1024 * 1024
        {
            return Err(fail(ApplyCode::LimitExceeded));
        }
        let actions = capture.plan.actions().to_vec();
        capture.requests = mutation::compile(&actions, target, guard)?;
        let mut state = before.clone();
        for request in &capture.requests {
            let after = state.advance(&actions, &request.action_indices)?;
            if state.matches(&after, target)? {
                return Err(fail(ApplyCode::InvalidInput));
            }
            state = after;
        }
        let plan = capture
            .plan
            .report()
            .plan_hash
            .clone()
            .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
        let inventory = capture.inventory.proof().inventory_hash.clone();
        let approval_expiry = capture
            .plan
            .accepted_approval
            .as_ref()
            .map(|a| a.expires_at_unix);
        let snapshot =
            PreparedSnapshot::capture(capture, guard).map_err(|_| fail(ApplyCode::InvalidInput))?;
        // Leave room for the original baseline and every bounded response/state event.
        if checked(serde_json::to_vec(&snapshot))?.len() > 32 * 1024 * 1024 {
            return Err(fail(ApplyCode::LimitExceeded));
        }
        Ok(Self {
            recovery: None,
            snapshot,
            actions,
            before,
            inventory,
            plan,
            binding: hash(&target.registration)?,
            approval_expiry,
        })
    }
    pub fn requests(&self) -> &[journal::PreparedRequest] {
        self.snapshot.requests()
    }
}

#[derive(Default)]
pub struct Cancellation(AtomicBool);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    fn check(&self) -> ApplyResult<()> {
        if self.0.load(Ordering::SeqCst) {
            Err(fail(ApplyCode::Cancelled))
        } else {
            Ok(())
        }
    }
    async fn wait(&self) {
        while !self.0.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
#[derive(Clone)]
pub struct ApplyLimits {
    pub read_attempts: u8,
    pub read_ms: u64,
    pub verification_ms: u64,
}
/// Reserve recovery-reader capacity for the original baseline, preparation, every exact
/// intent, bounded transport/resolution event, verifier proof and two terminal reports.
/// Oversized histories refuse before storage/SP writes, never halfway through known work.
pub(super) fn journal_capacity(
    snapshot_bytes: usize,
    baseline_bytes: usize,
    requests: &[journal::PreparedRequest],
) -> ApplyResult<()> {
    let mut total = snapshot_bytes
        .checked_add(baseline_bytes)
        .and_then(|n| n.checked_add(journal::MAX_EVENT + 512 * 1024))
        .ok_or_else(|| fail(ApplyCode::LimitExceeded))?;
    for request in requests {
        let bytes = checked(serde_json::to_vec(request))?.len();
        total = total
            .checked_add(bytes)
            .and_then(|n| n.checked_add(journal::MAX_EVENT + 64 * 1024))
            .ok_or_else(|| fail(ApplyCode::LimitExceeded))?;
    }
    if total > journal::MAX_RECOVERY_BYTES {
        return Err(fail(ApplyCode::LimitExceeded));
    }
    Ok(())
}
impl Default for ApplyLimits {
    fn default() -> Self {
        Self {
            read_attempts: 3,
            read_ms: 5000,
            verification_ms: 40 * 60 * 1000,
        }
    }
}
impl ApplyLimits {
    pub(super) fn validate(&self) -> ApplyResult<()> {
        if !(1..=3).contains(&self.read_attempts)
            || self.read_ms == 0
            || self.read_ms > 5000
            || self.verification_ms == 0
            || self.verification_ms > 40 * 60 * 1000
        {
            return Err(fail(ApplyCode::InvalidInput));
        }
        Ok(())
    }
}
pub struct ApplyRuntime<'a, S, G, C, J, W, R, V> {
    pub session: &'a ExecutionSession<'a, S, G, C>,
    pub target: &'a CertifiedMutationTarget,
    pub journal_store: &'a J,
    pub journal_certificate: &'a CertifiedJournalStore,
    pub guard: &'a SecretGuard,
    pub writer: &'a W,
    pub reader: &'a R,
    pub verifier: &'a V,
    pub cancellation: &'a Cancellation,
    pub limits: ApplyLimits,
}
pub(super) fn coordinate<T>(result: super::authority::CoordinationResult<T>) -> ApplyResult<T> {
    use super::authority::CoordinationCode as C;
    result.map_err(|e| {
        let code = match e.current_context().code {
            C::NotAuthoritative
            | C::StaleIntent
            | C::RolloutIncomplete
            | C::BarrierRequired
            | C::MembershipChanged => ApplyCode::NotAuthoritative,
            C::TargetMismatch => ApplyCode::TargetMismatch,
            _ => ApplyCode::LeaseLost,
        };
        e.change_context(ApplyError { code })
    })
}
fn journaled<T>(result: journal::JournalResult<T>) -> ApplyResult<T> {
    result.map_err(|_| fail(ApplyCode::JournalUnavailable))
}
impl<
        'a,
        S: CoordinatorStore,
        G: SafetyControls,
        C: CoordinationClock,
        J: ImmutableJournalStore,
        W: MutationWriter,
        R: ApplyReader,
        V: ApplyVerifier,
    > ApplyRuntime<'a, S, G, C, J, W, R, V>
{
    async fn inventory(&self) -> ApplyResult<CompleteInventory> {
        for attempt in 0..self.limits.read_attempts {
            let result = tokio::time::timeout(
                Duration::from_millis(self.limits.read_ms),
                self.reader.complete(),
            )
            .await;
            if let Ok(Ok(inventory)) = result {
                self.target.inventory(&inventory)?;
                return Ok(inventory);
            }
            if attempt + 1 < self.limits.read_attempts {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
        Err(fail(ApplyCode::ReadIncomplete))
    }
    async fn gate(&self, approval_expiry: Option<u64>) -> ApplyResult<()> {
        if approval_expiry.is_some_and(|t| {
            self.session
                .now_for_apply()
                .map_or(true, |now| t <= now / 1000)
        }) {
            return Err(fail(ApplyCode::NotAuthoritative));
        }
        self.cancellation.check()?;
        coordinate(self.session.check_authority().await)
    }
    /// Heartbeat is independently polled during all storage/read/verification waits. Dropping
    /// this future stops local dispatch; the durable reservation survives without a final report.
    pub async fn execute(
        &self,
        prepared: PreparedApply,
        baseline: Option<&ArtifactReference>,
    ) -> ApplyReport {
        struct Stop<'a, S: CoordinatorStore, G: SafetyControls, C: CoordinationClock>(
            &'a ExecutionSession<'a, S, G, C>,
        );
        impl<S: CoordinatorStore, G: SafetyControls, C: CoordinationClock> Drop for Stop<'_, S, G, C> {
            fn drop(&mut self) {
                self.0.stop();
            }
        }
        let _stop = Stop(self.session);
        let work = self.execute_inner(prepared, baseline);
        tokio::pin!(work);
        tokio::select! {
            report = &mut work => report,
            _ = self.session.renew_until_stopped() => {
                self.session.stop();
                // Do not drop an in-flight request and label it refused. Its bounded wait and
                // direct observation still run; no further mutation can pass the stopped gate.
                work.await
            }
        }
    }
    // The command orchestrator renews across planning AND execution; it must not run a second
    // competing renewal loop. Other callers use execute(), which owns renewal itself.
    pub(super) async fn execute_inner(
        &self,
        prepared: PreparedApply,
        baseline: Option<&ArtifactReference>,
    ) -> ApplyReport {
        let count = prepared.requests().len();
        let approval_expiry = prepared.approval_expiry;
        let mut report = ApplyReport {
            schema_version: 1,
            run: self.session.execution().owner.run.clone(),
            intent_generation: self.session.execution().intent.generation,
            input_manifest: self.session.execution().intent.input_manifest.clone(),
            bundle: self.session.execution().intent.bundle.clone(),
            executor_image: self.session.execution().intent.executor_image.clone(),
            leases_acquired: 1,
            target: prepared.binding.clone(),
            plan: prepared.plan.clone(),
            outcome: ApplyOutcome::Refused,
            exit_code: 2,
            dispatched: 0,
            committed: vec![],
            rejected: vec![],
            unknown: vec![],
            reason: None,
            recovery_artifact: None,
            verification: None,
            requests: vec![],
            evidence_scope: self.target.scope,
            actions: prepared.actions.iter().fold(
                std::collections::BTreeMap::new(),
                |mut counts, action| {
                    *counts.entry(action.kind.clone()).or_insert(0) += 1;
                    counts
                },
            ),
            hosted_apply: "disabled",
        };
        let mut preflight = false;
        let mut verified = false;
        let mut journal = None;
        let result: ApplyResult<()> = async {
            self.limits.validate()?;
            self.target.validate()?;
            let execution = self.session.execution();
            match &prepared.recovery {
                None if execution.operation == super::coordination::Operation::Apply
                    && execution.intent.recovery.is_none() => (),
                Some(recovery) if execution.operation == recovery.operation
                    && execution.operation != super::coordination::Operation::Apply
                    && execution.intent.recovery.is_some()
                    && execution.owner.run == recovery.run
                    && checked(super::authority::hash(&execution.intent))? == recovery.intent
                    && baseline == Some(&recovery.baseline) => (),
                _ => return Err(fail(ApplyCode::NotAuthoritative)),
            }
            let snapshot_bytes = checked(serde_json::to_vec(&prepared.snapshot))?.len()
                .checked_add(prepared.recovery.as_ref().map_or(0, |r| r.reference.bytes()))
                .ok_or_else(|| fail(ApplyCode::LimitExceeded))?;
            journal_capacity(
                snapshot_bytes,
                baseline.map_or(snapshot_bytes, ArtifactReference::bytes),
                prepared.requests(),
            )?;
            if self.journal_certificate.contract_id() != &self.target.registration.journal_contract {
                return Err(fail(ApplyCode::JournalUnavailable));
            }
            if prepared.binding != hash(self.session.registration())?
                || self.session.execution().plan != prepared.plan
                || approval_expiry.is_some_and(|t| {
                    self.session
                        .now_for_apply()
                        .map_or(true, |now| t <= now / 1000)
                })
            {
                return Err(fail(ApplyCode::NotAuthoritative));
            }
            self.gate(approval_expiry).await?;
            let inventory = self.inventory().await?;
            if inventory.proof().inventory_hash != prepared.inventory {
                return Err(fail(ApplyCode::ConcurrentChange));
            }
            let mut before_version = inventory.proof().version.clone();
            let mut before = prepared.before;
            let requests = prepared.snapshot.requests().to_vec();
            let (new_journal, _) = journaled(
                RunJournal::persist_prepared(
                    self.journal_store,
                    self.journal_certificate,
                    self.guard,
                    self.session.execution().owner.run.clone(),
                    prepared.snapshot,
                    baseline,
                )
                .await,
            )?;
            report.recovery_artifact = Some(new_journal.artifact_hash().clone());
            journal = Some(new_journal);
            let run = journal
                .as_mut()
                .ok_or_else(|| fail(ApplyCode::JournalUnavailable))?;
            coordinate(
                self.session
                    .bind_journal(&journaled(run.checkpoint())?)
                    .await,
            )?;
            preflight = true;
            let mut observed_inventory = inventory.proof().inventory_hash.clone();
            for (ordinal, request) in requests.iter().enumerate() {
                self.gate(approval_expiry).await?;
                // Every prerequisite and the sole context bulk gets a fresh complete re-read.
                let live = self.inventory().await?;
                let live_world = World::read(&live, self.target)?;
                if live.proof().inventory_hash != observed_inventory
                    || !before.matches(&live_world, self.target)?
                {
                    return Err(fail(ApplyCode::ConcurrentChange));
                }
                let after = before.advance(&prepared.actions, &request.action_indices)?;
                let intent = journaled(run.record_intent(ordinal).await)?;
                coordinate(
                    self.session
                        .reserve_request(&journaled(run.checkpoint())?, &intent)
                        .await,
                )?;
                self.gate(approval_expiry).await?;
                let request_hash = checked(request.hash())?;
                coordinate(self.session.recheck_before_dispatch(&request_hash).await)?;
                // The journal reservation already survives a dropped executor. Count dispatch
                // only once the transport future is polled; cancellation before that is a
                // no-write refusal. Neither request bytes nor action order are rebuilt.
                let mut dispatched = false;
                let started = tokio::time::Instant::now();
                let reply = tokio::select! {
                    biased;
                    () = self.cancellation.wait() => WireReply::unknown(TransportOutcome::ExecutorLost),
                    result = tokio::time::timeout(Duration::from_millis(self.target.request_ms),async {
                        dispatched = true;
                        self.writer.send(Dispatch {intent:&intent,target:self.target}).await
                    }) =>
                        result.unwrap_or_else(|_|WireReply::unknown(TransportOutcome::Timeout)),
                };
                if !dispatched {
                    return Err(fail(ApplyCode::Cancelled));
                }
                report.dispatched += 1;
                report.unknown.push(request_hash.clone());
                report.requests.push(RequestReport {
                    request: request_hash.clone(),
                    kind: match request.action_indices.first().and_then(|i| prepared.actions.get(*i)).map(|a| &a.kind) {
                        Some(super::planner_types::ActionKind::CreateDefault) => "create_default",
                        Some(super::planner_types::ActionKind::ExtendDimension) => "extend_dimension",
                        _ => "context_transaction",
                    },
                    phase: "transport", transport_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    status: reply.status, outcome: "unknown",
                });
                journaled(
                    run.record_observation(Observation::Transport {
                        request: request_hash.clone(),
                        status: reply.status,
                        response_body_hash: reply.body_hash,
                        version: reply.version.clone(),
                        request_id: reply.request_id,
                        outcome: reply.outcome,
                    })
                    .await,
                )?;
                let observed = self.inventory().await?;
                let evidence = apply_observation::resolution(
                    apply_observation::RequestObservation {
                        request,
                        before: &before,
                        after: &after,
                        before_version: &before_version,
                        status: reply.status.filter(|_| reply.valid_response),
                        response_version: &reply.version,
                    },
                    &observed,
                    self.target,
                    self.guard,
                )?;
                // Report what the complete direct observation establishes even if persisting
                // that proof fails. The coordinator still retains its pending request until
                // a durable resolution receipt exists; recovery must independently re-observe.
                report.unknown.retain(|r| r != &request_hash);
                match evidence.disposition {
                    RequestDisposition::Committed => report.committed.push(request_hash.clone()),
                    RequestDisposition::RejectedNoChange => report.rejected.push(request_hash),
                }
                if let Some(observation) = report.requests.last_mut() {
                    observation.phase = "observed";
                    observation.outcome = match evidence.disposition {
                        RequestDisposition::Committed => "committed",
                        RequestDisposition::RejectedNoChange => "rejected_no_change",
                    };
                }
                let resolved = journaled(run.record_resolution(&evidence).await)?;
                coordinate(
                    self.session
                        .resolve_request(&journaled(run.checkpoint())?, &resolved)
                        .await,
                )?;
                if evidence.disposition == RequestDisposition::RejectedNoChange {
                    return Err(fail(ApplyCode::RequestRejected));
                }
                before = World::read(&observed, self.target)?;
                before_version = observed.proof().version.clone();
                observed_inventory = observed.proof().inventory_hash.clone();
                // Known post-commit failure is not permission to proceed to another request.
                if !reply.valid_response || !reply.status.is_some_and(|s| (200..300).contains(&s)) {
                    return Err(fail(ApplyCode::VerificationFailed));
                }
            }
            self.gate(approval_expiry).await?;
            let binding = VerificationBinding {
                target: self.target.registration.target.clone(),
                plan: report.plan.clone(),
                inventory: observed_inventory,
                release: self.session.execution().intent.release.clone(),
            };
            let proof = tokio::time::timeout(
                Duration::from_millis(self.limits.verification_ms),
                self.verifier.verify(&binding),
            )
            .await
            .map_err(|_| fail(ApplyCode::VerificationFailed))?
            .map_err(|_| fail(ApplyCode::VerificationFailed))?;
            if proof.binding != hash(&binding)? || proof.scope != self.target.scope {
                return Err(fail(ApplyCode::VerificationFailed));
            }
            self.gate(approval_expiry).await?;
            journaled(run.record_run_report(proof.evidence.clone()).await)?;
            report.verification = Some(checked(proof.evidence.hash())?);
            verified = true;
            Ok(())
        }.await;
        report.reason = result.err().map(|e| e.current_context().code);
        (report.outcome, report.exit_code) = classify(OutcomeFacts {
            planned: count,
            dispatched: report.dispatched,
            committed: report.committed.len(),
            rejected: report.rejected.len(),
            unknown: report.unknown.len(),
            preflight,
            verified,
            reason: report.reason,
        });
        if let Some(run) = journal.as_mut() {
            let persisted = match serde_json::to_vec(&report)
                .ok()
                .and_then(|bytes| RestrictedJson::parse(&bytes, self.guard).ok())
            {
                Some(raw) => run.record_run_report(raw).await.is_ok(),
                None => false,
            };
            if !persisted {
                report.reason = Some(ApplyCode::JournalUnavailable);
                (report.outcome, report.exit_code) = classify(OutcomeFacts {
                    planned: count,
                    dispatched: report.dispatched,
                    committed: report.committed.len(),
                    rejected: report.rejected.len(),
                    unknown: report.unknown.len(),
                    preflight,
                    verified: false,
                    reason: report.reason,
                });
            }
        }
        // Authority can be lost while persisting the final report. Never return a stale
        // verified result; append the correction when storage still permits it.
        if verified {
            if let Err(error) = self.gate(approval_expiry).await {
                report.reason = Some(error.current_context().code);
                (report.outcome, report.exit_code) = classify(OutcomeFacts {
                    planned: count,
                    dispatched: report.dispatched,
                    committed: report.committed.len(),
                    rejected: report.rejected.len(),
                    unknown: report.unknown.len(),
                    preflight,
                    verified: false,
                    reason: report.reason,
                });
                if let Some(run) = journal.as_mut() {
                    if let Ok(bytes) = serde_json::to_vec(&report) {
                        if let Ok(raw) = RestrictedJson::parse(&bytes, self.guard) {
                            let _ = run.record_run_report(raw).await;
                        }
                    }
                }
            }
        }
        // Holds/active run remain until Step 11/12 certified terminal verification or recovery
        // handoff. No automatic release/clear/restore is implemented by this disabled engine.
        report
    }
}
