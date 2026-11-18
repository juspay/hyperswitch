//! Explicit target-bound adoption and durable pending-create intentions. No marker writes.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    planner_types::*,
    raw_json::Json,
    types::{CompleteInventory, TargetIdentity},
    ContextKey, Digest, ManagedKey, Name,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptionGrant {
    pub server_id: Name,
    pub condition: ContextKey,
    pub observed_row_hash: Digest,
    #[serde(deserialize_with = "unique_set")]
    pub attributes: BTreeSet<ManagedKey>,
    pub allow_row_deletion: bool,
    pub approver: Name,
    pub review: Name,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationGrant {
    pub intent_id: Name,
    pub condition: ContextKey,
    #[serde(deserialize_with = "unique_set")]
    pub attributes: BTreeSet<ManagedKey>,
    pub approver: Name,
    pub review: Name,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementManifest {
    pub bundle_hash: Digest,
    pub intent_generation: u64,
    #[serde(deserialize_with = "unique_set")]
    pub server_ids: BTreeSet<Name>,
    pub approver: Name,
    pub review: Name,
}

pub(super) fn unique_set<'de, D, T>(deserializer: D) -> Result<BTreeSet<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Ord,
{
    let values = Vec::<T>::deserialize(deserializer)?;
    let len = values.len();
    let set: BTreeSet<_> = values.into_iter().collect();
    if set.len() != len {
        return Err(serde::de::Error::custom("duplicate set member"));
    }
    Ok(set)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnershipManifest {
    pub schema_version: u16,
    pub target: TargetIdentity,
    pub owner: Name,
    pub revision: Name,
    /// Reviewed compiled feature scope. A later feature absence cannot reuse this delegation.
    pub source_availability_hash: Digest,
    pub grants: Vec<AdoptionGrant>,
    pub creations: Vec<CreationGrant>,
    pub excluded_conditions: Vec<ContextKey>,
    pub retirement: Option<RetirementManifest>,
}

pub(super) fn context_key(row: &Json) -> PlanResult<Option<ContextKey>> {
    let value = checked(row.get("value"))?;
    Ok(serde_json::from_str(value.raw()).ok())
}

pub fn observed_row_hash(inventory: &CompleteInventory, id: &Name) -> PlanResult<Digest> {
    hash(
        &inventory
            .contexts()
            .get(id)
            .ok_or_else(|| refuse(PlanCode::StaleGrant))?
            .raw,
    )
}

pub(super) struct OwnershipIndex<'a> {
    pub grants: BTreeMap<ContextKey, &'a AdoptionGrant>,
    pub creations: BTreeMap<ContextKey, &'a CreationGrant>,
    pub rows: BTreeMap<ContextKey, Name>,
    pub classifications: BTreeMap<String, usize>,
}

fn vocabulary(condition: &ContextKey, desired: &VerifiedDesired) -> PlanResult<()> {
    for (dimension, value) in checked(json(condition)?.object())? {
        let dimension: Name = dimension
            .clone()
            .try_into()
            .map_err(|_| refuse(PlanCode::BindingMismatch))?;
        let value = checked(value.string())?;
        if desired
            .catalog
            .sdk
            .dimensions
            .get(&dimension)
            .is_none_or(|values| !values.iter().any(|v| v.as_str() == value))
        {
            return Err(refuse(PlanCode::BindingMismatch));
        }
    }
    Ok(())
}

pub(super) fn inspect<'a>(
    inventory: &CompleteInventory,
    desired: &VerifiedDesired,
    manifest: &'a OwnershipManifest,
    intent: &PlanIntent,
) -> PlanResult<OwnershipIndex<'a>> {
    if manifest.schema_version != 1
        || manifest.target != inventory.proof().target
        || manifest.grants.len() + manifest.creations.len() > 512
    {
        return Err(refuse(PlanCode::BindingMismatch));
    }
    if manifest.source_availability_hash != desired.source_availability_hash {
        return Err(refuse(PlanCode::RetirementRequired));
    }
    let mut index = OwnershipIndex {
        grants: BTreeMap::new(),
        creations: BTreeMap::new(),
        rows: BTreeMap::new(),
        classifications: BTreeMap::new(),
    };
    let excluded: BTreeSet<_> = manifest.excluded_conditions.iter().collect();
    if excluded.len() != manifest.excluded_conditions.len() {
        return Err(refuse(PlanCode::DuplicateGrant));
    }
    let mut ids = BTreeSet::new();
    for grant in &manifest.grants {
        vocabulary(&grant.condition, desired)?;
        if grant.attributes.is_empty()
            || !ids.insert(grant.server_id.clone())
            || index
                .grants
                .insert(grant.condition.clone(), grant)
                .is_some()
        {
            return Err(refuse(PlanCode::DuplicateGrant));
        }
        let row = inventory
            .contexts()
            .get(&grant.server_id)
            .ok_or_else(|| refuse(PlanCode::StaleGrant))?;
        if context_key(&row.raw)?.as_ref() != Some(&grant.condition)
            || hash(&row.raw)? != grant.observed_row_hash
        {
            return Err(refuse(PlanCode::StaleGrant));
        }
        if excluded.contains(&grant.condition)
            || desired
                .excluded_connectors
                .contains(&grant.condition.connector)
        {
            return Err(refuse(PlanCode::ExcludedScope));
        }
    }
    ids.clear();
    for grant in &manifest.creations {
        vocabulary(&grant.condition, desired)?;
        if grant.attributes.is_empty()
            || !ids.insert(grant.intent_id.clone())
            || index.grants.contains_key(&grant.condition)
            || index
                .creations
                .insert(grant.condition.clone(), grant)
                .is_some()
        {
            return Err(refuse(PlanCode::DuplicateGrant));
        }
        if excluded.contains(&grant.condition)
            || desired
                .excluded_connectors
                .contains(&grant.condition.connector)
        {
            return Err(refuse(PlanCode::ExcludedScope));
        }
    }
    for (id, row) in inventory.contexts() {
        let key = context_key(&row.raw)?;
        let class = if let Some(key) = key {
            if index.rows.insert(key.clone(), id.clone()).is_some() {
                return Err(refuse(PlanCode::AmbiguousIdentity));
            }
            if excluded.contains(&key) || desired.excluded_connectors.contains(&key.connector) {
                "excluded"
            } else if let Some(grant) = index.grants.get(&key) {
                let owned = grant
                    .attributes
                    .iter()
                    .map(key_text)
                    .collect::<PlanResult<BTreeSet<_>>>()?;
                if checked(checked(row.raw.get("override"))?.object())?
                    .keys()
                    .all(|k| owned.contains(k))
                {
                    "fully_managed"
                } else {
                    "shared"
                }
            } else {
                "adoption_candidate"
            }
        } else if matches!(
            row.projection().condition,
            super::types::ObservedCondition::QuarantinedUnsupportedPredicate
        ) {
            "excluded_unsupported"
        } else {
            "unrelated"
        };
        *index.classifications.entry(class.to_owned()).or_default() += 1;
    }
    for key in desired.desired.contexts.keys() {
        if excluded.contains(key) || desired.excluded_connectors.contains(&key.connector) {
            return Err(refuse(PlanCode::ExcludedScope));
        }
        if index.rows.contains_key(key) && !index.grants.contains_key(key)
            || !index.rows.contains_key(key) && !index.creations.contains_key(key)
        {
            return Err(refuse(PlanCode::OwnershipRequired));
        }
    }
    // A pending create is not an adoption grant for a future/manual row at that condition.
    if index
        .creations
        .keys()
        .any(|key| index.rows.contains_key(key))
    {
        return Err(refuse(PlanCode::OwnershipRequired));
    }
    if desired.desired.contexts.is_empty() && !index.grants.is_empty() {
        let retirement = manifest
            .retirement
            .as_ref()
            .ok_or_else(|| refuse(PlanCode::EmptyDesired))?;
        if retirement.bundle_hash != desired.bundle_hash
            || retirement.intent_generation != intent.generation
            || retirement.server_ids
                != manifest
                    .grants
                    .iter()
                    .map(|g| g.server_id.clone())
                    .collect::<BTreeSet<_>>()
        {
            return Err(refuse(PlanCode::RetirementRequired));
        }
    }
    Ok(index)
}
