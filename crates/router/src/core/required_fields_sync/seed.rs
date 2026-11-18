//! Pure, formatting-preserving local seed editing. This never grants ownership of live rows.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value};

use super::{
    planner_types::{self, refuse, PlanCode, PlanResult, VerifiedDesired},
    preflight::{self, DefaultCreation, DimensionRule, FieldRule, PrerequisitePolicy},
    raw_json::{fingerprint, Json},
    ContextKey, Decision, Digest, ManagedKey, ManagedValue, Name,
};

pub const MAX_SEED_BYTES: usize = 4 * 1024 * 1024;
const MAX_ROWS: usize = 10_000;
const MAX_CHANGES: usize = 10_000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub condition: ContextKey,
    #[serde(deserialize_with = "super::ownership::unique_set")]
    pub attributes: BTreeSet<ManagedKey>,
    /// Absence or exclusion is not retirement permission. Each removed attribute is explicit.
    #[serde(deserialize_with = "super::ownership::unique_set")]
    pub retire: BTreeSet<ManagedKey>,
    pub review: Name,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub grants: Vec<Grant>,
    pub dimensions: BTreeMap<Name, DimensionRule>,
    pub fields: BTreeMap<Name, FieldRule>,
    pub create_defaults: Vec<DefaultCreation>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema_version: u16,
    pub contract_id: Name,
    pub profile_hash: Digest,
    #[serde(deserialize_with = "super::ownership::unique_set")]
    pub router_feature_keys: BTreeSet<Name>,
    #[serde(deserialize_with = "super::ownership::unique_set")]
    pub payment_methods_feature_keys: BTreeSet<Name>,
    pub scope: Decision<Scope>,
}
#[derive(Serialize)]
pub struct Change {
    pub kind: &'static str,
    pub subject: Digest,
    pub before: Digest,
    pub after: Digest,
}
#[derive(Serialize)]
pub struct Report {
    pub schema_version: u16,
    pub profile_hash: Digest,
    pub policy_hash: Digest,
    pub bundle_hash: Digest,
    pub template_hash: Digest,
    pub generated_hash: Digest,
    pub preserved_content_hash: Digest,
    pub changes: Vec<Change>,
    pub superposition_mutations_dispatched: u32,
    pub leases_acquired: u32,
    pub hosted_apply: &'static str,
}
pub struct Generated {
    pub text: String,
    pub report: Report,
}
fn invalid<T>(v: Option<T>) -> PlanResult<T> {
    v.ok_or_else(|| refuse(PlanCode::InvalidInput))
}
fn parsed(text: &str) -> PlanResult<JsonValue> {
    toml_edit::de::from_str(text).map_err(|_| refuse(PlanCode::InvalidInput))
}
fn key_text(key: &ManagedKey) -> PlanResult<String> {
    planner_types::key_text(key)
}
fn data(value: &ManagedValue) -> PlanResult<JsonValue> {
    match value {
        ManagedValue::Required(v) => Ok(JsonValue::Bool(*v)),
        ManagedValue::Options(v) => {
            serde_json::to_value(v).map_err(|_| refuse(PlanCode::InvalidInput))
        }
    }
}
fn encode(value: &JsonValue) -> PlanResult<Value> {
    Ok(match value {
        JsonValue::Bool(v) => Value::from(*v),
        JsonValue::String(v) => Value::from(v.clone()),
        JsonValue::Number(v) => {
            if let Some(v) = v.as_i64() {
                Value::from(v)
            } else {
                return Err(refuse(PlanCode::InvalidInput));
            }
        }
        JsonValue::Array(v) => {
            let mut array = Array::new();
            for v in v {
                array.push(encode(v)?);
            }
            Value::Array(array)
        }
        JsonValue::Object(v) => {
            let mut table = InlineTable::new();
            for (key, v) in v {
                table.insert(key, encode(v)?);
            }
            Value::InlineTable(table)
        }
        JsonValue::Null => return Err(refuse(PlanCode::InvalidInput)),
    })
}
fn condition(key: &ContextKey) -> PlanResult<JsonValue> {
    serde_json::to_value(key).map_err(|_| refuse(PlanCode::InvalidInput))
}
fn identity(value: &JsonValue) -> PlanResult<Vec<u8>> {
    planner_types::checked(planner_types::json(value)?.canonical())
}
fn record(
    changes: &mut Vec<Change>,
    kind: &'static str,
    subject: &impl Serialize,
    before: &impl Serialize,
    after: &impl Serialize,
) -> PlanResult<()> {
    if changes.len() >= MAX_CHANGES {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    changes.push(Change {
        kind,
        subject: planner_types::digest(subject)?,
        before: planner_types::digest(before)?,
        after: planner_types::digest(after)?,
    });
    Ok(())
}
fn shape(root: &JsonValue) -> PlanResult<BTreeMap<Vec<u8>, usize>> {
    let defaults = invalid(root.get("default-configs").and_then(JsonValue::as_object))?;
    let dimensions = invalid(root.get("dimensions").and_then(JsonValue::as_object))?;
    let rows = invalid(root.get("overrides").and_then(JsonValue::as_array))?;
    if rows.len() > MAX_ROWS || defaults.len() > MAX_ROWS || dimensions.len() > 256 {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    let mut indices = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let map = invalid(row.as_object())?;
        let c = invalid(map.get("_context_"))?;
        if invalid(c.as_object())?.is_empty() || map.len() < 2 {
            return Err(refuse(PlanCode::InvalidInput));
        }
        if indices.insert(identity(c)?, index).is_some() {
            return Err(refuse(PlanCode::AmbiguousIdentity));
        }
    }
    Ok(indices)
}
fn scrub_rows(root: &mut JsonValue, grants: &BTreeMap<Vec<u8>, &Grant>) -> PlanResult<()> {
    let rows = invalid(root.get_mut("overrides").and_then(JsonValue::as_array_mut))?;
    for row in rows.iter_mut() {
        let map = invalid(row.as_object_mut())?;
        let id = identity(invalid(map.get("_context_"))?)?;
        if let Some(grant) = grants.get(&id) {
            for key in &grant.attributes {
                map.remove(&key_text(key)?);
            }
        }
    }
    rows.retain(|row| row.as_object().is_none_or(|row| row.len() > 1));
    Ok(())
}

/// Only a re-rendered VerifiedDesired can enter this pure boundary. Policy JSON cannot replace it.
pub fn generate(
    template: &str,
    desired: &VerifiedDesired,
    policy: &Policy,
    profile_hash: &Digest,
) -> PlanResult<Generated> {
    if template.len() > MAX_SEED_BYTES {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    if policy.schema_version != 1 {
        return Err(refuse(PlanCode::InvalidInput));
    }
    let build = serde_json::to_value(&desired.desired.input_manifest.build)
        .map_err(|_| refuse(PlanCode::InvalidInput))?;
    if policy.profile_hash != *profile_hash
        || build.get("router_feature_keys") != Some(&serde_json::json!(policy.router_feature_keys))
        || build.get("payment_methods_feature_keys")
            != Some(&serde_json::json!(policy.payment_methods_feature_keys))
    {
        return Err(refuse(PlanCode::BindingMismatch));
    }
    let scope = policy
        .scope
        .approved()
        .map_err(|_| refuse(PlanCode::ApprovalRequired))?;
    if scope.grants.len() > MAX_ROWS
        || scope.create_defaults.len() > 256
        || scope.dimensions.len() > 256
        || scope.fields.len() > MAX_ROWS
        || scope
            .grants
            .iter()
            .try_fold(0usize, |count, grant| {
                count.checked_add(grant.attributes.len())
            })
            .is_none_or(|count| count > MAX_CHANGES)
    {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    if desired.desired.contexts.is_empty() {
        return Err(refuse(PlanCode::EmptyDesired));
    }
    let before = parsed(template)?;
    let indices = shape(&before)?;
    let mut doc: DocumentMut = template
        .parse()
        .map_err(|_| refuse(PlanCode::InvalidInput))?;
    let mut grants = BTreeMap::new();
    for grant in &scope.grants {
        if grant.attributes.is_empty() || !grant.retire.is_subset(&grant.attributes) {
            return Err(refuse(PlanCode::OwnershipRequired));
        }
        let id = identity(&condition(&grant.condition)?)?;
        if grants.insert(id, grant).is_some() {
            return Err(refuse(PlanCode::DuplicateGrant));
        }
    }
    for (context, values) in &desired.desired.contexts {
        let id = identity(&condition(context)?)?;
        let grant = grants
            .get(&id)
            .ok_or_else(|| refuse(PlanCode::OwnershipRequired))?;
        if values
            .keys()
            .any(|key| !grant.attributes.contains(key) || grant.retire.contains(key))
        {
            return Err(refuse(PlanCode::OwnershipRequired));
        }
    }
    let mut changes = vec![];
    let mut removed_rows = vec![];
    let rows = invalid(
        doc.get_mut("overrides")
            .and_then(Item::as_array_of_tables_mut),
    )?;
    // Existing row order is retained. New rows use sorted exact condition identities.
    for (id, grant) in &grants {
        let values = desired.desired.contexts.get(&grant.condition);
        let mut new_row = Table::new();
        new_row.insert(
            "_context_",
            Item::Value(encode(&condition(&grant.condition)?)?),
        );
        let row = match indices.get(id) {
            Some(index) => invalid(rows.get_mut(*index))?,
            None => &mut new_row,
        };
        for typed_key in &grant.attributes {
            let key = key_text(typed_key)?;
            let old = indices
                .get(id)
                .and_then(|index| before.get("overrides")?.get(*index)?.get(&key))
                .cloned();
            if let Some(value) = &old {
                let valid = if key.ends_with("._is_required") {
                    value.is_boolean()
                } else {
                    value
                        .as_array()
                        .is_some_and(|values| values.iter().all(JsonValue::is_string))
                };
                if !valid {
                    return Err(refuse(PlanCode::UnsupportedSchema));
                }
            }
            let requested = values
                .and_then(|v| v.get(typed_key))
                .map(data)
                .transpose()?;
            if old == requested {
                continue;
            }
            if let Some(value) = &requested {
                let mut value = encode(value)?;
                if let Some(old) = row.get(&key).and_then(Item::as_value) {
                    *value.decor_mut() = old.decor().clone();
                }
                row.insert(&key, Item::Value(value));
            } else if old.is_some() {
                if !grant.retire.contains(typed_key) {
                    return Err(refuse(PlanCode::RetirementRequired));
                }
                row.remove(&key);
            }
            record(
                &mut changes,
                "context_attribute",
                &(id, &key),
                &old,
                &requested,
            )?;
        }
        if let Some(index) = indices.get(id) {
            if row.len() == 1 {
                removed_rows.push(*index);
            }
        } else if row.len() > 1 {
            rows.push(new_row);
        }
    }
    removed_rows.sort_unstable();
    for index in removed_rows.into_iter().rev() {
        rows.remove(index);
    }

    // Presentation is never generated. Reuse the planner's exact descriptor checks below.
    let mut created = BTreeSet::new();
    let mut declared = BTreeSet::new();
    for declaration in &scope.create_defaults {
        let key = key_text(&declaration.key)?;
        let field = preflight::field_key(&key)?;
        if !declared.insert(key.clone())
            || (key.ends_with("._is_required")
                != matches!(declaration.value, ManagedValue::Required(_)))
            || !desired
                .desired
                .prerequisites
                .field_contracts
                .contains(&field)
            || declaration.description.len() > 1024
        {
            return Err(refuse(PlanCode::InvalidInput));
        }
        let value = data(&declaration.value)?;
        if let Some(old) = before.get("default-configs").and_then(|m| m.get(&key)) {
            if old.get("value") != Some(&value) {
                return Err(refuse(PlanCode::BindingMismatch));
            }
            continue;
        }
        let schema = match declaration.value {
            ManagedValue::Required(_) => serde_json::json!({"type":"boolean"}),
            ManagedValue::Options(_) => {
                serde_json::json!({"type":"array","items":{"type":"string"}})
            }
        };
        let row = serde_json::json!({"value":value,"schema":schema,"description":declaration.description});
        invalid(
            doc.get_mut("default-configs")
                .and_then(Item::as_table_like_mut),
        )?
        .insert(&key, Item::Value(encode(&row)?));
        created.insert(key.clone());
        record(&mut changes, "create_default", &key, &JsonValue::Null, &row)?;
    }
    let mut extensions = BTreeMap::new();
    for (dimension, labels) in &desired.desired.prerequisites.dimension_labels {
        let rule = scope
            .dimensions
            .get(dimension)
            .ok_or_else(|| refuse(PlanCode::MissingDimension))?;
        let name = dimension.as_str();
        let old = before
            .get("dimensions")
            .and_then(|d| d.get(name))
            .ok_or_else(|| refuse(PlanCode::MissingDimension))?;
        preflight::no_functions(&planner_types::json(old)?)?;
        let mandatory = match old.get("mandatory") {
            None => false,
            Some(value) => invalid(value.as_bool())?,
        };
        if old.get("position").and_then(JsonValue::as_i64) != Some(i64::from(rule.position))
            || old.get("type").and_then(JsonValue::as_str) != Some("REGULAR")
            || mandatory != rule.mandatory
        {
            return Err(refuse(PlanCode::UnsupportedSchema));
        }
        let schema = invalid(old.get("schema"))?;
        let values = invalid(schema.get("enum").and_then(JsonValue::as_array))?;
        let mut seen = BTreeSet::new();
        for value in values {
            if !seen.insert(invalid(value.as_str())?) {
                return Err(refuse(PlanCode::UnsupportedSchema));
            }
        }
        let mut added = vec![];
        for label in labels {
            if !rule.vocabulary.contains(label) {
                return Err(refuse(PlanCode::UnsupportedSchema));
            }
            if !seen.contains(label.as_str()) {
                if !rule.allow_enum_extension {
                    return Err(refuse(PlanCode::MissingDimension));
                }
                added.push(label.as_str().to_owned());
            }
        }
        let mut projected = schema.clone();
        invalid(projected.get_mut("enum").and_then(JsonValue::as_array_mut))?
            .extend(added.iter().cloned().map(JsonValue::String));
        for label in labels {
            preflight::schema_accepts(
                &planner_types::json(&projected)?,
                &planner_types::json(label)?,
            )?;
        }
        if !added.is_empty() {
            let array = invalid(
                doc.get_mut("dimensions")
                    .and_then(|d| d.get_mut(name))
                    .and_then(|d| d.get_mut("schema"))
                    .and_then(|d| d.get_mut("enum"))
                    .and_then(Item::as_array_mut),
            )?;
            for value in &added {
                array.push(value.as_str());
            }
            extensions.insert(name.to_owned(), values.len());
            record(
                &mut changes,
                "extend_dimension_enum",
                &name,
                schema,
                &projected,
            )?;
        }
    }
    let text = doc.to_string();
    if text.len() > MAX_SEED_BYTES {
        return Err(refuse(PlanCode::LimitExceeded));
    }
    let after = parsed(&text)?;
    shape(&after)?;
    let defaults = invalid(after.get("default-configs").and_then(JsonValue::as_object))?;
    let mut values: BTreeMap<String, Json> = BTreeMap::new();
    for (key, row) in defaults {
        values.insert(
            key.clone(),
            planner_types::json(invalid(row.get("value"))?)?,
        );
    }
    for (key, value) in desired.desired.contexts.values().flat_map(|v| v.iter()) {
        let key = key_text(key)?;
        let row = defaults
            .get(&key)
            .ok_or_else(|| refuse(PlanCode::MissingDefault))?;
        preflight::no_functions(&planner_types::json(row)?)?;
        let schema = planner_types::json(invalid(row.get("schema"))?)?;
        preflight::schema_accepts(&schema, &planner_types::json(invalid(row.get("value"))?)?)?;
        preflight::schema_accepts(&schema, &preflight::managed(value)?)?;
    }
    for field in &desired.desired.prerequisites.field_contracts {
        let prefix = format!("dynamic_fields.{}._", field.as_str());
        for (_, row) in defaults.iter().filter(|(k, _)| k.starts_with(&prefix)) {
            preflight::no_functions(&planner_types::json(row)?)?;
        }
    }
    preflight::validate_fields(
        &values,
        &desired.desired.prerequisites.field_contracts,
        desired,
        &PrerequisitePolicy {
            schema_version: 1,
            contract_id: policy.contract_id.clone(),
            dimensions: scope.dimensions.clone(),
            workspace_flags: BTreeMap::new(),
            mandatory_dimensions: BTreeSet::new(),
            fields: scope.fields.clone(),
            create_defaults: scope.create_defaults.clone(),
        },
    )?;

    let mut preserved_before = before;
    let mut preserved_after = after;
    scrub_rows(&mut preserved_before, &grants)?;
    scrub_rows(&mut preserved_after, &grants)?;
    let defaults = invalid(
        preserved_after
            .get_mut("default-configs")
            .and_then(JsonValue::as_object_mut),
    )?;
    for key in created {
        defaults.remove(&key);
    }
    for (name, original_len) in extensions {
        invalid(
            preserved_after
                .get_mut("dimensions")
                .and_then(|v| v.get_mut(&name))
                .and_then(|v| v.get_mut("schema"))
                .and_then(|v| v.get_mut("enum"))
                .and_then(JsonValue::as_array_mut),
        )?
        .truncate(original_len);
    }
    if preserved_before != preserved_after {
        return Err(refuse(PlanCode::BindingMismatch));
    }
    let hash =
        |v: &[u8]| fingerprint("seed-bytes-v1", v).map_err(|_| refuse(PlanCode::InvalidInput));
    Ok(Generated {
        report: Report {
            schema_version: 1,
            profile_hash: profile_hash.clone(),
            policy_hash: planner_types::digest(policy)?,
            bundle_hash: desired.bundle_hash().clone(),
            template_hash: hash(template.as_bytes())?,
            generated_hash: hash(text.as_bytes())?,
            preserved_content_hash: planner_types::digest(&preserved_after)?,
            changes,
            superposition_mutations_dispatched: 0,
            leases_acquired: 0,
            hosted_apply: "disabled",
        },
        text,
    })
}
