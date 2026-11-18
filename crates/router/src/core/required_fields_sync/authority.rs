//! Target/release and complete rollout contracts. No ambient discovery or deployment action.
use std::collections::{BTreeMap, BTreeSet};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{
    raw_json::{fingerprint, Json},
    types::TargetIdentity,
    Digest, Name, Revision,
};

pub type CoordinationResult<T> = error_stack::Result<T, CoordinationError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationCode {
    CertificationPending,
    InvalidInput,
    LimitExceeded,
    TargetMismatch,
    NotAuthoritative,
    StaleIntent,
    RolloutIncomplete,
    MembershipChanged,
    BarrierRequired,
    OldRequestNotExcluded,
    Conflict,
    LeaseLost,
    RecoveryRequired,
    UnknownOutcome,
    PermissionDenied,
    MalformedResponse,
    JournalMismatch,
    VerificationPending,
    ClockInvalid,
}
#[derive(Debug, thiserror::Error)]
#[error("required-fields coordination refused: {code:?}")]
pub struct CoordinationError {
    pub code: CoordinationCode,
}
pub(super) fn refuse(code: CoordinationCode) -> error_stack::Report<CoordinationError> {
    CoordinationError { code }.into()
}
pub(super) fn encode<T: Serialize>(value: &T) -> CoordinationResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| refuse(CoordinationCode::InvalidInput))
}
pub(super) fn hash<T: Serialize>(value: &T) -> CoordinationResult<Digest> {
    fingerprint("coordination-v1", &encode(value)?)
        .map_err(|_| refuse(CoordinationCode::InvalidInput))
}
pub fn parse_contract<T: DeserializeOwned>(bytes: &[u8]) -> CoordinationResult<T> {
    if bytes.len() > 65536 {
        return Err(refuse(CoordinationCode::LimitExceeded));
    }
    Json::parse(bytes).map_err(|_| refuse(CoordinationCode::InvalidInput))?;
    serde_json::from_slice(bytes).map_err(|_| refuse(CoordinationCode::InvalidInput))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinatorIdentity {
    pub cluster: Digest,
    pub namespace: Name,
    pub object: Name,
    pub uid: Name,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadKind {
    Deployment,
    Rollout,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReaderRegistration {
    pub region: Name,
    pub cluster: Digest,
    pub namespace: Name,
    pub workload: Name,
    pub uid: Name,
    pub kind: WorkloadKind,
    /// Stable/canary Services, EndpointSlices and ingress/mesh sources selected by the adapter.
    pub traffic_resources: Vec<Name>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRegistration {
    pub schema_version: u16,
    pub target: TargetIdentity,
    /// Physical backend identity, not an endpoint hostname or regional release label.
    pub backing_instance: Digest,
    pub endpoint_identity: Digest,
    pub coordinator: CoordinatorIdentity,
    pub authority: Name,
    pub issuer: Name,
    pub server_contract: Digest,
    pub journal_contract: Digest,
    pub controls_contract: Digest,
    pub readers: BTreeMap<Name, ReaderRegistration>,
}
impl TargetRegistration {
    /// Excludes release/region/alias/generation so none can create a competing regional lock.
    pub fn scope_hash(&self) -> CoordinationResult<Digest> {
        hash(&(
            &self.backing_instance,
            &self.target.organisation,
            &self.target.workspace,
        ))
    }
    pub fn validate(&self) -> CoordinationResult<()> {
        if self.schema_version != 1
            || self.target.instance_generation == 0
            || self.readers.is_empty()
            || self.readers.len() > 32
            || !self.readers.contains_key(&self.authority)
            || !dns_label(self.coordinator.namespace.as_str())
            || !dns_label(self.coordinator.object.as_str())
        {
            return Err(refuse(CoordinationCode::InvalidInput));
        }
        let mut workloads = BTreeSet::new();
        let mut workload_uids = BTreeSet::new();
        for reader in self.readers.values() {
            if reader.traffic_resources.is_empty()
                || reader.traffic_resources.len() > 16
                || reader
                    .traffic_resources
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != reader.traffic_resources.len()
                || !workloads.insert(encode(&(
                    &reader.cluster,
                    &reader.namespace,
                    &reader.workload,
                ))?)
                || !workload_uids.insert(encode(&(&reader.cluster, &reader.uid))?)
            {
                return Err(refuse(CoordinationCode::InvalidInput));
            }
        }
        Ok(())
    }
}
pub fn validate_registry(targets: &[TargetRegistration]) -> CoordinationResult<()> {
    if targets.is_empty() || targets.len() > 64 {
        return Err(refuse(CoordinationCode::InvalidInput));
    }
    let mut scopes = BTreeSet::new();
    let mut coordinators = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for target in targets {
        target.validate()?;
        if !scopes.insert(encode(&target.scope_hash()?)?)
            || !coordinators.insert(encode(&(
                &target.coordinator.cluster,
                &target.coordinator.namespace,
                &target.coordinator.object,
            ))?)
            || !ids.insert(target.target.target_id.clone())
        {
            return Err(refuse(CoordinationCode::TargetMismatch));
        }
    }
    Ok(())
}
pub(super) fn dns_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
/// Candidate JSON cannot create this capability. No Q3/Q4/Q6 production registration exists.
pub struct CertifiedExecutionTarget {
    pub(super) registration: TargetRegistration,
    pub(super) synthetic: bool,
}
impl CertifiedExecutionTarget {
    pub fn from_registry(_id: &Name) -> CoordinationResult<Self> {
        Err(refuse(CoordinationCode::CertificationPending))
    }
    pub fn registration(&self) -> &TargetRegistration {
        &self.registration
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReaderBuild {
    pub image: Digest,
    pub build: Digest,
    pub inputs: Digest,
    /// Certified compatibility with before AND after semantics, including retained SDK sessions.
    pub compatibility: Digest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseIntent {
    pub schema_version: u16,
    pub target: TargetIdentity,
    pub issuer: Name,
    pub authority: Name,
    pub generation: u64,
    pub source: Revision,
    pub application_uid: Name,
    pub git_revision: Revision,
    pub executor_image: Digest,
    pub build: Digest,
    pub feature_keys: Vec<Name>,
    pub input_manifest: Digest,
    pub bundle: Digest,
    pub policy: Digest,
    pub catalog: Digest,
    pub settings: Digest,
    pub release: Digest,
    pub readers: BTreeMap<Name, ReaderBuild>,
    /// Issued by the same authority adapter as the release intent, never a CLI override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<RecoveryIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryIntent {
    pub from_run: Digest,
    pub snapshot: Digest,
    pub first_adoption: Digest,
    pub reader_builds: Digest,
}
impl ReleaseIntent {
    pub fn validate(&self, target: &TargetRegistration) -> CoordinationResult<()> {
        let features: BTreeSet<_> = self.feature_keys.iter().map(Name::as_str).collect();
        if self.schema_version != 1
            || self.generation == 0
            || self.target != target.target
            || self.issuer != target.issuer
            || self.authority != target.authority
            || self.readers.keys().collect::<Vec<_>>() != target.readers.keys().collect::<Vec<_>>()
            || features.len() != self.feature_keys.len()
            || self.feature_keys.len() > 64
            || !features.contains("V1")
            || !features.contains("REQUIRED_FIELDS_SYNC")
            || features.contains("V2")
        {
            return Err(refuse(CoordinationCode::NotAuthoritative));
        }
        let writer = self
            .readers
            .get(&self.authority)
            .ok_or_else(|| refuse(CoordinationCode::NotAuthoritative))?;
        if let Some(recovery) = &self.recovery {
            // A retained executor may differ from a pre-feature serving binary. Discovery,
            // build/input attestations and compatibility still cover every current reader.
            if recovery.reader_builds != hash(&self.readers)? {
                return Err(refuse(CoordinationCode::NotAuthoritative));
            }
        } else if writer.image != self.executor_image
            || writer.build != self.build
            || writer.inputs != self.input_manifest
        {
            return Err(refuse(CoordinationCode::NotAuthoritative));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentStamp {
    pub generation: u64,
    pub hash: Digest,
}
impl IntentStamp {
    pub fn of(intent: &ReleaseIntent) -> CoordinationResult<Self> {
        Ok(Self {
            generation: intent.generation,
            hash: hash(intent)?,
        })
    }
    pub fn accepts(&self, next: &Self) -> CoordinationResult<()> {
        if next.generation < self.generation
            || (next.generation == self.generation && next.hash != self.hash)
        {
            return Err(refuse(CoordinationCode::StaleIntent));
        }
        Ok(())
    }
}
/// Controller publication rule: identical re-publication is harmless; rollback is a newer intent.
pub fn validate_intent_advance(
    previous: &ReleaseIntent,
    next: &ReleaseIntent,
    target: &TargetRegistration,
) -> CoordinationResult<()> {
    previous.validate(target)?;
    next.validate(target)?;
    IntentStamp::of(previous)?.accepts(&IntentStamp::of(next)?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PodObservation {
    pub uid: Name,
    pub image: Digest,
    pub build: Digest,
    pub inputs: Digest,
    pub ready: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadObservation {
    pub registration: ReaderRegistration,
    pub spec_generation: u64,
    pub observed_generation: u64,
    pub desired_replicas: u32,
    pub updated_replicas: u32,
    pub ready_replicas: u32,
    pub old_replicas: u32,
    pub paused: bool,
    pub degraded: bool,
    pub progressing: bool,
    pub promoted: bool,
    pub old_connections_drained: bool,
    pub controller_resource_version: Name,
    pub traffic_resource_versions: BTreeMap<Name, Name>,
    pub compatibility: Digest,
    pub pods: Vec<PodObservation>,
    pub traffic_pod_uids: Vec<Name>,
}
/// Produced only through the selected certified platform adapter, never CLI-supplied gate JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyObservation {
    pub target: TargetIdentity,
    pub coordinator: CoordinatorIdentity,
    pub controls_contract: Digest,
    pub intent: ReleaseIntent,
    pub intent_gate: Digest,
    pub membership: Digest,
    pub observed_at_ms: u64,
    pub valid_until_ms: u64,
    pub complete_reader_discovery: bool,
    pub deployment_gate_held: bool,
    pub workloads: BTreeMap<Name, WorkloadObservation>,
    pub barrier: BarrierObservation,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterPath {
    Ui,
    Api,
    Router,
    Seeder,
    Automation,
    Administration,
}
pub fn required_writer_paths() -> BTreeSet<WriterPath> {
    [
        WriterPath::Ui,
        WriterPath::Api,
        WriterPath::Router,
        WriterPath::Seeder,
        WriterPath::Automation,
        WriterPath::Administration,
    ]
    .into_iter()
    .collect()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarrierObservation {
    pub id: Digest,
    pub owner: Name,
    pub run: Digest,
    pub scope: Digest,
    pub enforcement: Digest,
    pub covered_paths: Vec<WriterPath>,
    pub enforced: bool,
    pub privileged_edit_detected: bool,
    /// Revocation/drain covers requests already accepted by the server, not just dead Jobs.
    pub predecessor: Option<PreviousExecutorExclusion>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviousExecutorExclusion {
    pub run: Digest,
    pub owner: Name,
    pub revoked_and_stopped: bool,
    pub accepted_requests_drained: bool,
    pub direct_state_observed: bool,
    pub journal_head: Digest,
    pub observed_inventory: Digest,
    pub receipt: Digest,
}

pub(super) fn validate_safety(
    observation: &SafetyObservation,
    target: &TargetRegistration,
    intent: &ReleaseIntent,
    owner: &Name,
    run: &Digest,
    now: u64,
    margin: u64,
) -> CoordinationResult<()> {
    if observation.target != target.target
        || observation.coordinator != target.coordinator
        || observation.controls_contract != target.controls_contract
    {
        return Err(refuse(CoordinationCode::TargetMismatch));
    }
    intent.validate(target)?;
    if observation.intent != *intent {
        return Err(refuse(CoordinationCode::NotAuthoritative));
    }
    if observation.observed_at_ms > now
        || now.saturating_sub(observation.observed_at_ms) > 5000
        || observation
            .valid_until_ms
            .checked_sub(now)
            .is_none_or(|left| left < margin)
        || !observation.complete_reader_discovery
        || !observation.deployment_gate_held
        || observation.workloads.keys().collect::<Vec<_>>()
            != target.readers.keys().collect::<Vec<_>>()
        || observation.membership != hash(&observation.workloads)?
    {
        return Err(refuse(CoordinationCode::RolloutIncomplete));
    }
    for (id, registered) in &target.readers {
        let workload = observation
            .workloads
            .get(id)
            .ok_or_else(|| refuse(CoordinationCode::RolloutIncomplete))?;
        let expected = intent
            .readers
            .get(id)
            .ok_or_else(|| refuse(CoordinationCode::RolloutIncomplete))?;
        if workload.registration != *registered
            || workload.spec_generation == 0
            || workload.spec_generation != workload.observed_generation
            || workload.paused
            || workload.degraded
            || workload.progressing
            || !workload.promoted
            || !workload.old_connections_drained
            || workload.old_replicas != 0
            || workload.desired_replicas == 0
            || workload.desired_replicas > 256
            || workload.updated_replicas != workload.desired_replicas
            || workload.ready_replicas != workload.desired_replicas
            || workload.pods.len()
                != usize::try_from(workload.desired_replicas)
                    .map_err(|_| refuse(CoordinationCode::LimitExceeded))?
            || workload.compatibility != expected.compatibility
            || workload
                .traffic_resource_versions
                .keys()
                .collect::<BTreeSet<_>>()
                != registered.traffic_resources.iter().collect::<BTreeSet<_>>()
        {
            return Err(refuse(CoordinationCode::RolloutIncomplete));
        }
        let pods: BTreeSet<_> = workload.pods.iter().map(|pod| &pod.uid).collect();
        let traffic: BTreeSet<_> = workload.traffic_pod_uids.iter().collect();
        if pods.len() != workload.pods.len()
            || traffic.len() != workload.traffic_pod_uids.len()
            || pods != traffic
            || workload.pods.iter().any(|pod| {
                !pod.ready
                    || pod.image != expected.image
                    || pod.inputs != expected.inputs
                    || pod.build != expected.build
            })
        {
            return Err(refuse(CoordinationCode::RolloutIncomplete));
        }
    }
    let barrier = &observation.barrier;
    if barrier.owner != *owner
        || barrier.run != *run
        || barrier.scope != target.scope_hash()?
        || !barrier.enforced
        || barrier.privileged_edit_detected
        || barrier
            .covered_paths
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            != required_writer_paths()
        || barrier.covered_paths.len() != required_writer_paths().len()
    {
        return Err(refuse(CoordinationCode::BarrierRequired));
    }
    Ok(())
}
