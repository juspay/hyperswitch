//! Read authority, lossless observations and the sealed completeness boundary.

use std::{collections::BTreeMap, time::Duration};

use serde::Serialize;

use super::{raw_json::Json, Digest, Name, Revision};

pub type InventoryResult<T> = error_stack::Result<T, InventoryError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryErrorKind {
    CertificationPending,
    PermissionScopeUnproven,
    InvalidContract,
    InvalidTarget,
    InvalidLimits,
    InvalidRequest,
    PermissionDenied,
    HttpStatus,
    PartialResponse,
    Redirect,
    Transport,
    Deadline,
    ByteLimit,
    ItemLimit,
    PageLimit,
    RequestLimit,
    JsonLimit,
    MalformedJson,
    MalformedResponse,
    MissingField,
    FilteredResponse,
    PaginationMismatch,
    DuplicateIdentity,
    AmbiguousCondition,
    OverrideIdentityCollision,
    VersionChanged,
    InventoryChanged,
    WorkspaceMismatch,
}

#[derive(Debug, thiserror::Error)]
#[error("required-fields inventory refused: {kind:?}")]
pub struct InventoryError {
    pub kind: InventoryErrorKind,
}

pub(crate) fn failure(kind: InventoryErrorKind) -> error_stack::Report<InventoryError> {
    InventoryError { kind }.into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Contexts,
    Defaults,
    Dimensions,
    Experiments,
    ExperimentGroups,
    Workspace,
    VersionHead,
}

pub(crate) const PAGED: [Resource; 5] = [
    Resource::Contexts,
    Resource::Defaults,
    Resource::Dimensions,
    Resource::Experiments,
    Resource::ExperimentGroups,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetIdentity {
    pub target_id: Name,
    pub organisation: Name,
    pub workspace: Name,
    pub instance_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceScope {
    CertifiedServer,
    #[cfg(test)]
    SyntheticFixture,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadEvidence {
    pub contract_id: Name,
    pub client_version: String,
    pub server_image: Digest,
    pub server_source: Revision,
    pub reader_principal: Name,
    pub permission_receipt: Digest,
    pub scope: EvidenceScope,
}

/// Per-resource proof: one-based counted pages, exact totals, stable full-row ordering, and
/// the certified empty-list boundary. No `all=true` inference from a different operation.
#[derive(Debug, Clone)]
pub(crate) struct PageContract {
    pub empty_total_pages: u32,
}

/// Cannot be deserialized or constructed from caller-supplied "certified" booleans.
/// A registry entry requires real Step 2 evidence and a target/principal permission review.
#[derive(Clone)]
pub struct CertifiedReadTarget {
    pub(super) identity: TargetIdentity,
    pub(super) endpoint: reqwest::Url,
    pub(super) evidence: ReadEvidence,
    // Bound by the independently reviewed permission receipt. Never emitted in reports.
    // Rotation requires a fresh review/binding; a caller-supplied principal name is no proof.
    pub(super) authorization_fingerprint: Digest,
    pub(super) pages: BTreeMap<Resource, PageContract>,
    pub(super) full_read_scope: bool,
    pub(super) all_experiment_history: bool,
    pub(super) latest_version_descending: bool,
    pub(super) empty_version_total_pages: u32,
}

impl CertifiedReadTarget {
    pub fn from_registry(target_id: &Name) -> InventoryResult<Self> {
        // Step 2 has no certified server/permission profiles. Adding a registry entry is a
        // separate evidence review, not a CLI force flag or permission inferred from totals.
        const CERTIFIED_TARGETS: &[CertifiedReadTarget] = &[];
        CERTIFIED_TARGETS
            .iter()
            .find(|entry| &entry.identity.target_id == target_id)
            .cloned()
            .ok_or_else(|| failure(InventoryErrorKind::CertificationPending))
    }

    pub(crate) fn validate(&self) -> InventoryResult<()> {
        use InventoryErrorKind as K;
        if !self.full_read_scope {
            return Err(failure(K::PermissionScopeUnproven));
        }
        if self.evidence.client_version != "0.119.0"
            || !self.all_experiment_history
            || !self.latest_version_descending
            || self.empty_version_total_pages > 1
            || self.pages.len() != PAGED.len()
            || PAGED
                .iter()
                .any(|resource| !self.pages.contains_key(resource))
            || self
                .pages
                .values()
                .any(|contract| contract.empty_total_pages > 1)
        {
            return Err(failure(K::InvalidContract));
        }
        let identifier = |value: &Name| {
            value
                .as_str()
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        };
        if self.identity.instance_generation == 0
            || !identifier(&self.identity.organisation)
            || !identifier(&self.identity.workspace)
            || self.endpoint.scheme() != "https"
            || self.endpoint.host_str().is_none()
            || !self.endpoint.username().is_empty()
            || self.endpoint.password().is_some()
            || self.endpoint.query().is_some()
            || self.endpoint.fragment().is_some()
            || self.endpoint.path_segments().is_none_or(|segments| {
                segments.into_iter().any(|s| {
                    !s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                })
            })
        {
            return Err(failure(K::InvalidTarget));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct InventoryLimits {
    pub page_size: u32,
    pub max_pages_per_resource: u32,
    pub max_items_per_resource: u32,
    pub max_total_items: u32,
    pub max_response_bytes: usize,
    pub max_total_bytes: usize,
    pub max_requests: u32,
    pub request_timeout: Duration,
    pub total_timeout: Duration,
}

impl Default for InventoryLimits {
    fn default() -> Self {
        Self {
            page_size: 100,
            max_pages_per_resource: 32,
            max_items_per_resource: 5_000,
            max_total_items: 30_000,
            max_response_bytes: 2 * 1024 * 1024,
            max_total_bytes: 32 * 1024 * 1024,
            max_requests: 400,
            request_timeout: Duration::from_secs(10),
            total_timeout: Duration::from_secs(60),
        }
    }
}

impl InventoryLimits {
    pub(crate) fn validate(&self) -> InventoryResult<()> {
        if !(1..=1_000).contains(&self.page_size)
            || !(1..=128).contains(&self.max_pages_per_resource)
            || !(1..=20_000).contains(&self.max_items_per_resource)
            || !(1..=100_000).contains(&self.max_total_items)
            || !(1..=4 * 1024 * 1024).contains(&self.max_response_bytes)
            || self.max_total_bytes < self.max_response_bytes
            || self.max_total_bytes > 64 * 1024 * 1024
            || !(1..=2_000).contains(&self.max_requests)
            || self.request_timeout.is_zero()
            || self.request_timeout > Duration::from_secs(30)
            || self.total_timeout < self.request_timeout
            || self.total_timeout > Duration::from_secs(120)
        {
            return Err(failure(InventoryErrorKind::InvalidLimits));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Scalar {
    String(String),
    Bool(bool),
    Number(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ObservedCondition {
    Equality(BTreeMap<String, Scalar>),
    /// Original condition is retained in the raw row. No owned ContextKey is inferred.
    QuarantinedUnsupportedPredicate,
}

#[derive(Debug, Clone)]
pub struct ContextProjection {
    pub id: Name,
    pub override_id: Name,
    pub condition: ObservedCondition,
}
#[derive(Debug, Clone)]
pub struct DefaultProjection {
    pub key: Name,
}
#[derive(Debug, Clone)]
pub enum DimensionKind {
    Regular,
    LocalCohort(String),
    RemoteCohort(String),
}
#[derive(Debug, Clone)]
pub struct DimensionProjection {
    pub name: Name,
    pub position: i32,
    pub mandatory: bool,
    pub kind: DimensionKind,
}
#[derive(Debug, Clone)]
pub struct VariantReference {
    pub id: Name,
    pub context_id: Option<Name>,
    pub override_id: Option<Name>,
}
#[derive(Debug, Clone)]
pub struct ExperimentProjection {
    pub id: Name,
    pub status: String,
    pub condition: ObservedCondition,
    pub variants: Vec<VariantReference>,
    pub group_id: Option<Name>,
}
#[derive(Debug, Clone)]
pub struct ExperimentGroupProjection {
    pub id: Name,
    pub condition: ObservedCondition,
    pub member_experiment_ids: Vec<Name>,
    pub buckets: Vec<Option<BucketReference>>,
}
#[derive(Debug, Clone)]
pub struct BucketReference {
    pub experiment_id: Name,
    pub variant_id: Name,
}
#[derive(Debug, Clone)]
pub struct WorkspaceProjection {
    pub organisation: Name,
    pub workspace: Name,
    pub config_version: Option<Name>,
}

#[derive(Debug, Clone)]
pub struct ObservedRow<T> {
    pub(crate) raw: Json,
    pub(crate) projection: T,
}
impl<T> ObservedRow<T> {
    pub fn raw_json(&self) -> &str {
        self.raw.raw()
    }
    pub fn projection(&self) -> &T {
        &self.projection
    }
    pub fn raw_field(&self, name: &str) -> Option<&str> {
        self.raw.object().ok()?.get(name).map(Json::raw)
    }
}

#[derive(Debug, Clone)]
pub struct RawPage {
    pub(crate) resource: Resource,
    pub(crate) page: u32,
    pub(crate) body: Json,
}
impl RawPage {
    pub fn resource(&self) -> Resource {
        self.resource
    }
    pub fn page(&self) -> u32 {
        self.page
    }
    pub fn raw_json(&self) -> &str {
        self.body.raw()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InventoryPass {
    pub contexts: BTreeMap<Name, ObservedRow<ContextProjection>>,
    pub defaults: BTreeMap<Name, ObservedRow<DefaultProjection>>,
    pub dimensions: BTreeMap<Name, ObservedRow<DimensionProjection>>,
    pub experiments: BTreeMap<Name, ObservedRow<ExperimentProjection>>,
    pub groups: BTreeMap<Name, ObservedRow<ExperimentGroupProjection>>,
    pub workspace: ObservedRow<WorkspaceProjection>,
    pub pages: Vec<RawPage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompletenessProof {
    pub target: TargetIdentity,
    pub evidence: ReadEvidence,
    pub inventory_hash: Digest,
    pub version: Option<Name>,
    pub requests: u32,
    pub bytes: usize,
    pub observed_items: u32,
    pub identical_full_passes: u8,
    pub provisional_without_writer_exclusion: bool,
}

/// Only the bounded two-pass reader can construct this. No Deserialize, Default or public
/// unchecked constructor; raw rows and the completeness proof stay bound and immutable.
#[derive(Debug)]
pub struct CompleteInventory {
    pub(super) pass: InventoryPass,
    pub(super) proof: CompletenessProof,
    pub(super) version_pages: Vec<RawPage>,
}
impl CompleteInventory {
    pub fn proof(&self) -> &CompletenessProof {
        &self.proof
    }
    pub fn contexts(&self) -> &BTreeMap<Name, ObservedRow<ContextProjection>> {
        &self.pass.contexts
    }
    pub fn defaults(&self) -> &BTreeMap<Name, ObservedRow<DefaultProjection>> {
        &self.pass.defaults
    }
    pub fn dimensions(&self) -> &BTreeMap<Name, ObservedRow<DimensionProjection>> {
        &self.pass.dimensions
    }
    pub fn experiments(&self) -> &BTreeMap<Name, ObservedRow<ExperimentProjection>> {
        &self.pass.experiments
    }
    pub fn experiment_groups(&self) -> &BTreeMap<Name, ObservedRow<ExperimentGroupProjection>> {
        &self.pass.groups
    }
    pub fn workspace(&self) -> &ObservedRow<WorkspaceProjection> {
        &self.pass.workspace
    }
    pub fn raw_pages(&self) -> &[RawPage] {
        &self.pass.pages
    }
    pub fn version_observations(&self) -> &[RawPage] {
        &self.version_pages
    }
}
