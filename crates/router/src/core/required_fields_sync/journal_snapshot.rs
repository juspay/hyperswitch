//! Restricted snapshot construction from the sealed inventory and accepted read-only plan.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    canonical_hash,
    journal::{
        checksum, encode, fail, JournalCode, JournalResult, PreparedRequest, RestrictedJson,
        Retention, SecretGuard, MAX_REQUESTS, MAX_SNAPSHOT,
    },
    ownership::OwnershipManifest,
    plan::ReadOnlyPlan,
    planner_types::{self, ProjectionContract},
    preflight::PrerequisitePolicy,
    raw_json::Json,
    types::{CompleteInventory, TargetIdentity},
    Digest, HashDomain, Name,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryIdentity {
    pub target: TargetIdentity,
    /// Digest of the certified endpoint identity; no URL/userinfo/query/credentials.
    pub endpoint_identity: Digest,
    pub coordinator: Name,
    pub executor_image: Digest,
    pub retained_recovery_image: Digest,
}

/// All arbitrary JSON remains restricted. These inputs are checked against the actual plan.
/// Endpoint/image identity and request wire correctness need later certified adapters, not flags.
pub struct Capture<'a> {
    pub plan: &'a ReadOnlyPlan,
    pub inventory: &'a CompleteInventory,
    pub ownership: &'a OwnershipManifest,
    pub prerequisites: &'a PrerequisitePolicy,
    pub projection: &'a ProjectionContract,
    pub bundle: RestrictedJson,
    pub identity: RecoveryIdentity,
    pub retention: Retention,
    pub requests: Vec<PreparedRequest>,
    pub unreadable_attributes: Vec<Name>,
    pub server_managed_attributes: Vec<Name>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SnapshotData {
    pub schema_version: u16,
    pub identity: RecoveryIdentity,
    pub retention: Retention,
    pub requests: Vec<PreparedRequest>,
    pub pending_create_ownership: Vec<Digest>,
    pub contents: RestrictedJson,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<super::journal::ArtifactReference>,
}
/// Sealed construction from accepted inputs; no public Deserialize/unchecked constructor.
#[derive(Serialize)]
#[serde(transparent)]
pub struct PreparedSnapshot {
    pub(super) data: SnapshotData,
}
impl PreparedSnapshot {
    pub fn capture(input: Capture<'_>, guard: &SecretGuard) -> JournalResult<Self> {
        let report = input.plan.report();
        let proof = input.inventory.proof();
        let invalid = || fail(JournalCode::BindingMismatch);
        let parsed = Json::parse(input.bundle.bytes()).map_err(|_| invalid())?;
        // Foundation bundles allow only integer numbers. Preserve bytes; canonical hashing alone
        // uses Value, never the full-row restoration images (which may contain decimal lexemes).
        let bundle: serde_json::Value =
            serde_json::from_slice(parsed.raw().as_bytes()).map_err(|_| invalid())?;
        if report.plan_hash.is_none()
            || report.inputs.inventory_hash != proof.inventory_hash
            || input.identity.target != proof.target
            || planner_types::digest(&proof.target).map_err(|_| invalid())?
                != report.inputs.target_hash
            || planner_types::digest(input.ownership).map_err(|_| invalid())?
                != report.inputs.ownership_hash
            || planner_types::digest(input.prerequisites).map_err(|_| invalid())?
                != report.inputs.prerequisites_hash
            || super::plan::projection_hash(input.projection).map_err(|_| invalid())?
                != report.inputs.projection_hash
            || canonical_hash(HashDomain::Bundle, &bundle).map_err(|_| invalid())?
                != report.inputs.bundle_hash
        {
            return Err(invalid());
        }
        if input.unreadable_attributes.len() > 128 || input.server_managed_attributes.len() > 128 {
            return Err(fail(JournalCode::LimitExceeded));
        }
        let raw = |bytes: &[u8]| RestrictedJson::parse(bytes, guard);
        let mut rows = BTreeMap::new();
        macro_rules! rows {
            ($label:literal, $source:expr) => {
                rows.insert(
                    $label,
                    $source
                        .values()
                        .map(|row| raw(row.raw_json().as_bytes()))
                        .collect::<JournalResult<Vec<_>>>()?,
                );
            };
        }
        rows!("contexts", input.inventory.contexts());
        rows!("defaults", input.inventory.defaults());
        rows!("dimensions", input.inventory.dimensions());
        rows!("experiments", input.inventory.experiments());
        rows!("experiment_groups", input.inventory.experiment_groups());
        let pages = input
            .inventory
            .raw_pages()
            .iter()
            .chain(input.inventory.version_observations())
            .map(|page| {
                Ok((
                    page.resource(),
                    page.page(),
                    raw(page.raw_json().as_bytes())?,
                ))
            })
            .collect::<JournalResult<Vec<_>>>()?;
        let actions = input.plan.actions().iter().map(|action| {
            Ok(serde_json::json!({
                "summary": action.summary().map_err(|_| invalid())?, "server_id": action.server_id,
                "condition": action.condition,
                "before": action.before_json().map(|s| raw(s.as_bytes())).transpose()?,
                "after": action.after_json().map(|s| raw(s.as_bytes())).transpose()?,
            }))
        }).collect::<JournalResult<Vec<_>>>()?;
        // The planner already verified every report/approval. Retain their original bytes.
        let reports = input
            .plan
            .accepted_reports
            .iter()
            .map(|bytes| raw(bytes))
            .collect::<JournalResult<Vec<_>>>()?;
        let contents = encode(&serde_json::json!({
            "plan": report, "proof": proof, "rows": rows, "pages": pages,
            "workspace": raw(input.inventory.workspace().raw_json().as_bytes())?,
            "bundle": input.bundle, "ownership": input.ownership, "prerequisites": input.prerequisites,
            "projection_contract": super::plan::projection_material(input.projection),
            "trusted_report_digests": input.projection.trusted_report_digests,
            "actions": actions, "case_request": raw(input.plan.case_request_json().as_bytes())?,
            "accepted_reports": reports, "accepted_approval": input.plan.accepted_approval,
            "unreadable_attributes": input.unreadable_attributes,
            "server_managed_attributes": input.server_managed_attributes,
        }))?;
        let snapshot = Self {
            data: SnapshotData {
                schema_version: 1,
                identity: input.identity,
                retention: input.retention,
                requests: input.requests,
                pending_create_ownership: input
                    .plan
                    .actions()
                    .iter()
                    .filter_map(|action| action.ownership_intent().cloned())
                    .collect(),
                contents: raw(&contents)?,
                recovery: None,
            },
        };
        snapshot.validate()?;
        guard.check(&encode(&snapshot)?)?;
        Ok(snapshot)
    }
    pub fn contents(&self) -> &RestrictedJson {
        &self.data.contents
    }
    pub fn pending_create_ownership(&self) -> &[Digest] {
        &self.data.pending_create_ownership
    }
    pub fn requests(&self) -> &[PreparedRequest] {
        &self.data.requests
    }
    pub fn hash(&self) -> JournalResult<Digest> {
        checksum(&encode(self)?)
    }
    pub(super) fn validate(&self) -> JournalResult<()> {
        self.data.validate()
    }
}
impl SnapshotData {
    pub(super) fn validate(&self) -> JournalResult<()> {
        self.retention.validate()?;
        if ![1, 2].contains(&self.schema_version)
            || (self.schema_version == 1 && self.recovery.is_some())
            || (self.schema_version == 2 && self.recovery.is_none())
            || self.identity.target.instance_generation == 0
            || self.requests.len() > MAX_REQUESTS
            || encode(self)?.len() > MAX_SNAPSHOT
        {
            return Err(fail(JournalCode::LimitExceeded));
        }
        let contents =
            Json::parse(self.contents.bytes()).map_err(|_| fail(JournalCode::CorruptArtifact))?;
        let actions = contents
            .get("actions")
            .and_then(Json::array)
            .map_err(|_| fail(JournalCode::CorruptArtifact))?;
        let mut next = 0;
        for request in &self.requests {
            request.validate()?;
            for index in &request.action_indices {
                if *index != next {
                    return Err(fail(JournalCode::BindingMismatch));
                }
                next += 1;
            }
        }
        if next != actions.len() {
            return Err(fail(JournalCode::BindingMismatch));
        }
        Ok(())
    }
}
