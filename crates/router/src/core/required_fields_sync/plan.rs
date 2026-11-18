//! Pure candidate construction and exact evidence/approval validation. No I/O or execution.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    impact,
    ownership::{self, OwnershipManifest},
    planner_types::*,
    preflight::{self, PrerequisitePolicy},
    raw_json::Json,
    types::CompleteInventory,
    Digest, Name,
};

pub struct PlanCandidate {
    report: PlanReport,
    actions: Vec<PlannedAction>,
    analysis: impact::Analysis,
    actions_hash: Digest,
    target: super::types::TargetIdentity,
}
impl PlanCandidate {
    pub fn report(&self) -> &PlanReport {
        &self.report
    }
    /// Restricted before/after images for review/journaling. They are not HTTP request bodies.
    pub fn actions(&self) -> &[PlannedAction] {
        &self.actions
    }
    pub fn actions_hash(&self) -> &Digest {
        &self.actions_hash
    }
    /// Contains live values; keep in restricted artifact storage, never public diagnostics.
    pub fn case_request_json(&self) -> &str {
        self.analysis.request.raw()
    }
}
pub struct ReadOnlyPlan {
    candidate: PlanCandidate,
}
impl ReadOnlyPlan {
    pub fn report(&self) -> &PlanReport {
        self.candidate.report()
    }
    pub fn actions(&self) -> &[PlannedAction] {
        self.candidate.actions()
    }
}

fn projection_hash(contract: &ProjectionContract) -> PlanResult<Digest> {
    digest(&(
        &contract.id,
        &contract.target,
        &contract.authority,
        &contract.read_contract,
        &contract.server_image,
        &contract.server_source,
        &contract.scope,
        &contract.receipt,
        &contract.sdk_contract,
        &contract.web_revision,
        &contract.shared_revision,
        &contract.evaluator,
        &contract.case_pack,
        (
            &contract.provenance,
            &contract.artifacts,
            &contract.stage_cases,
            &contract.connector_sets,
            &contract.browser_scenarios,
        ),
    ))
}

pub fn prepare(
    inventory: &CompleteInventory,
    desired: &VerifiedDesired,
    ownership: &OwnershipManifest,
    prerequisites: &PrerequisitePolicy,
    intent: PlanIntent,
    contract: &ProjectionContract,
    limits: PlanLimits,
) -> PlanResult<PlanCandidate> {
    prepare_inner(
        inventory,
        desired,
        ownership,
        prerequisites,
        intent,
        contract,
        limits,
    )
    .map_err(|error| {
        PlanError {
            code: error.current_context().code,
            subject: Some(inventory.proof().inventory_hash.clone()),
            remedy: error.current_context().remedy,
        }
        .into()
    })
}

fn prepare_inner(
    inventory: &CompleteInventory,
    desired: &VerifiedDesired,
    ownership: &OwnershipManifest,
    prerequisites: &PrerequisitePolicy,
    intent: PlanIntent,
    contract: &ProjectionContract,
    limits: PlanLimits,
) -> PlanResult<PlanCandidate> {
    limits.validate()?;
    let proof = inventory.proof();
    let sdk = &desired.catalog.sdk;
    if proof.identical_full_passes != 2
        || !proof.provisional_without_writer_exclusion
        || contract.target != proof.target
        || contract.read_contract != proof.evidence.contract_id
        || contract.server_image != proof.evidence.server_image
        || contract.server_source != proof.evidence.server_source
        || contract.scope != proof.evidence.scope
        || proof.evidence.client_version != "0.119.0"
        || intent.generation == 0
        || intent.authority != contract.authority
        || intent.release_hash != desired.release_hash
        || contract.sdk_contract != sdk.contract_id
        || contract.web_revision != sdk.web_revision
        || contract.shared_revision != sdk.shared_revision
        || contract.evaluator != sdk.evaluator_hash
    {
        return Err(refuse(PlanCode::BindingMismatch));
    }
    if inventory
        .contexts()
        .len()
        .max(desired.desired.contexts.len())
        > limits.max_contexts
    {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    if desired.desired.schema_version != 2 || !desired.desired.diagnostics.is_empty() {
        return Err(refuse(PlanCode::InvalidInput));
    }
    let index = ownership::inspect(inventory, desired, ownership, &intent)?;
    let mut actions = Vec::new();
    let mut managed_values: BTreeMap<String, Vec<Json>> = BTreeMap::new();
    let mut counts = ImpactCounts::default();
    let before = impact::rows(inventory)?;
    let mut after = before.clone();
    let contexts: BTreeSet<_> = desired
        .desired
        .contexts
        .keys()
        .chain(index.grants.keys())
        .cloned()
        .collect();
    for condition in contexts {
        let wanted = desired
            .desired
            .contexts
            .get(&condition)
            .cloned()
            .unwrap_or_default();
        if desired.desired.contexts.contains_key(&condition) && wanted.is_empty() {
            return Err(refuse(PlanCode::EmptyDesired));
        }
        let owned = if let Some(grant) = index.grants.get(&condition) {
            &grant.attributes
        } else {
            &index
                .creations
                .get(&condition)
                .ok_or_else(|| refuse(PlanCode::OwnershipRequired))?
                .attributes
        };
        if wanted.keys().any(|key| !owned.contains(key)) {
            return Err(refuse(PlanCode::OwnershipRequired));
        }
        let mut wanted_raw = BTreeMap::new();
        for (key, value) in &wanted {
            let key = key_text(key)?;
            let value = preflight::managed(value)?;
            managed_values
                .entry(key.clone())
                .or_default()
                .push(value.clone());
            wanted_raw.insert(key, value);
        }
        for key in owned {
            managed_values.entry(key_text(key)?).or_default();
        }
        if let Some(grant) = index.grants.get(&condition) {
            let row = &inventory
                .contexts()
                .get(&grant.server_id)
                .ok_or_else(|| refuse(PlanCode::StaleGrant))?
                .raw;
            let original = checked(checked(row.get("override"))?.object())?;
            let mut retained = original.clone();
            for key in owned {
                let key = key_text(key)?;
                if let Some(value) = retained.remove(&key) {
                    managed_values
                        .entry(key.clone())
                        .or_default()
                        .push(value.clone());
                    if !wanted_raw.contains_key(&key) {
                        counts.managed_removed += 1;
                    } else if checked(value.canonical())?
                        != checked(
                            wanted_raw
                                .get(&key)
                                .ok_or_else(|| refuse(PlanCode::InvalidInput))?
                                .canonical(),
                        )?
                    {
                        counts.managed_changed += 1;
                    }
                } else if wanted_raw.contains_key(&key) {
                    counts.managed_added += 1;
                }
            }
            let preserved = checked(Json::members(&BTreeMap::from([
                (
                    "row_metadata".to_owned(),
                    checked(row.replacing("override", json(&())?))?,
                ),
                (
                    "unrelated_override".to_owned(),
                    checked(Json::members(&retained))?,
                ),
            ])))?;
            retained.extend(wanted_raw);
            let replacement = checked(Json::members(&retained))?;
            if checked(replacement.canonical())?
                == checked(checked(row.get("override"))?.canonical())?
            {
                continue;
            }
            let empty = retained.is_empty();
            if empty && !grant.allow_row_deletion {
                return Err(refuse(PlanCode::RetirementRequired));
            }
            let proposed = if empty {
                None
            } else {
                Some(checked(row.replacing("override", replacement.clone()))?)
            };
            actions.push(PlannedAction {
                kind: if empty {
                    ActionKind::Delete
                } else {
                    ActionKind::Replace
                },
                server_id: Some(grant.server_id.clone()),
                condition: Some(condition),
                before: Some(row.clone()),
                after: proposed,
                preservation_hash: hash(&preserved)?,
                ownership_intent: None,
            });
            if empty {
                after.retain(|row| row.id != grant.server_id.as_str());
            } else {
                after
                    .iter_mut()
                    .find(|row| row.id == grant.server_id.as_str())
                    .ok_or_else(|| refuse(PlanCode::StaleGrant))?
                    .values = replacement;
            }
        } else {
            let grant = index
                .creations
                .get(&condition)
                .ok_or_else(|| refuse(PlanCode::OwnershipRequired))?;
            let intention = digest(&(
                &proof.target,
                &ownership.owner,
                &ownership.revision,
                &intent,
                &desired.bundle_hash,
                grant,
            ))?;
            let values = checked(Json::members(&wanted_raw))?;
            counts.managed_added += wanted_raw.len();
            let condition_json = json(&condition)?;
            let row = checked(Json::members(&BTreeMap::from([
                ("value".to_owned(), condition_json.clone()),
                ("override".to_owned(), values.clone()),
                (
                    "description".to_owned(),
                    json(&"Managed required-field contribution (contract rf-v1)")?,
                ),
            ])))?;
            let virtual_id = format!(
                "pending:{}",
                serde_json::to_string(&intention).map_err(|_| refuse(PlanCode::InvalidInput))?
            );
            if after.iter().any(|row| row.id == virtual_id) {
                return Err(refuse(PlanCode::AmbiguousIdentity));
            }
            after.push(impact::ContextRow {
                id: virtual_id,
                condition: condition_json,
                values,
                created: None,
            });
            actions.push(PlannedAction {
                kind: ActionKind::Create,
                server_id: None,
                condition: Some(condition),
                before: None,
                after: Some(row),
                preservation_hash: digest(&())?,
                ownership_intent: Some(intention),
            });
        }
    }
    if after.len() > limits.max_contexts {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    let live = preflight::inspect(inventory, desired, prerequisites, &managed_values)?;
    // A referenced historical context/override is protected too. Prerequisite changes have global
    // scope, so this profile refuses them in the presence of any active experiment.
    for experiment in inventory.experiments().values() {
        if !live.actions.is_empty()
            && matches!(
                experiment.projection().status.as_str(),
                "CREATED" | "INPROGRESS" | "PAUSED"
            )
        {
            return Err(refuse(PlanCode::ExperimentConflict));
        }
        for action in &actions {
            if let Some(id) = &action.server_id {
                let override_id = &inventory
                    .contexts()
                    .get(id)
                    .ok_or_else(|| refuse(PlanCode::StaleGrant))?
                    .projection()
                    .override_id;
                if experiment.projection().variants.iter().any(|v| {
                    v.context_id.as_ref() == Some(id) || v.override_id.as_ref() == Some(override_id)
                }) {
                    return Err(refuse(PlanCode::ExperimentConflict));
                }
            }
            if matches!(
                experiment.projection().status.as_str(),
                "CREATED" | "INPROGRESS" | "PAUSED"
            ) {
                if let Some(condition) = &action.condition {
                    if impact::overlap(checked(experiment.raw.get("context"))?, &json(condition)?)?
                    {
                        return Err(refuse(PlanCode::ExperimentConflict));
                    }
                }
            }
        }
    }
    for group in inventory.experiment_groups().values() {
        let bucket_ids = group
            .projection()
            .buckets
            .iter()
            .flatten()
            .map(|bucket| &bucket.experiment_id);
        if group
            .projection()
            .member_experiment_ids
            .iter()
            .chain(bucket_ids)
            .any(|id| {
                inventory.experiments().get(id).is_none_or(|e| {
                    matches!(
                        e.projection().status.as_str(),
                        "CREATED" | "INPROGRESS" | "PAUSED"
                    )
                })
            })
        {
            if !live.actions.is_empty() {
                return Err(refuse(PlanCode::ExperimentConflict));
            }
            for action in &actions {
                if let Some(condition) = &action.condition {
                    if impact::overlap(checked(group.raw.get("context"))?, &json(condition)?)? {
                        return Err(refuse(PlanCode::ExperimentConflict));
                    }
                }
            }
        }
    }
    let mut fields: BTreeSet<_> = managed_values
        .keys()
        .map(|key| preflight::field_key(key))
        .collect::<PlanResult<_>>()?;
    // Include aliases that can compete for a managed write path, even when they are unowned.
    let paths: BTreeSet<_> = fields
        .iter()
        .map(|field| {
            desired
                .catalog
                .fields
                .get(field)
                .ok_or_else(|| refuse(PlanCode::MissingPresentation))?
                .approved()
                .map(|v| v.write_path.as_str().to_owned())
                .map_err(|_| refuse(PlanCode::MissingPresentation))
        })
        .collect::<PlanResult<_>>()?;
    for values in std::iter::once(&live.defaults).chain(
        before
            .iter()
            .map(|row| row.values.object())
            .collect::<super::types::InventoryResult<Vec<_>>>()
            .map_err(|_| refuse(PlanCode::InvalidInput))?,
    ) {
        for (key, value) in values {
            if key.ends_with("._confirm_request_write_path")
                && paths.contains(&checked(value.string())?)
            {
                fields.insert(preflight::field_key(key)?);
            }
        }
    }
    for (key, row) in inventory.defaults() {
        if key.as_str().starts_with("dynamic_fields.")
            && fields.contains(&preflight::field_key(key.as_str())?)
        {
            preflight::no_functions(&row.raw)?;
        }
    }
    let mut analysis = impact::analyze(impact::AnalysisInputs {
        inventory,
        desired,
        policy: prerequisites,
        contract,
        before: &before,
        after: &after,
        live: &live,
        fields: &fields,
        limits: &limits,
    })?;
    analysis.counts.managed_added = counts.managed_added;
    analysis.counts.managed_removed = counts.managed_removed;
    analysis.counts.managed_changed = counts.managed_changed;
    if analysis.reference_hashes.iter().all(|(a, b)| a == b) {
        analysis.counts.shadowed_row_changes = actions.len();
    }
    actions.splice(0..0, live.actions);
    if actions.len() > limits.max_actions
        || actions
            .iter()
            .map(|a| {
                a.before.as_ref().map_or(0, |v| v.raw().len())
                    + a.after.as_ref().map_or(0, |v| v.raw().len())
            })
            .sum::<usize>()
            > limits.max_artifact_bytes
    {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    let inputs = BoundInputs {
        target_hash: digest(&proof.target)?,
        inventory_hash: proof.inventory_hash.clone(),
        bundle_hash: desired.bundle_hash.clone(),
        ownership_hash: digest(ownership)?,
        prerequisites_hash: digest(prerequisites)?,
        intent,
        projection_hash: projection_hash(contract)?,
    };
    let summaries = actions
        .iter()
        .map(PlannedAction::summary)
        .collect::<PlanResult<Vec<_>>>()?;
    let actions_hash = digest(&summaries)?;
    analysis.request = checked(analysis.request.replacing("bound_inputs", json(&inputs)?))?;
    if analysis.request.raw().len() > 262144 {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    let request_hash = hash(&analysis.request)?;
    let impact_hash = digest(&(
        &analysis.counts,
        &analysis.reference_hashes,
        &analysis.rust_reference_hashes,
        &analysis.browser_case_hashes,
        &analysis.browser_expected,
    ))?;
    let candidate_hash = digest(&(
        &inputs,
        &actions_hash,
        &request_hash,
        &impact_hash,
        &limits,
        &index.classifications,
    ))?;
    Ok(PlanCandidate {
        report: PlanReport {
            schema_version: 1,
            candidate_hash,
            plan_hash: None,
            inputs,
            actions: summaries,
            actions_hash: actions_hash.clone(),
            classifications: index.classifications,
            impact: analysis.counts.clone(),
            browser_impact: None,
            case_request_hash: request_hash,
            impact_hash,
            scope: contract.scope,
            provisional: true,
            hosted_apply: "disabled",
            review_state: "awaiting_bound_sdk_evidence_and_approval",
        },
        actions,
        analysis,
        actions_hash,
        target: proof.target.clone(),
    })
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Execution {
    engine: SdkStage,
    artifact_sha256: String,
    runtime: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseResult {
    id: Name,
    status: String,
    detail: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Step1Report {
    schema_version: u16,
    contract_id: String,
    pack_sha256: String,
    provenance: serde_json::Value,
    stage: SdkStage,
    execution: Execution,
    results: Vec<CaseResult>,
    status: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserObservation {
    pub case_hash: Digest,
    pub selected_descriptors_hash: Digest,
    pub included_fields_hash: Digest,
    pub visible_fields_hash: Digest,
    pub validation_hash: Digest,
    pub payload_hash: Digest,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundSdkReport {
    schema_version: u16,
    request_hash: Digest,
    phase: String,
    report: Step1Report,
    reference_hashes: Vec<Digest>,
    browser_observations: Vec<BrowserObservation>,
}

/// Sanitized evidence for the reviewer before authorizing semantic impact.
#[derive(Debug, Clone, Serialize)]
pub struct CaseEvidenceReport {
    pub candidate_hash: Digest,
    pub sdk_evidence_hash: Digest,
    pub browser_impact: BTreeMap<SdkStage, BrowserImpactCounts>,
}

/// A self-asserted `passed` Step 1 report is not trust. All eight phase/stage reports
/// are required even for a no-action candidate. No runner or browser is invoked here.
pub fn examine_cases(
    candidate: &PlanCandidate,
    contract: &ProjectionContract,
    reports: &[&[u8]],
) -> PlanResult<CaseEvidenceReport> {
    if candidate.report.inputs.projection_hash != projection_hash(contract)? {
        return Err(refuse(PlanCode::BindingMismatch));
    }
    if reports.len() != 8 {
        return Err(refuse(PlanCode::MissingCaseEvidence));
    }
    let mut seen = BTreeSet::new();
    let mut receipts = BTreeMap::new();
    let mut browser_observations = BTreeMap::new();
    for bytes in reports {
        if bytes.len() > 65536 {
            return Err(refuse(PlanCode::LimitExceeded));
        }
        let raw = checked(Json::parse(bytes))?;
        let receipt = hash(&raw)?;
        if !contract.trusted_report_digests.contains(&receipt) {
            return Err(refuse(PlanCode::MissingCaseEvidence));
        }
        let bound: BoundSdkReport =
            serde_json::from_slice(bytes).map_err(|_| refuse(PlanCode::InvalidCaseEvidence))?;
        let report = &bound.report;
        let phase = match bound.phase.as_str() {
            "before" => 0,
            "after" => 1,
            _ => return Err(refuse(PlanCode::InvalidCaseEvidence)),
        };
        let plain = |digest: &Digest| {
            serde_json::to_value(digest)
                .ok()
                .and_then(|v| {
                    v.as_str()
                        .map(|v| v.strip_prefix("sha256:").unwrap_or(v).to_owned())
                })
                .unwrap_or_default()
        };
        if bound.schema_version != 1
            || bound.request_hash != candidate.report.case_request_hash
            || report.schema_version != 1
            || report.contract_id != "required-fields-sdk-v1"
            || report.pack_sha256 != plain(&contract.case_pack)
            || report.provenance != contract.provenance
            || report.status != "passed"
            || report.execution.engine != report.stage
            || report.execution.runtime.is_empty()
            || report.execution.runtime.len() > 128
            || contract
                .artifacts
                .get(&report.stage)
                .is_none_or(|v| plain(v) != report.execution.artifact_sha256)
            || !seen.insert((phase, report.stage.clone()))
        {
            return Err(refuse(PlanCode::InvalidCaseEvidence));
        }
        let ids: BTreeSet<_> = report.results.iter().map(|r| r.id.clone()).collect();
        if ids.len() != report.results.len()
            || ids.is_empty()
            || ids.len() > 128
            || contract.stage_cases.get(&report.stage) != Some(&ids)
            || report
                .results
                .iter()
                .any(|r| r.status != "passed" || r.detail.len() > 512)
        {
            return Err(refuse(PlanCode::InvalidCaseEvidence));
        }
        match report.stage {
            SdkStage::JsResolver | SdkStage::RustResolver => {
                let expected = if report.stage == SdkStage::JsResolver {
                    &candidate.analysis.reference_hashes
                } else {
                    &candidate.analysis.rust_reference_hashes
                };
                let expected: Vec<_> = expected
                    .iter()
                    .map(|(a, b)| if phase == 0 { a.clone() } else { b.clone() })
                    .collect();
                if bound.reference_hashes != expected || !bound.browser_observations.is_empty() {
                    return Err(refuse(PlanCode::InvalidCaseEvidence));
                }
            }
            SdkStage::BrowserComponents | SdkStage::Checkout => {
                if !bound.reference_hashes.is_empty()
                    || bound.browser_observations.len()
                        != candidate.analysis.browser_case_hashes.len()
                {
                    return Err(refuse(PlanCode::InvalidCaseEvidence));
                }
                for ((observation, id), (a, b)) in bound
                    .browser_observations
                    .iter()
                    .zip(&candidate.analysis.browser_case_hashes)
                    .zip(&candidate.analysis.browser_expected)
                {
                    if &observation.case_hash != id
                        || &observation.selected_descriptors_hash != if phase == 0 { a } else { b }
                    {
                        return Err(refuse(PlanCode::InvalidCaseEvidence));
                    }
                }
                browser_observations.insert(
                    (phase, report.stage.clone()),
                    bound.browser_observations.clone(),
                );
            }
        }
        receipts.insert((phase, report.stage.clone()), receipt);
    }
    let evidence_hash = digest(&receipts.iter().collect::<Vec<_>>())?;
    let mut browser_impact = BTreeMap::new();
    for stage in [SdkStage::BrowserComponents, SdkStage::Checkout] {
        let before = browser_observations
            .get(&(0, stage.clone()))
            .ok_or_else(|| refuse(PlanCode::MissingCaseEvidence))?;
        let after = browser_observations
            .get(&(1, stage.clone()))
            .ok_or_else(|| refuse(PlanCode::MissingCaseEvidence))?;
        let mut counts = BrowserImpactCounts::default();
        for (a, b) in before.iter().zip(after) {
            counts.included_changed +=
                usize::from(a.included_fields_hash != b.included_fields_hash);
            counts.visible_changed += usize::from(a.visible_fields_hash != b.visible_fields_hash);
            counts.validation_changed += usize::from(a.validation_hash != b.validation_hash);
            counts.payload_changed += usize::from(a.payload_hash != b.payload_hash);
        }
        browser_impact.insert(stage, counts);
    }
    if checked(checked(candidate.analysis.request.get("before"))?.canonical())?
        == checked(checked(candidate.analysis.request.get("after"))?.canonical())?
        && browser_impact.values().any(|c| {
            c.included_changed + c.visible_changed + c.validation_changed + c.payload_changed > 0
        })
    {
        return Err(refuse(PlanCode::InvalidCaseEvidence));
    }
    Ok(CaseEvidenceReport {
        candidate_hash: candidate.report.candidate_hash.clone(),
        sdk_evidence_hash: evidence_hash,
        browser_impact,
    })
}

pub fn accept(
    candidate: PlanCandidate,
    contract: &ProjectionContract,
    reports: &[&[u8]],
    approval: Option<&SemanticApproval>,
    now_unix: u64,
) -> PlanResult<ReadOnlyPlan> {
    let evidence = examine_cases(&candidate, contract, reports)?;
    let evidence_hash = evidence.sdk_evidence_hash;
    let needs_approval =
        !candidate.actions.is_empty() || candidate.report.impact.connector_order_conflicts > 0;
    let approval_hash = if needs_approval {
        let approval = approval.ok_or_else(|| refuse(PlanCode::ApprovalRequired))?;
        if approval.expires_at_unix <= now_unix {
            return Err(refuse(PlanCode::ApprovalExpired));
        }
        if approval.schema_version != 1
            || approval.target != candidate.target
            || approval.candidate_hash != candidate.report.candidate_hash
            || approval.actions_hash != candidate.actions_hash
            || approval.case_request_hash != candidate.report.case_request_hash
            || approval.impact_hash != candidate.report.impact_hash
            || approval.sdk_evidence_hash != evidence_hash
        {
            return Err(refuse(PlanCode::ApprovalRequired));
        }
        Some(digest(approval)?)
    } else {
        None
    };
    let mut candidate = candidate;
    candidate.report.browser_impact = Some(evidence.browser_impact);
    candidate.report.plan_hash = Some(digest(&(
        &candidate.report.candidate_hash,
        &evidence_hash,
        &approval_hash,
    ))?);
    candidate.report.review_state = if needs_approval {
        "provisional_reviewed"
    } else {
        "provisional_no_change"
    };
    Ok(ReadOnlyPlan { candidate })
}

#[cfg(test)]
#[path = "planner_tests.rs"]
mod tests;
