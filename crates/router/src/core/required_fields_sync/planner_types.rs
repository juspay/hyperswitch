//! Pure planner boundaries. Reports contain digests/counters, never arbitrary row or SDK data.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    raw_json::{fingerprint, Json},
    types::{EvidenceScope, TargetIdentity},
    ContextKey, DesiredState, Digest, FieldContractCatalog, ManagedKey, Name, Revision,
};

pub type PlanResult<T> = error_stack::Result<T, PlanError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanCode {
    CertificationPending,
    InvalidInput,
    LimitExceeded,
    BindingMismatch,
    OwnershipRequired,
    StaleGrant,
    DuplicateGrant,
    ExcludedScope,
    RetirementRequired,
    EmptyDesired,
    MissingDimension,
    UnsupportedSchema,
    UnsupportedFunction,
    MissingDefault,
    MissingPresentation,
    IncompatiblePresentation,
    InvalidOptions,
    ExperimentConflict,
    UnsupportedPredicate,
    AmbiguousIdentity,
    UnknownProjection,
    OrderedValueCollision,
    MissingCaseEvidence,
    InvalidCaseEvidence,
    ConnectorOrderConflict,
    ApprovalRequired,
    ApprovalExpired,
}

#[derive(Debug, thiserror::Error, Serialize)]
#[error("required-fields planning refused: {code:?}")]
pub struct PlanError {
    pub code: PlanCode,
    pub subject: Option<Digest>,
    pub remedy: &'static str,
}

pub(super) fn refuse(code: PlanCode) -> error_stack::Report<PlanError> {
    let remedy = match code {
        PlanCode::CertificationPending | PlanCode::UnknownProjection => {
            "Certify the exact target/server projection and reader case contract."
        }
        PlanCode::OwnershipRequired | PlanCode::StaleGrant | PlanCode::DuplicateGrant => {
            "Review target-bound grants against the complete current row hashes."
        }
        PlanCode::RetirementRequired | PlanCode::EmptyDesired | PlanCode::ExcludedScope => {
            "Review an explicit retirement or ownership handoff; exclusions do not grant deletion."
        }
        PlanCode::MissingPresentation
        | PlanCode::IncompatiblePresentation
        | PlanCode::MissingDefault => {
            "Promote or correct reviewed live metadata/defaults; a valid seed is not live evidence."
        }
        PlanCode::MissingCaseEvidence | PlanCode::InvalidCaseEvidence => {
            "Execute every bound before/after SDK stage through the trusted case runner."
        }
        PlanCode::ApprovalRequired | PlanCode::ApprovalExpired => {
            "Obtain an unexpired approval for the exact candidate, cases and impact hash."
        }
        PlanCode::LimitExceeded => {
            "Review a tractable bounded migration; do not truncate or batch automatically."
        }
        _ => "Inspect the restricted candidate inputs and resolve this refusal before replanning.",
    };
    PlanError {
        code,
        subject: None,
        remedy,
    }
    .into()
}

pub(super) fn checked<T>(result: super::types::InventoryResult<T>) -> PlanResult<T> {
    result.map_err(|_| refuse(PlanCode::InvalidInput))
}

pub(super) fn json<T: Serialize>(value: &T) -> PlanResult<Json> {
    let bytes = serde_json::to_vec(value).map_err(|_| refuse(PlanCode::InvalidInput))?;
    checked(Json::parse(&bytes))
}
pub(super) fn hash(value: &Json) -> PlanResult<Digest> {
    checked(fingerprint("planner-v1", &checked(value.canonical())?))
}
pub(super) fn digest<T: Serialize>(value: &T) -> PlanResult<Digest> {
    hash(&json(value)?)
}

/// Bounded, duplicate-key rejecting input boundary; no file/environment reads.
pub fn parse_manifest<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> PlanResult<T> {
    if bytes.len() > 128 * 1024 {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    checked(Json::parse(bytes))?;
    serde_json::from_slice(bytes).map_err(|_| refuse(PlanCode::InvalidInput))
}

/// Only the renderer adapter (or synthetic unit fixtures) creates this boundary.
/// No Deserialize or public fields: a desired map cannot replace reviewed renderer inputs.
#[derive(Clone)]
pub struct VerifiedDesired {
    pub(super) desired: DesiredState,
    pub(super) catalog: FieldContractCatalog,
    pub(super) bundle_hash: Digest,
    pub(super) release_hash: Digest,
    pub(super) source_availability_hash: Digest,
    pub(super) excluded_connectors: BTreeSet<Name>,
}
impl VerifiedDesired {
    pub fn bundle_hash(&self) -> &Digest {
        &self.bundle_hash
    }
    pub fn release_hash(&self) -> &Digest {
        &self.release_hash
    }
    pub fn source_availability_hash(&self) -> &Digest {
        &self.source_availability_hash
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanIntent {
    pub authority: Name,
    pub release_hash: Digest,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanLimits {
    pub max_contexts: usize,
    pub max_actions: usize,
    pub max_partitions: usize,
    pub max_connector_orders: usize,
    pub max_evaluations: usize,
    pub max_artifact_bytes: usize,
}
impl Default for PlanLimits {
    fn default() -> Self {
        Self {
            max_contexts: 512,
            max_actions: 100,
            max_partitions: 128,
            max_connector_orders: 24,
            max_evaluations: 4096,
            max_artifact_bytes: 1024 * 1024,
        }
    }
}
impl PlanLimits {
    pub(super) fn validate(&self) -> PlanResult<()> {
        if self.max_contexts == 0
            || self.max_contexts > 512
            || self.max_actions == 0
            || self.max_actions > 100
            || self.max_partitions == 0
            || self.max_partitions > 128
            || self.max_connector_orders == 0
            || self.max_connector_orders > 24
            || self.max_evaluations == 0
            || self.max_evaluations > 4096
            || self.max_artifact_bytes == 0
            || self.max_artifact_bytes > 1024 * 1024
        {
            return Err(refuse(PlanCode::LimitExceeded));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Create,
    Replace,
    Delete,
    ExtendDimension,
    CreateDefault,
}

/// Restricted artifact. Do not log/serialize it as the public report. New rows have no server ID.
#[derive(Debug, Clone)]
pub struct PlannedAction {
    pub(super) kind: ActionKind,
    pub(super) server_id: Option<Name>,
    pub(super) condition: Option<ContextKey>,
    pub(super) before: Option<Json>,
    pub(super) after: Option<Json>,
    pub(super) preservation_hash: Digest,
    pub(super) ownership_intent: Option<Digest>,
}
impl PlannedAction {
    pub fn kind(&self) -> &ActionKind {
        &self.kind
    }
    pub fn server_id(&self) -> Option<&Name> {
        self.server_id.as_ref()
    }
    pub fn before_json(&self) -> Option<&str> {
        self.before.as_ref().map(Json::raw)
    }
    pub fn after_json(&self) -> Option<&str> {
        self.after.as_ref().map(Json::raw)
    }
    pub fn ownership_intent(&self) -> Option<&Digest> {
        self.ownership_intent.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSummary {
    pub kind: ActionKind,
    pub subject: Digest,
    pub before_hash: Option<Digest>,
    pub after_hash: Option<Digest>,
    pub preserved_hash: Digest,
    pub ownership_intent: Option<Digest>,
}
impl PlannedAction {
    pub(super) fn summary(&self) -> PlanResult<ActionSummary> {
        Ok(ActionSummary {
            kind: self.kind.clone(),
            subject: digest(&(&self.server_id, &self.condition))?,
            before_hash: self.before.as_ref().map(hash).transpose()?,
            after_hash: self.after.as_ref().map(hash).transpose()?,
            preserved_hash: self.preservation_hash.clone(),
            ownership_intent: self.ownership_intent.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SdkStage {
    JsResolver,
    RustResolver,
    BrowserComponents,
    Checkout,
}

/// Private certification boundary. Step 2 supplies no valid registration yet.
/// Function pointers must come from a reviewed exact-build implementation, never a caller flag.
pub struct ProjectionContract {
    pub(super) id: Name,
    pub(super) target: TargetIdentity,
    pub(super) authority: Name,
    pub(super) read_contract: Name,
    pub(super) server_image: Digest,
    pub(super) server_source: Revision,
    pub(super) scope: EvidenceScope,
    pub(super) receipt: Digest,
    pub(super) sdk_contract: Name,
    pub(super) web_revision: Revision,
    pub(super) shared_revision: Revision,
    pub(super) evaluator: Digest,
    pub(super) case_pack: Digest,
    pub(super) provenance: serde_json::Value,
    pub(super) artifacts: BTreeMap<SdkStage, Digest>,
    pub(super) stage_cases: BTreeMap<SdkStage, BTreeSet<Name>>,
    pub(super) connector_sets: Vec<BTreeSet<Name>>,
    pub(super) browser_scenarios: BTreeSet<Name>,
    pub(super) override_identity: fn(&str) -> PlanResult<String>,
    // Populated by a future trusted receipt adapter, not JSON supplied by the CLI.
    pub(super) trusted_report_digests: Vec<Digest>,
}
impl ProjectionContract {
    pub fn from_registry(_id: &Name) -> PlanResult<Self> {
        Err(refuse(PlanCode::CertificationPending))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundInputs {
    pub target_hash: Digest,
    pub inventory_hash: Digest,
    pub bundle_hash: Digest,
    pub ownership_hash: Digest,
    pub prerequisites_hash: Digest,
    pub intent: PlanIntent,
    pub projection_hash: Digest,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ImpactCounts {
    pub evaluated_partitions: usize,
    pub evaluated_connector_orders: usize,
    pub required_added: usize,
    pub required_removed: usize,
    pub options_changed: usize,
    pub options_reordered: usize,
    pub options_emptied: usize,
    pub shadowed_row_changes: usize,
    pub connector_order_conflicts: usize,
    pub managed_added: usize,
    pub managed_removed: usize,
    pub managed_changed: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BrowserImpactCounts {
    pub included_changed: usize,
    pub visible_changed: usize,
    pub validation_changed: usize,
    pub payload_changed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticApproval {
    pub schema_version: u16,
    pub target: TargetIdentity,
    pub candidate_hash: Digest,
    pub actions_hash: Digest,
    pub case_request_hash: Digest,
    pub impact_hash: Digest,
    pub sdk_evidence_hash: Digest,
    pub approver: Name,
    pub review: Name,
    pub expires_at_unix: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanReport {
    pub schema_version: u16,
    pub candidate_hash: Digest,
    pub plan_hash: Option<Digest>,
    pub inputs: BoundInputs,
    pub actions: Vec<ActionSummary>,
    pub actions_hash: Digest,
    pub classifications: BTreeMap<String, usize>,
    pub impact: ImpactCounts,
    /// None until both browser stages execute against both bound configurations.
    pub browser_impact: Option<BTreeMap<SdkStage, BrowserImpactCounts>>,
    pub case_request_hash: Digest,
    pub impact_hash: Digest,
    pub scope: EvidenceScope,
    pub provisional: bool,
    pub hosted_apply: &'static str,
    pub review_state: &'static str,
}

pub(super) fn key_text(key: &ManagedKey) -> PlanResult<String> {
    serde_json::from_value(serde_json::to_value(key).map_err(|_| refuse(PlanCode::InvalidInput))?)
        .map_err(|_| refuse(PlanCode::InvalidInput))
}
