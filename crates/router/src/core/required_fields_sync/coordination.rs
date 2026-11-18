//! Owner-checked coordination shared by future apply/resume/restore. No Superposition writes.
//! Lease and non-expiring active record share one CAS object; release never deletes it.
use std::{
    future::Future,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use super::{
    authority::*,
    journal::{
        self, CertifiedJournalStore, JournalCheckpoint, JournalReader, RecordedIntent,
        ResolvedIntent, UnresolvedRunRecord,
    },
    Digest, Name,
};

pub const LEASE_MS: u64 = 60_000;
pub const RENEW_MS: u64 = 10_000;
pub const DISPATCH_MARGIN_MS: u64 = 15_000;
pub const MAX_STATE_BYTES: usize = 65536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Apply,
    Resume,
    Restore,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub job_uid: Name,
    pub run: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseOwnership {
    pub owner: Owner,
    pub renewed_at_ms: u64,
    pub expires_at_ms: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPhase {
    Preparing,
    Prepared,
    RequestInFlight,
    Unresolved,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveRun {
    pub owner: Owner,
    pub operation: Operation,
    pub intent: IntentStamp,
    pub plan: Digest,
    pub phase: RunPhase,
    pub journal: Option<UnresolvedRunRecord>,
    pub request_in_flight: Option<Digest>,
    pub gate: Option<Digest>,
    pub membership: Option<Digest>,
    pub barrier: Option<Digest>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinationState {
    pub schema_version: u16,
    pub scope: Digest,
    pub target: super::types::TargetIdentity,
    pub highest_intent: Option<IntentStamp>,
    pub lease: Option<LeaseOwnership>,
    pub active: Option<ActiveRun>,
    /// Recovery predecessors are never erased by lease expiry/release or Job cleanup.
    pub predecessors: Vec<ActiveRun>,
}
impl CoordinationState {
    pub fn initial(target: &TargetRegistration) -> CoordinationResult<Self> {
        target.validate()?;
        Ok(Self {
            schema_version: 1,
            scope: target.scope_hash()?,
            target: target.target.clone(),
            highest_intent: None,
            lease: None,
            active: None,
            predecessors: vec![],
        })
    }
    pub(super) fn validate(&self, target: &TargetRegistration) -> CoordinationResult<()> {
        if self.schema_version != 1
            || self.scope != target.scope_hash()?
            || self.target != target.target
        {
            return Err(refuse(CoordinationCode::TargetMismatch));
        }
        if self.predecessors.len() > 8 || encode(self)?.len() > MAX_STATE_BYTES {
            return Err(refuse(CoordinationCode::LimitExceeded));
        }
        if let Some(active) = &self.active {
            if self.highest_intent.as_ref() != Some(&active.intent)
                || (active.phase == RunPhase::RequestInFlight && active.request_in_flight.is_none())
                || ((matches!(active.phase, RunPhase::Prepared | RunPhase::RequestInFlight)
                    || active.request_in_flight.is_some())
                    && active.journal.is_none())
                || self
                    .lease
                    .as_ref()
                    .is_some_and(|lease| lease.owner != active.owner)
                || active
                    .journal
                    .as_ref()
                    .is_some_and(|record| !record.blocks_new_execution)
            {
                return Err(refuse(CoordinationCode::MalformedResponse));
            }
        } else if self.lease.is_some() || !self.predecessors.is_empty() {
            return Err(refuse(CoordinationCode::MalformedResponse));
        }
        if self.lease.as_ref().is_some_and(|lease| {
            lease.expires_at_ms.checked_sub(lease.renewed_at_ms) != Some(LEASE_MS)
        }) {
            return Err(refuse(CoordinationCode::MalformedResponse));
        }
        Ok(())
    }
}
/// The wire image is retained by adapters to preserve unknown Kubernetes metadata on PUT.
pub struct ObservedState {
    pub version: Name,
    pub state: CoordinationState,
    pub(super) wire: Option<journal::RestrictedJson>,
}
pub trait CoordinatorStore: Sync {
    fn read(&self) -> impl Future<Output = CoordinationResult<ObservedState>> + Send;
    fn compare_exchange(
        &self,
        observed: &ObservedState,
        next: &CoordinationState,
    ) -> impl Future<Output = CoordinationResult<ObservedState>> + Send;
}
/// Implementations must come from certified platform enforcement, not healthy-pod sampling.
/// No real implementation is selected until actual chart/routing/identity controls are proved.
pub trait SafetyControls: Sync {
    fn current_intent(&self) -> impl Future<Output = CoordinationResult<ReleaseIntent>> + Send;
    fn establish(
        &self,
        owner: &Owner,
        previous: Option<&ActiveRun>,
    ) -> impl Future<Output = CoordinationResult<SafetyObservation>> + Send;
    fn observe(
        &self,
        owner: &Owner,
    ) -> impl Future<Output = CoordinationResult<SafetyObservation>> + Send;
}
pub struct UnavailableSafetyControls;
impl SafetyControls for UnavailableSafetyControls {
    async fn current_intent(&self) -> CoordinationResult<ReleaseIntent> {
        Err(refuse(CoordinationCode::CertificationPending))
    }
    async fn establish(
        &self,
        _: &Owner,
        _: Option<&ActiveRun>,
    ) -> CoordinationResult<SafetyObservation> {
        Err(refuse(CoordinationCode::BarrierRequired))
    }
    async fn observe(&self, _: &Owner) -> CoordinationResult<SafetyObservation> {
        Err(refuse(CoordinationCode::BarrierRequired))
    }
}
/// Production clock adapters must use the repository's instrumented clock seam. No ambient clock
/// is read here; backward steps stop the session and platform skew bounds require certification.
pub trait CoordinationClock: Sync {
    fn now_ms(&self) -> CoordinationResult<u64>;
}
pub struct ExecutionRequest {
    pub owner: Owner,
    pub operation: Operation,
    pub intent: ReleaseIntent,
    pub plan: Digest,
}
pub struct RecoveryEvidence {
    previous: Owner,
    journal: Option<UnresolvedRunRecord>,
    head: Digest,
}
/// Reader-issued evidence, not a caller's successful boolean or a user-supplied head hash.
pub async fn inspect_recovery<R: JournalReader>(
    reader: &R,
    certificate: &CertifiedJournalStore,
    active: &ActiveRun,
) -> CoordinationResult<RecoveryEvidence> {
    let journal = if let Some(anchor) = &active.journal {
        let recovered = journal::recover(reader, certificate, &anchor.locator, Some(anchor))
            .await
            .map_err(|_| refuse(CoordinationCode::JournalMismatch))?;
        Some(recovered.unresolved)
    } else {
        // A voluntarily released pre-preparation claim has no journal yet. It remains blocked
        // until the same executor/request exclusion proof as any other takeover is supplied.
        if !matches!(active.phase, RunPhase::Preparing | RunPhase::Unresolved)
            || active.request_in_flight.is_some()
        {
            return Err(refuse(CoordinationCode::JournalMismatch));
        }
        None
    };
    Ok(RecoveryEvidence {
        previous: active.owner.clone(),
        head: hash(&journal)?,
        journal,
    })
}
impl RecoveryEvidence {
    pub fn head(&self) -> &Digest {
        &self.head
    }
}

async fn bounded<T>(future: impl Future<Output = CoordinationResult<T>>) -> CoordinationResult<T> {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .map_err(|_| refuse(CoordinationCode::UnknownOutcome))?
        .map_err(|error| refuse(error.current_context().code))
}
async fn read<S: CoordinatorStore>(
    store: &S,
    target: &TargetRegistration,
) -> CoordinationResult<ObservedState> {
    let observed = bounded(store.read()).await?;
    observed.state.validate(target)?;
    Ok(observed)
}
async fn replace<S: CoordinatorStore>(
    store: &S,
    target: &TargetRegistration,
    before: &ObservedState,
    next: &CoordinationState,
) -> CoordinationResult<ObservedState> {
    next.validate(target)?;
    let written = bounded(store.compare_exchange(before, next)).await?;
    written.state.validate(target)?;
    if written.version == before.version || encode(&written.state)? != encode(next)? {
        return Err(refuse(CoordinationCode::UnknownOutcome));
    }
    let checked = read(store, target).await?;
    if checked.version != written.version || encode(&checked.state)? != encode(next)? {
        return Err(refuse(CoordinationCode::UnknownOutcome));
    }
    Ok(checked)
}
fn lease(owner: &Owner, now: u64) -> CoordinationResult<LeaseOwnership> {
    Ok(LeaseOwnership {
        owner: owner.clone(),
        renewed_at_ms: now,
        expires_at_ms: now
            .checked_add(LEASE_MS)
            .ok_or_else(|| refuse(CoordinationCode::ClockInvalid))?,
    })
}

/// Shared handle: a renewal task can run independently of slow inventory/journal/verification.
/// An error/cancelled write poisons local dispatch; the durable active record remains blocking.
pub struct ExecutionSession<'a, S, G, C> {
    store: &'a S,
    controls: &'a G,
    clock: &'a C,
    target: &'a CertifiedExecutionTarget,
    request: ExecutionRequest,
    stopped: AtomicBool,
    last_clock: AtomicU64,
    gate: Digest,
    membership: Digest,
    barrier: Digest,
    mutation_busy: AtomicBool,
    predecessor_exclusion: Option<PreviousExecutorExclusion>,
}
impl<'a, S: CoordinatorStore, G: SafetyControls, C: CoordinationClock>
    ExecutionSession<'a, S, G, C>
{
    pub async fn acquire(
        store: &'a S,
        controls: &'a G,
        clock: &'a C,
        target: &'a CertifiedExecutionTarget,
        request: ExecutionRequest,
        recovery: Option<&RecoveryEvidence>,
    ) -> CoordinationResult<Self> {
        let registration = &target.registration;
        registration.validate()?;
        request.intent.validate(registration)?;
        let current = bounded(controls.current_intent()).await?;
        if current != request.intent {
            return Err(refuse(CoordinationCode::NotAuthoritative));
        }
        let observed = read(store, registration).await?;
        let stamp = IntentStamp::of(&current)?;
        if let Some(highest) = &observed.state.highest_intent {
            highest.accepts(&stamp)?;
        }
        let mut next = observed.state.clone();
        let now = clock.now_ms()?;
        let held_safety = if let Some(previous) = &observed.state.active {
            // Even a released/expired lease cannot bypass unresolved work.
            let proof = recovery.ok_or_else(|| refuse(CoordinationCode::RecoveryRequired))?;
            if request.operation == Operation::Apply
                || proof.previous != previous.owner
                || previous.owner == request.owner
                || proof
                    .journal
                    .as_ref()
                    .is_some_and(|record| !record.blocks_new_execution)
            {
                return Err(refuse(CoordinationCode::RecoveryRequired));
            }
            let safety = bounded(controls.establish(&request.owner, Some(previous))).await?;
            validate_safety(
                &safety,
                registration,
                &request.intent,
                &request.owner.job_uid,
                &request.owner.run,
                clock.now_ms()?,
                DISPATCH_MARGIN_MS,
            )?;
            let exclusion = safety
                .barrier
                .predecessor
                .as_ref()
                .ok_or_else(|| refuse(CoordinationCode::OldRequestNotExcluded))?;
            if exclusion.run != previous.owner.run
                || exclusion.owner != previous.owner.job_uid
                || !exclusion.revoked_and_stopped
                || !exclusion.accepted_requests_drained
                || !exclusion.direct_state_observed
                || exclusion.journal_head != proof.head
            {
                return Err(refuse(CoordinationCode::OldRequestNotExcluded));
            }
            let mut retained = previous.clone();
            retained.journal = proof.journal.clone();
            retained.phase = RunPhase::Unresolved;
            next.predecessors.push(retained);
            Some(safety)
        } else {
            if request.operation == Operation::Resume
                || recovery.is_some()
                || observed.state.lease.is_some()
            {
                return Err(refuse(CoordinationCode::RecoveryRequired));
            }
            None
        };
        // One CAS atomically claims lease + durable active record + monotonic intent high-water.
        next.highest_intent = Some(stamp.clone());
        next.lease = Some(lease(&request.owner, clock.now_ms()?)?);
        next.active = Some(ActiveRun {
            owner: request.owner.clone(),
            operation: request.operation,
            intent: stamp,
            plan: request.plan.clone(),
            phase: RunPhase::Preparing,
            journal: None,
            request_in_flight: None,
            gate: None,
            membership: None,
            barrier: None,
        });
        let claimed = replace(store, registration, &observed, &next).await?;
        let safety = match held_safety {
            Some(value) => value,
            None => bounded(controls.establish(&request.owner, None)).await?,
        };
        let after = clock.now_ms()?;
        if after < now {
            return Err(refuse(CoordinationCode::ClockInvalid));
        }
        validate_safety(
            &safety,
            registration,
            &request.intent,
            &request.owner.job_uid,
            &request.owner.run,
            after,
            DISPATCH_MARGIN_MS,
        )?;
        let mut next = claimed.state.clone();
        let active = next
            .active
            .as_mut()
            .ok_or_else(|| refuse(CoordinationCode::LeaseLost))?;
        active.gate = Some(safety.intent_gate.clone());
        active.membership = Some(safety.membership.clone());
        active.barrier = Some(safety.barrier.id.clone());
        // Barrier acquisition can be slow: do not return an already expired lease.
        require_owner(&next, &request.owner, after, DISPATCH_MARGIN_MS)?;
        replace(store, registration, &claimed, &next).await?;
        Ok(Self {
            store,
            controls,
            clock,
            target,
            request,
            stopped: AtomicBool::new(false),
            last_clock: AtomicU64::new(after),
            gate: safety.intent_gate,
            membership: safety.membership,
            barrier: safety.barrier.id,
            mutation_busy: AtomicBool::new(false),
            predecessor_exclusion: safety.barrier.predecessor,
        })
    }
    fn now(&self) -> CoordinationResult<u64> {
        let now = self.clock.now_ms()?;
        if now < self.last_clock.fetch_max(now, Ordering::SeqCst) {
            self.stop();
            return Err(refuse(CoordinationCode::ClockInvalid));
        }
        Ok(now)
    }
    fn running(&self) -> CoordinationResult<()> {
        if self.stopped.load(Ordering::SeqCst) {
            return Err(refuse(CoordinationCode::LeaseLost));
        }
        Ok(())
    }
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
    pub(super) fn execution(&self) -> &ExecutionRequest {
        &self.request
    }
    pub(super) fn now_for_apply(&self) -> CoordinationResult<u64> {
        self.now()
    }
    pub(super) fn registration(&self) -> &TargetRegistration {
        &self.target.registration
    }
    pub async fn check_authority(&self) -> CoordinationResult<()> {
        let result = self.checked(DISPATCH_MARGIN_MS).await.map(|_| ());
        if result.is_err() {
            self.stop();
        }
        result
    }
    /// A fresh exclusion receipt must bind both the scanned immutable head and this complete
    /// observation. Lease expiry, Job death and a pre-write re-read are not substitutes.
    pub(super) async fn recovery_exclusion(
        &self,
        source: &UnresolvedRunRecord,
        inventory: &Digest,
    ) -> CoordinationResult<PreviousExecutorExclusion> {
        let state = self.checked(DISPATCH_MARGIN_MS).await?;
        if self.request.operation == Operation::Apply {
            return Err(refuse(CoordinationCode::RecoveryRequired));
        }
        let previous = state
            .state
            .predecessors
            .last()
            .ok_or_else(|| refuse(CoordinationCode::RecoveryRequired))?;
        let expected = previous
            .journal
            .as_ref()
            .ok_or_else(|| refuse(CoordinationCode::JournalMismatch))?;
        if encode(expected)? != encode(source)?
            || self.request.intent.generation <= previous.intent.generation
        {
            return Err(refuse(CoordinationCode::JournalMismatch));
        }
        let safety = bounded(self.controls.observe(&self.request.owner)).await?;
        validate_safety(
            &safety,
            &self.target.registration,
            &self.request.intent,
            &self.request.owner.job_uid,
            &self.request.owner.run,
            self.now()?,
            DISPATCH_MARGIN_MS,
        )?;
        let proof = safety
            .barrier
            .predecessor
            .ok_or_else(|| refuse(CoordinationCode::OldRequestNotExcluded))?;
        if safety.intent_gate != self.gate
            || safety.membership != self.membership
            || safety.barrier.id != self.barrier
            || proof.run != previous.owner.run
            || proof.owner != previous.owner.job_uid
            || !proof.revoked_and_stopped
            || !proof.accepted_requests_drained
            || !proof.direct_state_observed
            || proof.journal_head != hash(&Some(source))?
            || proof.observed_inventory != *inventory
        {
            return Err(refuse(CoordinationCode::OldRequestNotExcluded));
        }
        self.check_authority().await?;
        Ok(proof)
    }
    pub async fn resolve_request(
        &self,
        checkpoint: &JournalCheckpoint,
        resolved: &ResolvedIntent,
    ) -> CoordinationResult<()> {
        let result = async {
            let _guard = self.exclusive()?;
            let observed = self.checked(DISPATCH_MARGIN_MS).await?;
            checkpoint
                .matches(
                    &self.target.registration,
                    &self.request.owner.run,
                    &self.request.intent,
                    &self.request.plan,
                    self.target.synthetic,
                )
                .map_err(|_| refuse(CoordinationCode::JournalMismatch))?;
            let mut next = observed.state.clone();
            let active = next
                .active
                .as_mut()
                .ok_or_else(|| refuse(CoordinationCode::LeaseLost))?;
            if active.phase != RunPhase::RequestInFlight
                || active.request_in_flight.as_ref() != Some(resolved.request_hash())
            {
                return Err(refuse(CoordinationCode::JournalMismatch));
            }
            checkpoint
                .resolves(
                    active
                        .journal
                        .as_ref()
                        .ok_or_else(|| refuse(CoordinationCode::JournalMismatch))?,
                    resolved,
                )
                .map_err(|_| refuse(CoordinationCode::JournalMismatch))?;
            active.journal = Some(checkpoint.record.clone());
            active.request_in_flight = None;
            active.phase = RunPhase::Prepared;
            self.change(&observed, &next).await
        }
        .await;
        if result.is_err() {
            self.stop();
        }
        result
    }
    async fn checked(&self, margin: u64) -> CoordinationResult<ObservedState> {
        self.running()?;
        let observed = read(self.store, &self.target.registration).await?;
        require_owner(&observed.state, &self.request.owner, self.now()?, margin)?;
        let safety = bounded(self.controls.observe(&self.request.owner)).await?;
        validate_safety(
            &safety,
            &self.target.registration,
            &self.request.intent,
            &self.request.owner.job_uid,
            &self.request.owner.run,
            self.now()?,
            margin,
        )?;
        if safety.intent_gate != self.gate
            || safety.membership != self.membership
            || safety.barrier.id != self.barrier
            || encode(&safety.barrier.predecessor)? != encode(&self.predecessor_exclusion)?
        {
            return Err(refuse(CoordinationCode::MembershipChanged));
        }
        if observed.state.highest_intent.as_ref() != Some(&IntentStamp::of(&self.request.intent)?) {
            return Err(refuse(CoordinationCode::StaleIntent));
        }
        // A slow control read or a suspended process can consume the lease after the first
        // check. Fresh rollout evidence does not renew our ownership.
        require_owner(&observed.state, &self.request.owner, self.now()?, margin)?;
        self.running()?;
        Ok(observed)
    }
    async fn change(
        &self,
        observed: &ObservedState,
        next: &CoordinationState,
    ) -> CoordinationResult<()> {
        // Cancellation while CAS/read-back is pending must not leave a reusable local permission.
        let mut guard = WriteGuard {
            stopped: &self.stopped,
            completed: false,
        };
        replace(self.store, &self.target.registration, observed, next).await?;
        self.running()?;
        guard.completed = true;
        Ok(())
    }
    pub async fn renew(&self) -> CoordinationResult<()> {
        let _guard = self.exclusive()?;
        let result = async {
            let observed = self.checked(1).await?;
            let mut next = observed.state.clone();
            next.lease = Some(lease(&self.request.owner, self.now()?)?);
            self.change(&observed, &next).await
        }
        .await;
        if result.is_err() {
            self.stop();
        }
        result
    }
    /// Intended to be polled/spawned by the same one-shot executor, not a separate daemon.
    pub async fn renew_until_stopped(&self) -> CoordinationResult<()> {
        loop {
            tokio::time::sleep(Duration::from_millis(RENEW_MS)).await;
            loop {
                self.running()?;
                match self.renew().await {
                    Ok(()) => break,
                    Err(error)
                        if error.current_context().code == CoordinationCode::Conflict
                            && !self.stopped.load(Ordering::SeqCst) =>
                    {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }
    fn exclusive(&self) -> CoordinationResult<BusyGuard<'_>> {
        self.mutation_busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| refuse(CoordinationCode::Conflict))?;
        Ok(BusyGuard(&self.mutation_busy))
    }
    pub async fn bind_journal(&self, checkpoint: &JournalCheckpoint) -> CoordinationResult<()> {
        let result = async {
            let _guard = self.exclusive()?;
            let observed = self.checked(DISPATCH_MARGIN_MS).await?;
            checkpoint
                .matches(
                    &self.target.registration,
                    &self.request.owner.run,
                    &self.request.intent,
                    &self.request.plan,
                    self.target.synthetic,
                )
                .map_err(|_| refuse(CoordinationCode::JournalMismatch))?;
            let mut next = observed.state.clone();
            let active = next
                .active
                .as_mut()
                .ok_or_else(|| refuse(CoordinationCode::LeaseLost))?;
            if active.phase != RunPhase::Preparing || checkpoint.record.last_sequence.is_some() {
                return Err(refuse(CoordinationCode::JournalMismatch));
            }
            active.journal = Some(checkpoint.record.clone());
            active.phase = RunPhase::Prepared;
            self.change(&observed, &next).await
        }
        .await;
        if result.is_err() {
            self.stop();
        }
        result
    }
    /// Durable reservation BEFORE a future dispatch. No reusable permission is returned. The
    /// Step 10 caller must call recheck immediately before its single bounded send after any pause.
    pub async fn reserve_request(
        &self,
        checkpoint: &JournalCheckpoint,
        intent: &RecordedIntent,
    ) -> CoordinationResult<()> {
        let result = async {
            let _guard = self.exclusive()?;
            let observed = self.checked(DISPATCH_MARGIN_MS).await?;
            checkpoint
                .matches(
                    &self.target.registration,
                    &self.request.owner.run,
                    &self.request.intent,
                    &self.request.plan,
                    self.target.synthetic,
                )
                .map_err(|_| refuse(CoordinationCode::JournalMismatch))?;
            checkpoint
                .matches_intent(intent)
                .map_err(|_| refuse(CoordinationCode::JournalMismatch))?;
            let mut next = observed.state.clone();
            let active = next
                .active
                .as_mut()
                .ok_or_else(|| refuse(CoordinationCode::LeaseLost))?;
            if active.phase != RunPhase::Prepared || active.request_in_flight.is_some() {
                return Err(refuse(CoordinationCode::RecoveryRequired));
            }
            let previous = active
                .journal
                .as_ref()
                .ok_or_else(|| refuse(CoordinationCode::JournalMismatch))?;
            checkpoint
                .follows(previous)
                .map_err(|_| refuse(CoordinationCode::JournalMismatch))?;
            active.journal = Some(checkpoint.record.clone());
            active.request_in_flight = Some(
                intent
                    .request()
                    .hash()
                    .map_err(|_| refuse(CoordinationCode::JournalMismatch))?,
            );
            active.phase = RunPhase::RequestInFlight;
            self.change(&observed, &next).await
        }
        .await;
        if result.is_err() {
            self.stop();
        }
        result
    }
    pub async fn recheck_before_dispatch(&self, request: &Digest) -> CoordinationResult<()> {
        let result = async {
            let state = self.checked(DISPATCH_MARGIN_MS).await?;
            if state
                .state
                .active
                .as_ref()
                .and_then(|a| a.request_in_flight.as_ref())
                != Some(request)
            {
                return Err(refuse(CoordinationCode::JournalMismatch));
            }
            // No action occurs here. A server-side barrier/exclusion contract remains mandatory.
            Ok(())
        }
        .await;
        if result.is_err() {
            self.stop();
        }
        result
    }
    /// Conditional release retains the unresolved record and external barrier. No delete or
    /// automatic resolution/hold release exists; V1–V4/restore completion is later work.
    pub async fn release_to_recovery(&self) -> CoordinationResult<()> {
        self.stop();
        let _guard = self.exclusive()?;
        let observed = read(self.store, &self.target.registration).await?;
        let held = observed
            .state
            .lease
            .as_ref()
            .ok_or_else(|| refuse(CoordinationCode::LeaseLost))?;
        if held.owner != self.request.owner {
            return Err(refuse(CoordinationCode::LeaseLost));
        }
        let mut next = observed.state.clone();
        next.lease = None;
        next.active
            .as_mut()
            .ok_or_else(|| refuse(CoordinationCode::LeaseLost))?
            .phase = RunPhase::Unresolved;
        replace(self.store, &self.target.registration, &observed, &next).await?;
        Ok(())
    }
}
struct BusyGuard<'a>(&'a AtomicBool);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
struct WriteGuard<'a> {
    stopped: &'a AtomicBool,
    completed: bool,
}
impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.stopped.store(true, Ordering::SeqCst);
        }
    }
}
fn require_owner(
    state: &CoordinationState,
    owner: &Owner,
    now: u64,
    margin: u64,
) -> CoordinationResult<()> {
    let lease = state
        .lease
        .as_ref()
        .ok_or_else(|| refuse(CoordinationCode::LeaseLost))?;
    if lease.owner != *owner
        || state.active.as_ref().is_none_or(|run| run.owner != *owner)
        || lease.renewed_at_ms > now
        || lease
            .expires_at_ms
            .checked_sub(now)
            .is_none_or(|left| left < margin)
    {
        return Err(refuse(CoordinationCode::LeaseLost));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CoordinationReport {
    pub target_hash: Digest,
    pub intent_generation: Option<u64>,
    pub active_run: Option<Digest>,
    pub unresolved: bool,
    pub request_unknown: bool,
    pub predecessors: usize,
    pub hosted_apply: &'static str,
}
impl CoordinationState {
    pub fn report(&self) -> CoordinationReport {
        CoordinationReport {
            target_hash: self.scope.clone(),
            intent_generation: self.highest_intent.as_ref().map(|s| s.generation),
            active_run: self.active.as_ref().map(|a| a.owner.run.clone()),
            unresolved: self.active.is_some(),
            request_unknown: self
                .active
                .as_ref()
                .is_some_and(|a| a.request_in_flight.is_some()),
            predecessors: self.predecessors.len(),
            hosted_apply: "disabled",
        }
    }
}

#[cfg(test)]
#[path = "coordination_tests.rs"]
pub(super) mod tests;
