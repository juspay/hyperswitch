//! Bounded equality partitions and connector-order analysis under a sealed projection contract.
//! This reference model does not claim to render a browser, apply prefill or build a payment body.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    planner_types::*,
    preflight,
    raw_json::Json,
    types::{CompleteInventory, DimensionKind},
    Name,
};

#[derive(Clone)]
pub(super) struct ContextRow {
    pub id: String,
    pub condition: Json,
    pub values: Json,
    pub created: Option<(i64, u32)>,
}

pub(super) fn rows(inventory: &CompleteInventory) -> PlanResult<Vec<ContextRow>> {
    inventory
        .contexts()
        .values()
        .map(|row| {
            let created = checked(checked(row.raw.get("created_at"))?.string())?;
            let created = superposition_sdk::primitives::DateTime::from_str(
                &created,
                superposition_sdk::primitives::DateTimeFormat::DateTime,
            )
            .map_err(|_| refuse(PlanCode::UnknownProjection))?;
            Ok(ContextRow {
                id: row.projection().id.as_str().to_owned(),
                condition: checked(row.raw.get("value"))?.clone(),
                values: checked(row.raw.get("override"))?.clone(),
                created: Some((created.secs(), created.subsec_nanos())),
            })
        })
        .collect()
}

pub(super) fn overlap(a: &Json, b: &Json) -> PlanResult<bool> {
    let a = checked(a.object())?;
    let b = checked(b.object())?;
    for value in a.values().chain(b.values()) {
        if !value.raw().starts_with('"') && !matches!(value.raw(), "true" | "false") {
            return Err(refuse(PlanCode::UnsupportedPredicate));
        }
    }
    for (key, value) in a {
        if !value.scalar() || value.null() {
            return Err(refuse(PlanCode::UnsupportedPredicate));
        }
        if let Some(other) = b.get(key) {
            if !other.scalar() || other.null() {
                return Err(refuse(PlanCode::UnsupportedPredicate));
            }
            if checked(value.canonical())? != checked(other.canonical())? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub(super) fn order(
    rows: &[ContextRow],
    inventory: &CompleteInventory,
) -> PlanResult<Vec<ContextRow>> {
    let mut ordered = Vec::new();
    let mut used_positions = BTreeMap::new();
    for row in rows {
        let mut weight = 0_u64;
        for (key, value) in checked(row.condition.object())? {
            if !value.raw().starts_with('"') && !matches!(value.raw(), "true" | "false") {
                return Err(refuse(PlanCode::UnsupportedPredicate));
            }
            let name: Name = key
                .clone()
                .try_into()
                .map_err(|_| refuse(PlanCode::UnsupportedPredicate))?;
            let dimension = inventory
                .dimensions()
                .get(&name)
                .ok_or_else(|| refuse(PlanCode::MissingDimension))?;
            let position = dimension.projection().position;
            if !(0..=30).contains(&position)
                || !matches!(dimension.projection().kind, DimensionKind::Regular)
                || !checked(checked(dimension.raw.get("dependency_graph"))?.object())?.is_empty()
            {
                return Err(refuse(PlanCode::UnknownProjection));
            }
            preflight::no_functions(&dimension.raw)?;
            if used_positions
                .insert(position, name.clone())
                .is_some_and(|previous| previous != name)
            {
                return Err(refuse(PlanCode::UnknownProjection));
            }
            weight += 1_u64 << position;
        }
        for (other_weight, other) in &ordered {
            let other: &ContextRow = other;
            // Unknown create time or a timestamp tie cannot choose a winner for overlapping rows.
            if *other_weight == weight
                && (row.created.is_none()
                    || other.created.is_none()
                    || row.created == other.created)
                && overlap(&row.condition, &other.condition)?
                && checked(row.values.canonical())? != checked(other.values.canonical())?
            {
                return Err(refuse(PlanCode::UnknownProjection));
            }
        }
        if let Some(observed) = inventory
            .contexts()
            .values()
            .find(|observed| observed.projection().id.as_str() == row.id)
        {
            if checked(checked(observed.raw.get("weight"))?.string())?
                .parse::<u64>()
                .ok()
                != Some(weight)
            {
                return Err(refuse(PlanCode::UnknownProjection));
            }
        }
        ordered.push((weight, row.clone()));
    }
    ordered.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.created.cmp(&b.1.created))
            .then(a.1.id.cmp(&b.1.id))
    });
    Ok(ordered.into_iter().map(|(_, row)| row).collect())
}

fn collisions(rows: &[ContextRow], contract: &ProjectionContract) -> PlanResult<()> {
    let mut seen = BTreeMap::new();
    for row in rows {
        let id = (contract.override_identity)(row.values.raw())?;
        if id.is_empty() || id.len() > 256 {
            return Err(refuse(PlanCode::UnknownProjection));
        }
        let canonical = checked(row.values.canonical())?;
        if seen
            .insert(id, canonical.clone())
            .is_some_and(|old| old != canonical)
        {
            return Err(refuse(PlanCode::OrderedValueCollision));
        }
    }
    Ok(())
}

fn partitions(rows: &[ContextRow], limit: usize) -> PlanResult<Vec<BTreeMap<String, Json>>> {
    let mut axes: BTreeMap<String, BTreeMap<Vec<u8>, Json>> = BTreeMap::new();
    for row in rows {
        for (key, value) in checked(row.condition.object())? {
            axes.entry(key.clone())
                .or_default()
                .insert(checked(value.canonical())?, value.clone());
        }
    }
    let mut result = vec![BTreeMap::new()];
    for (key, values) in axes {
        let mut choices = vec![None];
        let boolean = values.values().all(|v| matches!(v.raw(), "true" | "false"));
        let strings = values.values().all(|v| v.raw().starts_with('"'));
        if !boolean && !strings {
            return Err(refuse(PlanCode::UnsupportedPredicate));
        }
        if boolean {
            choices.extend([Some(json(&false)?), Some(json(&true)?)]);
        } else {
            choices.extend(values.values().cloned().map(Some));
            let mut suffix = 0;
            loop {
                let other = json(&format!("__required_fields_other_{suffix}"))?;
                if !values.contains_key(&checked(other.canonical())?) {
                    choices.push(Some(other));
                    break;
                }
                suffix += 1;
            }
        }
        if result
            .len()
            .checked_mul(choices.len())
            .is_none_or(|n| n > limit)
        {
            return Err(refuse(PlanCode::LimitExceeded));
        }
        let mut next = Vec::new();
        for base in result {
            for choice in &choices {
                let mut case = base.clone();
                if let Some(value) = choice {
                    case.insert(key.clone(), value.clone());
                }
                next.push(case);
            }
        }
        result = next;
    }
    Ok(result)
}

pub(super) fn resolve(
    rows: &[ContextRow],
    defaults: &BTreeMap<String, Json>,
    case: &BTreeMap<String, Json>,
) -> PlanResult<BTreeMap<String, Json>> {
    let mut values = defaults.clone();
    for row in rows {
        let mut matches = true;
        for (key, value) in checked(row.condition.object())? {
            matches &= case.get(key).map(|v| checked(v.canonical())).transpose()?
                == Some(checked(value.canonical())?);
        }
        if matches {
            values.extend(checked(row.values.object())?.clone());
        }
    }
    Ok(values)
}

fn config(rows: &[ContextRow], defaults: &BTreeMap<String, Json>) -> PlanResult<Json> {
    let mut contexts = Vec::new();
    let mut overrides = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let reference = format!("projection-{index}");
        // These keys are local case-artifact references, never server identities/requests.
        contexts.push(checked(Json::members(&BTreeMap::from([
            ("id".to_owned(), json(&row.id)?),
            ("condition".to_owned(), row.condition.clone()),
            (
                "override_with_keys".to_owned(),
                json(&vec![reference.clone()])?,
            ),
            ("weight".to_owned(), json(&0)?),
            ("priority".to_owned(), json(&0)?),
        ])))?);
        overrides.insert(reference, row.values.clone());
    }
    let context_bytes = format!(
        "[{}]",
        contexts.iter().map(Json::raw).collect::<Vec<_>>().join(",")
    );
    checked(Json::members(&BTreeMap::from([
        (
            "contexts".to_owned(),
            checked(Json::parse(context_bytes.as_bytes()))?,
        ),
        ("overrides".to_owned(), checked(Json::members(&overrides))?),
        (
            "default_configs".to_owned(),
            checked(Json::members(defaults))?,
        ),
        ("dimensions".to_owned(), json(&serde_json::json!({}))?),
    ])))
}

fn check_export(
    inventory: &CompleteInventory,
    rows: &[ContextRow],
    defaults: &BTreeMap<String, Json>,
) -> PlanResult<()> {
    let head = inventory
        .version_observations()
        .first()
        .ok_or_else(|| refuse(PlanCode::UnknownProjection))?;
    let versions = checked(checked(head.body.get("data"))?.array())?;
    let Some(version) = versions.first() else {
        return if rows.is_empty() && defaults.is_empty() {
            Ok(())
        } else {
            Err(refuse(PlanCode::UnknownProjection))
        };
    };
    let export = checked(version.get("config"))?;
    let contexts = checked(checked(export.get("contexts"))?.array())?;
    let overrides = checked(checked(export.get("overrides"))?.object())?;
    let exported_defaults = checked(export.get("default_configs"))?;
    if contexts.len() != rows.len()
        || checked(exported_defaults.canonical())?
            != checked(checked(Json::members(defaults))?.canonical())?
    {
        return Err(refuse(PlanCode::UnknownProjection));
    }
    for (exported, row) in contexts.iter().zip(rows) {
        let reference = checked(checked(exported.get("override_with_keys"))?.array())?;
        let source = inventory
            .contexts()
            .values()
            .find(|v| v.projection().id.as_str() == row.id)
            .ok_or_else(|| refuse(PlanCode::UnknownProjection))?;
        if checked(checked(exported.get("id"))?.string())? != row.id
            || reference.len() != 1
            || checked(
                reference
                    .first()
                    .ok_or_else(|| refuse(PlanCode::UnknownProjection))?
                    .string(),
            )? != source.projection().override_id.as_str()
            || checked(checked(exported.get("condition"))?.canonical())?
                != checked(row.condition.canonical())?
        {
            return Err(refuse(PlanCode::UnknownProjection));
        }
        let values = overrides
            .get(source.projection().override_id.as_str())
            .ok_or_else(|| refuse(PlanCode::UnknownProjection))?;
        if checked(values.canonical())? != checked(row.values.canonical())? {
            return Err(refuse(PlanCode::UnknownProjection));
        }
    }
    Ok(())
}

fn permutations(
    values: &[Name],
    prefix: Vec<Name>,
    result: &mut Vec<Vec<Name>>,
    limit: usize,
) -> PlanResult<()> {
    if values.is_empty() {
        if result.len() >= limit {
            return Err(refuse(PlanCode::LimitExceeded));
        }
        result.push(prefix);
        return Ok(());
    }
    for (i, value) in values.iter().enumerate() {
        let mut prefix = prefix.clone();
        prefix.push(value.clone());
        let remaining: Vec<_> = values
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, v)| v.clone())
            .collect();
        permutations(&remaining, prefix, result, limit)?;
    }
    Ok(())
}

struct DescriptorContract<'a> {
    fields: &'a BTreeSet<Name>,
    desired: &'a VerifiedDesired,
    policy: &'a preflight::PrerequisitePolicy,
}

fn aggregate(
    rows: &[ContextRow],
    defaults: &BTreeMap<String, Json>,
    case: &BTreeMap<String, Json>,
    order: &[Name],
    billing: bool,
    descriptors: &DescriptorContract<'_>,
) -> PlanResult<Json> {
    let DescriptorContract {
        fields,
        desired,
        policy,
    } = *descriptors;
    let mut by_path = BTreeMap::new();
    for connector in order {
        let mut supplied = case.clone();
        supplied.insert("connector".to_owned(), json(&connector.as_str())?);
        let values = resolve(rows, defaults, &supplied)?;
        preflight::validate_fields(&values, fields, desired, policy)?;
        let mut local_paths = BTreeSet::new();
        for field in fields {
            let contract = desired
                .catalog
                .fields
                .get(field)
                .ok_or_else(|| refuse(PlanCode::MissingPresentation))?
                .approved()
                .map_err(|_| refuse(PlanCode::MissingPresentation))?;
            let prefix = format!("dynamic_fields.{}._", field.as_str());
            // validate_fields above permits absence only for an exact declared default creation.
            let required = values
                .get(&format!("{prefix}is_required"))
                .map(|v| checked(v.boolean()))
                .transpose()?
                .unwrap_or(false);
            let forced = billing
                && contract
                    .write_path
                    .as_str()
                    .starts_with("payment_method_data.billing.address.");
            if !required && !forced {
                continue;
            }
            if !local_paths.insert(contract.write_path.clone()) {
                return Err(refuse(PlanCode::ConnectorOrderConflict));
            }
            let required = required
                || forced
                    && contract.write_path.as_str() != "payment_method_data.billing.address.line2";
            let mut descriptor = BTreeMap::new();
            descriptor.insert("required".to_owned(), json(&required)?);
            for (key, value) in values
                .iter()
                .filter(|(key, _)| key.starts_with(&prefix) && !key.ends_with("._is_required"))
            {
                descriptor.insert(key.clone(), value.clone());
            }
            by_path
                .entry(contract.write_path.as_str().to_owned())
                .or_insert(checked(Json::members(&descriptor))?);
        }
    }
    checked(Json::members(&by_path))
}

pub(super) struct Analysis {
    pub counts: ImpactCounts,
    pub request: Json,
    pub reference_hashes: Vec<(super::Digest, super::Digest)>,
    pub rust_reference_hashes: Vec<(super::Digest, super::Digest)>,
    pub browser_case_hashes: Vec<super::Digest>,
    pub browser_expected: Vec<(super::Digest, super::Digest)>,
}

pub(super) struct AnalysisInputs<'a> {
    pub inventory: &'a CompleteInventory,
    pub desired: &'a VerifiedDesired,
    pub policy: &'a preflight::PrerequisitePolicy,
    pub contract: &'a ProjectionContract,
    pub before: &'a [ContextRow],
    pub after: &'a [ContextRow],
    pub live: &'a preflight::LiveState,
    pub fields: &'a BTreeSet<Name>,
    pub limits: &'a PlanLimits,
}

pub(super) fn analyze(inputs: AnalysisInputs<'_>) -> PlanResult<Analysis> {
    let AnalysisInputs {
        inventory,
        desired,
        policy,
        contract,
        before,
        after,
        live,
        fields,
        limits,
    } = inputs;
    let descriptors = DescriptorContract {
        fields,
        desired,
        policy,
    };
    let before = order(before, inventory)?;
    let after = order(after, inventory)?;
    check_export(inventory, &before, &live.defaults)?;
    let all: Vec<_> = before.iter().chain(&after).cloned().collect();
    // Include the observed store as well as the projection: an in-place array reorder can
    // reuse an old override identity even when no second projected row shares it.
    collisions(&all, contract)?;
    let partitions = partitions(&all, limits.max_partitions)?;
    let mut orders = Vec::new();
    let mut sets = BTreeSet::new();
    for set in &contract.connector_sets {
        if set.is_empty() || set.len() > 4 || !sets.insert(set.clone()) {
            return Err(refuse(PlanCode::UnknownProjection));
        }
        permutations(
            &set.iter().cloned().collect::<Vec<_>>(),
            Vec::new(),
            &mut orders,
            limits.max_connector_orders,
        )?;
    }
    for key in desired.desired.contexts.keys() {
        if !contract
            .connector_sets
            .iter()
            .any(|set| set.contains(&key.connector))
        {
            return Err(refuse(PlanCode::UnknownProjection));
        }
    }
    if contract.browser_scenarios.is_empty()
        || orders.is_empty()
        || partitions
            .len()
            .checked_mul(orders.len() * 2)
            .is_none_or(|n| n > limits.max_evaluations)
    {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    let mut counts = ImpactCounts {
        evaluated_partitions: partitions.len(),
        evaluated_connector_orders: orders.len(),
        ..Default::default()
    };
    let mut contexts = Vec::new();
    let mut ids = Vec::new();
    let mut reference_hashes = Vec::new();
    let mut rust_reference_hashes = Vec::new();
    let mut browser_cases = Vec::new();
    let mut browser_case_hashes = Vec::new();
    let mut browser_expected = Vec::new();
    let mut seen_browser = BTreeSet::new();
    for (index, case) in partitions.iter().enumerate() {
        let id = format!("P{index:03}");
        ids.push(id.clone());
        contexts.push(checked(Json::members(case))?);
        let previous = resolve(&before, &live.defaults, case)?;
        let next = resolve(&after, &live.projected_defaults, case)?;
        preflight::validate_fields(&previous, fields, desired, policy)?;
        preflight::validate_fields(&next, fields, desired, policy)?;
        let mut relevant_before = BTreeMap::new();
        let mut relevant_after = BTreeMap::new();
        for field in fields {
            let prefix = format!("dynamic_fields.{}._", field.as_str());
            for (key, value) in previous.iter().filter(|(key, _)| key.starts_with(&prefix)) {
                relevant_before.insert(key.clone(), value.clone());
            }
            for (key, value) in next.iter().filter(|(key, _)| key.starts_with(&prefix)) {
                relevant_after.insert(key.clone(), value.clone());
            }
            let required = format!("{prefix}is_required");
            let was = previous
                .get(&required)
                .map(|v| checked(v.boolean()))
                .transpose()?
                .unwrap_or(false);
            let now = checked(
                next.get(&required)
                    .ok_or_else(|| refuse(PlanCode::MissingDefault))?
                    .boolean(),
            )?;
            counts.required_added += usize::from(!was && now);
            counts.required_removed += usize::from(was && !now);
            let options = format!("{prefix}dropdown_options");
            let a = previous.get(&options);
            let b = next.get(&options);
            if a.map(|v| checked(v.canonical())).transpose()?
                != b.map(|v| checked(v.canonical())).transpose()?
            {
                counts.options_changed += 1;
                if let Some(b) = b {
                    counts.options_emptied += usize::from(checked(b.array())?.is_empty());
                }
                if let (Some(a), Some(b)) = (a, b) {
                    let mut a = checked(a.array())?
                        .iter()
                        .map(|v| checked(v.canonical()))
                        .collect::<PlanResult<Vec<_>>>()?;
                    let mut b = checked(b.array())?
                        .iter()
                        .map(|v| checked(v.canonical()))
                        .collect::<PlanResult<Vec<_>>>()?;
                    a.sort();
                    b.sort();
                    counts.options_reordered += usize::from(a == b);
                }
            }
        }
        reference_hashes.push((
            hash(&checked(Json::members(&relevant_before))?)?,
            hash(&checked(Json::members(&relevant_after))?)?,
        ));
        relevant_before.retain(|key, _| live.defaults.contains_key(key));
        relevant_after.retain(|key, _| live.projected_defaults.contains_key(key));
        rust_reference_hashes.push((
            hash(&checked(Json::members(&relevant_before))?)?,
            hash(&checked(Json::members(&relevant_after))?)?,
        ));
        for billing in [false, true] {
            let mut by_set: BTreeMap<BTreeSet<Name>, BTreeSet<Vec<u8>>> = BTreeMap::new();
            for order in &orders {
                let a = aggregate(&before, &live.defaults, case, order, billing, &descriptors)?;
                let b = aggregate(
                    &after,
                    &live.projected_defaults,
                    case,
                    order,
                    billing,
                    &descriptors,
                )?;
                by_set
                    .entry(order.iter().cloned().collect())
                    .or_default()
                    .insert(checked(b.canonical())?);
                let mut base = case.clone();
                base.remove("connector");
                let input = checked(Json::members(&BTreeMap::from([
                    ("context".to_owned(), checked(Json::members(&base))?),
                    ("connectors".to_owned(), json(order)?),
                    ("billing_force".to_owned(), json(&billing)?),
                    ("scenarios".to_owned(), json(&contract.browser_scenarios)?),
                ])))?;
                let identity = hash(&input)?;
                if seen_browser.insert(checked(input.canonical())?) {
                    browser_case_hashes.push(identity);
                    browser_cases.push(input);
                    browser_expected.push((hash(&a)?, hash(&b)?));
                }
            }
            counts.connector_order_conflicts +=
                by_set.values().filter(|values| values.len() > 1).count();
        }
    }
    let array = |values: &[Json]| {
        checked(Json::parse(
            format!(
                "[{}]",
                values.iter().map(Json::raw).collect::<Vec<_>>().join(",")
            )
            .as_bytes(),
        ))
    };
    let request = checked(Json::members(&BTreeMap::from([
        ("schema_version".to_owned(), json(&1)?),
        (
            "contract_id".to_owned(),
            json(&"required-fields-planner-cases-v1")?,
        ),
        ("case_pack_hash".to_owned(), json(&contract.case_pack)?),
        ("case_ids".to_owned(), json(&ids)?),
        ("fields".to_owned(), json(fields)?),
        ("contexts".to_owned(), array(&contexts)?),
        ("before".to_owned(), config(&before, &live.defaults)?),
        (
            "after".to_owned(),
            config(&after, &live.projected_defaults)?,
        ),
        ("browser_matrix".to_owned(), array(&browser_cases)?),
    ])))?;
    if request.raw().len() > 262144 || browser_cases.len() > 128 {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    Ok(Analysis {
        counts,
        request,
        reference_hashes,
        rust_reference_hashes,
        browser_case_hashes,
        browser_expected,
    })
}
