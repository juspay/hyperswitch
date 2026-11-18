//! Exact full-inventory comparison. A before image is not proof that a timed-out request stopped.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    apply_contract::*,
    journal::{PreparedRequest, RestrictedJson, SecretGuard},
    planner_types::{ActionKind, PlannedAction},
    raw_json::Json,
    types::CompleteInventory,
    Digest, Name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestDisposition {
    Committed,
    RejectedNoChange,
}

/// Issued only by exact complete-state comparison under the selected server contract.
pub struct ObservedResolution {
    pub(super) request: Digest,
    pub(super) disposition: RequestDisposition,
    pub(super) inventory: Digest,
    pub(super) images: RestrictedJson,
    pub(super) contract: Digest,
}
#[derive(Clone)]
struct Context {
    id: Option<Name>,
    override_id: Option<Name>,
    raw: Json,
}
#[derive(Clone)]
pub(super) struct World {
    contexts: BTreeMap<Vec<u8>, Context>,
    defaults: BTreeMap<String, Json>,
    dimensions: BTreeMap<String, Json>,
    fixed: Digest,
    created_defaults: BTreeSet<String>,
}
fn condition(row: &Json) -> ApplyResult<Vec<u8>> {
    checked(checked(row.get("value"))?.canonical())
}
fn normalized(row: &Json, ignored: &BTreeSet<String>, extra: &[&str]) -> ApplyResult<Vec<u8>> {
    let fields = checked(row.object())?
        .iter()
        .filter(|(k, _)| !ignored.contains(*k) && !extra.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    checked(checked(Json::members(&fields))?.canonical())
}
impl World {
    pub(super) fn read(
        inventory: &CompleteInventory,
        contract: &CertifiedMutationTarget,
    ) -> ApplyResult<Self> {
        contract.inventory(inventory)?;
        Self::archived(&retained_images(inventory, &SecretGuard::default())?)
    }
    /// Durable images are read only through the authenticated, checksum-checked journal reader.
    /// This reconstructs comparison state, never a CompleteInventory/discovery certificate.
    pub(super) fn archived(images: &RestrictedJson) -> ApplyResult<Self> {
        let data = super::recovery_data::Images::parse(images)?;
        let mut contexts = BTreeMap::new();
        let mut override_ids = BTreeMap::new();
        for row in data.contexts.values() {
            let id: Name = checked(checked(row.get("id"))?.string())?
                .try_into()
                .map_err(|_| fail(ApplyCode::InvalidInput))?;
            let override_id: Name = checked(checked(row.get("override_id"))?.string())?
                .try_into()
                .map_err(|_| fail(ApplyCode::InvalidInput))?;
            let values = checked(checked(row.get("override"))?.canonical())?;
            if override_ids
                .insert(override_id.clone(), values.clone())
                .is_some_and(|old| old != values)
                || contexts
                    .insert(
                        condition(row)?,
                        Context {
                            id: Some(id),
                            override_id: Some(override_id),
                            raw: row.clone(),
                        },
                    )
                    .is_some()
            {
                return Err(fail(ApplyCode::ConcurrentChange));
            }
        }
        let fixed = hash(&(
            normalized(&data.workspace, &BTreeSet::new(), &["config_version"])?,
            data.experiments
                .iter()
                .map(|(k, v)| Ok((k, checked(v.canonical())?)))
                .collect::<ApplyResult<Vec<_>>>()?,
            data.groups
                .iter()
                .map(|(k, v)| Ok((k, checked(v.canonical())?)))
                .collect::<ApplyResult<Vec<_>>>()?,
        ))?;
        Ok(Self {
            contexts,
            defaults: data.defaults,
            dimensions: data.dimensions,
            fixed,
            created_defaults: BTreeSet::new(),
        })
    }
    pub(super) fn advance(
        &self,
        actions: &[PlannedAction],
        indices: &[usize],
    ) -> ApplyResult<Self> {
        let mut next = self.clone();
        for index in indices {
            let a = actions
                .get(*index)
                .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
            match a.kind {
                ActionKind::Create => {
                    let row = a
                        .after
                        .as_ref()
                        .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                    if next
                        .contexts
                        .insert(
                            condition(row)?,
                            Context {
                                id: None,
                                override_id: None,
                                raw: row.clone(),
                            },
                        )
                        .is_some()
                    {
                        return Err(fail(ApplyCode::ConcurrentChange));
                    }
                }
                ActionKind::Replace | ActionKind::Delete => {
                    let before = a
                        .before
                        .as_ref()
                        .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                    let key = condition(before)?;
                    let old = next
                        .contexts
                        .remove(&key)
                        .ok_or_else(|| fail(ApplyCode::ConcurrentChange))?;
                    if old.id != a.server_id {
                        return Err(fail(ApplyCode::ConcurrentChange));
                    }
                    if a.kind == ActionKind::Replace {
                        let row = a
                            .after
                            .as_ref()
                            .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                        if condition(row)? != key {
                            return Err(fail(ApplyCode::InvalidInput));
                        }
                        next.contexts.insert(
                            key,
                            Context {
                                id: old.id,
                                override_id: None,
                                raw: row.clone(),
                            },
                        );
                    }
                }
                ActionKind::ExtendDimension => {
                    let name = a
                        .server_id
                        .as_ref()
                        .ok_or_else(|| fail(ApplyCode::InvalidInput))?
                        .as_str();
                    if !next.dimensions.contains_key(name) {
                        return Err(fail(ApplyCode::ConcurrentChange));
                    }
                    next.dimensions.insert(
                        name.to_owned(),
                        a.after
                            .clone()
                            .ok_or_else(|| fail(ApplyCode::InvalidInput))?,
                    );
                }
                ActionKind::CreateDefault => {
                    let row = a
                        .after
                        .as_ref()
                        .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
                    let key = checked(checked(row.get("key"))?.string())?;
                    next.created_defaults.insert(key.clone());
                    if next.defaults.insert(key, row.clone()).is_some() {
                        return Err(fail(ApplyCode::ConcurrentChange));
                    }
                }
            }
        }
        // Bound each retained direct-state event before any dispatch. No truncated images.
        if next.bytes()? > 1024 * 1024 {
            return Err(fail(ApplyCode::LimitExceeded));
        }
        Ok(next)
    }
    fn bytes(&self) -> ApplyResult<usize> {
        checked(
            self.contexts
                .values()
                .map(|r| r.raw.raw().len())
                .chain(
                    self.defaults
                        .values()
                        .chain(self.dimensions.values())
                        .map(|r| r.raw().len()),
                )
                .try_fold(0usize, |a, b| a.checked_add(b).ok_or(())),
        )
    }
    pub(super) fn matches(
        &self,
        actual: &Self,
        contract: &CertifiedMutationTarget,
    ) -> ApplyResult<bool> {
        if self.fixed != actual.fixed
            || self.contexts.keys().ne(actual.contexts.keys())
            || self.defaults.keys().ne(actual.defaults.keys())
            || self.dimensions.keys().ne(actual.dimensions.keys())
        {
            return Ok(false);
        }
        for (key, wanted) in &self.contexts {
            let got = actual
                .contexts
                .get(key)
                .ok_or_else(|| fail(ApplyCode::ConcurrentChange))?;
            // A new row's ID/override ID/weight are server-generated. Existing rows retain
            // their exact ID and weight; replacement alone changes the derived override ID.
            let generated = if wanted.id.is_none() {
                &["id", "override_id", "weight"][..]
            } else {
                &["id", "override_id"][..]
            };
            if wanted
                .id
                .as_ref()
                .is_some_and(|id| got.id.as_ref() != Some(id))
                || wanted
                    .override_id
                    .as_ref()
                    .is_some_and(|id| got.override_id.as_ref() != Some(id))
                || normalized(&wanted.raw, &contract.context_audit, generated)?
                    != normalized(&got.raw, &contract.context_audit, generated)?
            {
                return Ok(false);
            }
        }
        for (wanted, got, audit) in [
            (&self.defaults, &actual.defaults, &contract.default_audit),
            (
                &self.dimensions,
                &actual.dimensions,
                &contract.dimension_audit,
            ),
        ] {
            for (key, row) in wanted {
                let actual = got
                    .get(key)
                    .ok_or_else(|| fail(ApplyCode::ConcurrentChange))?;
                let generated = if self.created_defaults.contains(key) {
                    for name in [
                        "value_validation_function_name",
                        "value_compute_function_name",
                    ] {
                        if checked(actual.optional(name))?.is_some_and(|v| !v.null()) {
                            return Ok(false);
                        }
                    }
                    &[
                        "value_validation_function_name",
                        "value_compute_function_name",
                    ][..]
                } else {
                    &[][..]
                };
                if normalized(row, audit, generated)? != normalized(actual, audit, generated)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
pub(super) struct RequestObservation<'a> {
    pub request: &'a PreparedRequest,
    pub before: &'a World,
    pub after: &'a World,
    pub before_version: &'a Option<Name>,
    pub status: Option<u16>,
    pub response_version: &'a Option<Name>,
}
pub(super) fn resolution(
    input: RequestObservation<'_>,
    inventory: &CompleteInventory,
    contract: &CertifiedMutationTarget,
    guard: &SecretGuard,
) -> ApplyResult<ObservedResolution> {
    let RequestObservation {
        request,
        before,
        after,
        before_version,
        status,
        response_version,
    } = input;
    let observed = World::read(inventory, contract)?;
    let disposition = if after.matches(&observed, contract)? {
        if response_version
            .as_ref()
            .is_some_and(|v| inventory.proof().version.as_ref() != Some(v))
        {
            return Err(fail(ApplyCode::CommitUnknown));
        }
        RequestDisposition::Committed
    } else if before.matches(&observed, contract)?
        && inventory.proof().version == *before_version
        && status.is_some_and(|s| contract.rollback_statuses.contains(&s))
    {
        RequestDisposition::RejectedNoChange
    } else {
        return Err(fail(ApplyCode::CommitUnknown));
    };
    let images = retained_images(inventory, guard)?;
    Ok(ObservedResolution {
        request: checked(request.hash())?,
        disposition,
        inventory: inventory.proof().inventory_hash.clone(),
        images,
        contract: contract.id.clone(),
    })
}
pub(super) fn retained_images(
    inventory: &CompleteInventory,
    guard: &SecretGuard,
) -> ApplyResult<RestrictedJson> {
    let rows = serde_json::json!({
        "proof":inventory.proof(),
        "contexts":inventory.contexts().values().map(|r|r.raw_json()).collect::<Vec<_>>(),
        "defaults":inventory.defaults().values().map(|r|r.raw_json()).collect::<Vec<_>>(),
        "dimensions":inventory.dimensions().values().map(|r|r.raw_json()).collect::<Vec<_>>(),
        "experiments":inventory.experiments().values().map(|r|r.raw_json()).collect::<Vec<_>>(),
        "experiment_groups":inventory.experiment_groups().values().map(|r|r.raw_json()).collect::<Vec<_>>(),
        "workspace":inventory.workspace().raw_json(),
        "versions":inventory.version_observations().iter().map(|r|r.raw_json()).collect::<Vec<_>>()
    });
    let bytes = checked(serde_json::to_vec(&rows))?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(fail(ApplyCode::LimitExceeded));
    }
    checked(RestrictedJson::parse(&bytes, guard))
}
