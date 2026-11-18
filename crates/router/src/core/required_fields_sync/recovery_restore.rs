//! Three-way inverse at the delegated attribute boundary, never an exported-workspace rollback.
use std::collections::{BTreeMap, BTreeSet};

use super::{
    super::{
        canonical_hash,
        planner_types::{self, ActionKind},
        HashDomain, ManagedKey, ManagedValue,
    },
    *,
};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreOrigin {
    RunBefore,
    FirstAdoption,
}

fn condition(row: &Json) -> ApplyResult<Vec<u8>> {
    checked(checked(row.get("value"))?.canonical())
}
fn values(row: Option<&Json>) -> ApplyResult<BTreeMap<String, Json>> {
    row.map(|r| checked(checked(r.get("override"))?.object()).cloned())
        .transpose()
        .map(Option::unwrap_or_default)
}
fn managed(key: &str, value: &Json) -> ApplyResult<ManagedValue> {
    if key.ends_with("._is_required") {
        Ok(ManagedValue::Required(checked(value.boolean())?))
    } else if key.ends_with("._dropdown_options") {
        Ok(ManagedValue::Options(
            checked(value.array())?
                .iter()
                .map(|v| checked(v.string()))
                .collect::<ApplyResult<_>>()?,
        ))
    } else {
        Err(fail(ApplyCode::InvalidInput))
    }
}
fn original_grant(
    snapshot: &journal::PreparedSnapshot,
    action: &PlannedAction,
) -> ApplyResult<BTreeSet<ManagedKey>> {
    let root = checked(Json::parse(snapshot.contents().bytes()))?;
    let ownership: OwnershipManifest = data::decode(checked(root.get("ownership"))?)?;
    let key = action
        .condition
        .as_ref()
        .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
    let attrs = if action.kind == ActionKind::Create {
        if action
            .ownership_intent
            .as_ref()
            .is_none_or(|i| !snapshot.pending_create_ownership().contains(i))
        {
            return Err(fail(ApplyCode::JournalUnavailable));
        }
        ownership
            .creations
            .iter()
            .find(|g| &g.condition == key)
            .map(|g| &g.attributes)
    } else {
        ownership
            .grants
            .iter()
            .find(|g| &g.condition == key && Some(&g.server_id) == action.server_id.as_ref())
            .map(|g| &g.attributes)
    };
    attrs
        .cloned()
        .ok_or_else(|| fail(ApplyCode::NotAuthoritative))
}
fn relevant(row: &Json) -> ApplyResult<Vec<u8>> {
    let fields = checked(row.object())?
        .iter()
        .filter(|(k, _)| {
            ![
                "created_at",
                "last_modified_at",
                "created_by",
                "last_modified_by",
                "change_reason",
                "override_id",
            ]
            .contains(&k.as_str())
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    checked(checked(Json::members(&fields))?.canonical())
}
impl RecoveryObservation {
    pub fn restore_draft(
        &self,
        inputs: RecoveryPlanning<'_>,
        origin: RestoreOrigin,
        guard: &SecretGuard,
    ) -> ApplyResult<RestorationDraft> {
        self.current(inputs.inventory)?;
        self.new_generation(&inputs.intent)?;
        let snapshot = &self.source.run.snapshot;
        // A baseline alone has no confirmed after chain for intervening releases. Do not infer
        // ownership of their changes. Recover the adoption run or supply reviewed old source.
        if matches!(origin, RestoreOrigin::FirstAdoption)
            && checked(snapshot.hash())? != checked(self.source.run.first_adoption.hash())?
        {
            return Err(fail(ApplyCode::JournalUnavailable));
        }
        let live = Images::parse(&self.live_images)?;
        let mut desired = inputs.desired.clone();
        desired.desired.contexts.clear();
        let mut creations = BTreeMap::new();
        let mut touched = BTreeSet::new();
        let mut residuals = Vec::new();
        for (action, images) in &self.applied {
            if matches!(
                action.kind,
                ActionKind::CreateDefault | ActionKind::ExtendDimension
            ) {
                // Explicit residuals remain in the current inventory and every impact case.
                residuals.push(checked(planner_types::digest(&checked(action.summary())?))?);
                continue;
            }
            let key = action
                .condition
                .as_ref()
                .ok_or_else(|| fail(ApplyCode::InvalidInput))?;
            if !touched.insert(key.clone()) {
                return Err(fail(ApplyCode::InvalidInput));
            }
            let old_attributes = original_grant(snapshot, action)?;
            let row_key = checked(checked(planner_types::json(key))?.canonical())?;
            let current = live.contexts.get(&row_key);
            let confirmed = Images::parse(images)?;
            let after = confirmed.contexts.get(&row_key);
            let before = action.before.as_ref();
            // Recover create identity only from durable observed rows matched by the full condition.
            // A changed/deleted ID on an existing row is not a new ownership grant.
            if let (Some(after), Some(current)) = (after, current) {
                if !data::same(
                    Some(checked(current.get("id"))?),
                    Some(checked(after.get("id"))?),
                )? {
                    return Err(fail(ApplyCode::ConcurrentChange));
                }
            }
            if action.kind == ActionKind::Delete && current.is_some() {
                return Err(fail(ApplyCode::ConcurrentChange));
            }
            let grant = inputs.ownership.grants.iter().find(|g| &g.condition == key);
            let creation = inputs
                .ownership
                .creations
                .iter()
                .find(|g| &g.condition == key);
            let attributes = if let Some(current) = current {
                let grant = grant.ok_or_else(|| fail(ApplyCode::NotAuthoritative))?;
                if checked(checked(current.get("id"))?.string())? != grant.server_id.as_str()
                    || checked(planner_types::hash(current))? != grant.observed_row_hash
                {
                    return Err(fail(ApplyCode::ConcurrentChange));
                }
                &grant.attributes
            } else {
                &creation
                    .ok_or_else(|| fail(ApplyCode::NotAuthoritative))?
                    .attributes
            };
            let before_values = values(before)?;
            let after_values = values(after)?;
            let mut current_values = values(current)?;
            let changed: BTreeSet<_> = before_values
                .keys()
                .chain(after_values.keys())
                .cloned()
                .collect();
            for name in changed {
                let old = before_values.get(&name);
                let applied = after_values.get(&name);
                if data::same(old, applied)? {
                    if let Ok(key) = ManagedKey::try_from(name.clone()) {
                        if old_attributes.contains(&key)
                            && !data::same(current_values.get(&name), old)?
                        {
                            return Err(fail(ApplyCode::ConcurrentChange));
                        }
                    }
                    continue;
                }
                let managed_key: ManagedKey = name
                    .clone()
                    .try_into()
                    .map_err(|_| fail(ApplyCode::InvalidInput))?;
                if !old_attributes.contains(&managed_key) || !attributes.contains(&managed_key) {
                    return Err(fail(ApplyCode::NotAuthoritative));
                }
                let current = current_values.get(&name);
                if !data::same(current, old)? && !data::same(current, applied)? {
                    return Err(fail(ApplyCode::ConcurrentChange));
                }
                if let Some(value) = old {
                    current_values.insert(name, value.clone());
                } else {
                    current_values.remove(&name);
                }
            }
            let wanted = attributes
                .iter()
                .filter_map(|key| {
                    let name = planner_types::key_text(key);
                    match name {
                        Ok(name) => current_values
                            .get(&name)
                            .map(|v| managed(&name, v).map(|v| (key.clone(), v))),
                        Err(_) => Some(Err(fail(ApplyCode::InvalidInput))),
                    }
                })
                .collect::<ApplyResult<BTreeMap<_, _>>>()?;
            if current_values.is_empty() && current.is_some() {
                // A sync-created row with a changed description/metadata cannot be deleted. If
                // unrelated keys exist, the replacement retains that row and its current metadata.
                if !grant.is_some_and(|g| g.allow_row_deletion)
                    || action.kind != ActionKind::Create
                    || relevant(current.ok_or_else(|| fail(ApplyCode::InvalidInput))?)?
                        != relevant(after.ok_or_else(|| fail(ApplyCode::JournalUnavailable))?)?
                {
                    return Err(fail(ApplyCode::ConcurrentChange));
                }
            }
            if action.kind == ActionKind::Delete {
                let saved = before.ok_or_else(|| fail(ApplyCode::JournalUnavailable))?;
                if condition(saved)? != row_key
                    || before_values.is_empty()
                    || before_values.keys().any(|k| {
                        k.clone()
                            .try_into()
                            .ok()
                            .is_none_or(|k: ManagedKey| !attributes.contains(&k))
                    })
                {
                    return Err(fail(ApplyCode::NotAuthoritative));
                }
                // The certified create wire carries these three attributes only. Unknown saved
                // metadata cannot be silently lost during recreation.
                if checked(saved.object())?.keys().any(|k| {
                    ![
                        "id",
                        "override_id",
                        "weight",
                        "created_at",
                        "last_modified_at",
                        "created_by",
                        "last_modified_by",
                        "change_reason",
                        "value",
                        "override",
                        "description",
                    ]
                    .contains(&k.as_str())
                }) {
                    return Err(fail(ApplyCode::UnsupportedServerContract));
                }
                let row = checked(Json::members(
                    &["value", "override", "description"]
                        .into_iter()
                        .map(|k| Ok((k.to_owned(), checked(saved.get(k))?.clone())))
                        .collect::<ApplyResult<_>>()?,
                ))?;
                creations.insert(key.clone(), row);
            } else if current.is_none() && !current_values.is_empty() {
                return Err(fail(ApplyCode::ConcurrentChange));
            }
            if !wanted.is_empty() {
                desired.desired.contexts.insert(key.clone(), wanted);
            }
        }
        if inputs
            .ownership
            .grants
            .iter()
            .any(|g| !touched.contains(&g.condition))
            || inputs
                .ownership
                .creations
                .iter()
                .any(|g| !touched.contains(&g.condition))
        {
            return Err(fail(ApplyCode::NotAuthoritative));
        }
        let bundle = serde_json::json!({"schema_version":1,"desired":desired.desired,"catalog":desired.catalog,
            "recovery":{"origin":origin,"source":self.report,"basis_bundle":inputs.desired.bundle_hash(),"residuals":residuals}});
        desired.bundle_hash = checked(canonical_hash(HashDomain::Bundle, &bundle))?;
        let bundle = checked(RestrictedJson::parse(
            &checked(serde_json::to_vec(&bundle))?,
            guard,
        ))?;
        Ok(RestorationDraft {
            binding: draft_binding(&inputs)?,
            observation: hash(&self.report)?,
            desired,
            bundle,
            creations,
            residuals,
        })
    }
}
fn draft_binding(inputs: &RecoveryPlanning<'_>) -> ApplyResult<Digest> {
    let mut ownership = inputs.ownership.clone();
    ownership.retirement = None;
    checked(planner_types::digest(&(
        inputs.inventory.proof().inventory_hash.clone(),
        inputs.desired.bundle_hash(),
        ownership,
        inputs.prerequisites,
        &inputs.intent,
        checked(plan::projection_hash(inputs.contract))?,
        &inputs.limits,
    )))
}
/// Preview exposes the exact bundle hash needed for the existing explicit retirement manifest.
/// It is not accepted/executable until the normal planner and approval checks run.
pub struct RestorationDraft {
    binding: Digest,
    observation: Digest,
    desired: VerifiedDesired,
    bundle: RestrictedJson,
    creations: BTreeMap<super::super::ContextKey, Json>,
    residuals: Vec<Digest>,
}
impl RestorationDraft {
    pub fn bundle_hash(&self) -> &Digest {
        self.desired.bundle_hash()
    }
    pub fn prepare(
        self,
        observation: &RecoveryObservation,
        inputs: RecoveryPlanning<'_>,
    ) -> ApplyResult<RecoveryCandidate> {
        observation.current(inputs.inventory)?;
        if self.binding != draft_binding(&inputs)? || self.observation != hash(&observation.report)?
        {
            return Err(fail(ApplyCode::ConcurrentChange));
        }
        let candidate = checked(plan::prepare_restoration(
            inputs.inventory,
            &self.desired,
            inputs.ownership,
            inputs.prerequisites,
            inputs.intent,
            inputs.contract,
            inputs.limits,
            &self.creations,
        ))?;
        if candidate.actions().iter().any(|a| {
            matches!(
                a.kind,
                ActionKind::CreateDefault | ActionKind::ExtendDimension
            )
        }) {
            return Err(fail(ApplyCode::InvalidInput));
        }
        observation.bind(
            candidate,
            Operation::Restore,
            Some(self.bundle),
            self.residuals,
        )
    }
}
