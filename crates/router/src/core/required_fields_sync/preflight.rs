//! Live prerequisites and a deliberately closed schema/descriptor vocabulary.
//! Unsupported functions/schema keywords refuse; nothing executes a validator or computation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    planner_types::*,
    raw_json::Json,
    types::{CompleteInventory, DimensionKind},
    ManagedKey, ManagedValue, Name, RenderKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionRule {
    pub position: i32,
    pub mandatory: bool,
    #[serde(deserialize_with = "super::ownership::unique_set")]
    pub vocabulary: BTreeSet<Name>,
    pub allow_enum_extension: bool,
    pub review: Name,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRule {
    /// Exact reviewed live presentation attributes, without the two managed attributes.
    pub metadata: BTreeMap<String, serde_json::Value>,
    /// Certified asset/wire values. Vector order is retained; this is not an option rewrite.
    pub allowed_options: Option<Vec<String>>,
    pub review: Name,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultCreation {
    pub key: ManagedKey,
    pub value: ManagedValue,
    pub description: String,
    pub review: Name,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrerequisitePolicy {
    pub schema_version: u16,
    pub contract_id: Name,
    pub dimensions: BTreeMap<Name, DimensionRule>,
    pub workspace_flags: BTreeMap<String, bool>,
    #[serde(deserialize_with = "super::ownership::unique_set")]
    pub mandatory_dimensions: BTreeSet<Name>,
    pub fields: BTreeMap<Name, FieldRule>,
    pub create_defaults: Vec<DefaultCreation>,
}

pub(super) fn no_functions(row: &Json) -> PlanResult<()> {
    for key in [
        "value_validation_function_name",
        "value_compute_function_name",
    ] {
        if checked(row.optional(key))?.is_some_and(|v| !v.null()) {
            return Err(refuse(PlanCode::UnsupportedFunction));
        }
    }
    Ok(())
}
pub(super) fn managed(value: &ManagedValue) -> PlanResult<Json> {
    match value {
        ManagedValue::Required(v) => json(v),
        ManagedValue::Options(v) => json(v),
    }
}
pub(super) fn field_key(key: &str) -> PlanResult<Name> {
    key.strip_prefix("dynamic_fields.")
        .and_then(|s| s.rsplit_once("._").map(|(field, _)| field))
        .ok_or_else(|| refuse(PlanCode::InvalidInput))?
        .to_owned()
        .try_into()
        .map_err(|_| refuse(PlanCode::InvalidInput))
}

/// Reject unimplemented constraints rather than treating arbitrary JSON Schema as compatible.
pub(super) fn schema_accepts(schema: &Json, value: &Json) -> PlanResult<()> {
    let entries = checked(schema.object())?;
    let kind = checked(checked(schema.get("type"))?.string())?;
    let allowed: &[&str] = match kind.as_str() {
        "boolean" | "string" => &["type", "enum", "description", "title"],
        "array" => &[
            "type",
            "items",
            "minItems",
            "maxItems",
            "uniqueItems",
            "description",
            "title",
        ],
        _ => return Err(refuse(PlanCode::UnsupportedSchema)),
    };
    if entries.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(refuse(PlanCode::UnsupportedSchema));
    }
    for key in ["description", "title"] {
        if let Some(v) = entries.get(key) {
            checked(v.string())?;
        }
    }
    match kind.as_str() {
        "boolean" => {
            checked(value.boolean()).map_err(|_| refuse(PlanCode::UnsupportedSchema))?;
        }
        "string" => {
            checked(value.string()).map_err(|_| refuse(PlanCode::UnsupportedSchema))?;
        }
        "array" => {
            let values = checked(value.array()).map_err(|_| refuse(PlanCode::UnsupportedSchema))?;
            let items = checked(schema.get("items"))?;
            if checked(checked(items.get("type"))?.string())? != "string" {
                return Err(refuse(PlanCode::UnsupportedSchema));
            }
            // An empty value must not hide unsupported constraints in its item schema.
            let representative = match checked(items.optional("enum"))? {
                Some(values) => checked(values.array())?
                    .first()
                    .cloned()
                    .ok_or_else(|| refuse(PlanCode::UnsupportedSchema))?,
                None => json(&"")?,
            };
            schema_accepts(items, &representative)?;
            for value in values {
                schema_accepts(items, value)?;
            }
            if let Some(min) = entries.get("minItems") {
                if values.len() < checked(min.unsigned())? as usize {
                    return Err(refuse(PlanCode::UnsupportedSchema));
                }
            }
            if let Some(max) = entries.get("maxItems") {
                if values.len() > checked(max.unsigned())? as usize {
                    return Err(refuse(PlanCode::UnsupportedSchema));
                }
            }
            if let Some(unique) = entries.get("uniqueItems") {
                if checked(unique.boolean())?
                    && values
                        .iter()
                        .map(|v| checked(v.canonical()))
                        .collect::<PlanResult<BTreeSet<_>>>()?
                        .len()
                        != values.len()
                {
                    return Err(refuse(PlanCode::UnsupportedSchema));
                }
            }
        }
        _ => return Err(refuse(PlanCode::UnsupportedSchema)),
    }
    if let Some(values) = entries.get("enum") {
        let mut seen = BTreeSet::new();
        let mut found = false;
        for item in checked(values.array())? {
            if !seen.insert(checked(item.canonical())?) {
                return Err(refuse(PlanCode::UnsupportedSchema));
            }
            found |= checked(item.canonical())? == checked(value.canonical())?;
        }
        if !found {
            return Err(refuse(PlanCode::UnsupportedSchema));
        }
    }
    Ok(())
}

pub(super) struct LiveState {
    pub defaults: BTreeMap<String, Json>,
    pub projected_defaults: BTreeMap<String, Json>,
    pub actions: Vec<PlannedAction>,
}

pub(super) fn inspect(
    inventory: &CompleteInventory,
    desired: &VerifiedDesired,
    policy: &PrerequisitePolicy,
    managed_values: &BTreeMap<String, Vec<Json>>,
) -> PlanResult<LiveState> {
    if policy.schema_version != 1 {
        return Err(refuse(PlanCode::InvalidInput));
    }
    let flags = [
        "allow_experiment_self_approval",
        "auto_populate_control",
        "enable_context_validation",
        "enable_change_reason_validation",
    ];
    if policy.workspace_flags.len() != flags.len() {
        return Err(refuse(PlanCode::BindingMismatch));
    }
    for key in flags {
        if policy.workspace_flags.get(key).copied()
            != Some(checked(
                checked(inventory.workspace().raw.get(key))?.boolean(),
            )?)
        {
            return Err(refuse(PlanCode::BindingMismatch));
        }
    }
    let mandatory: BTreeSet<Name> =
        match checked(inventory.workspace().raw.optional("mandatory_dimensions"))?
            .filter(|v| !v.null())
        {
            Some(v) => {
                let values: Vec<Name> =
                    serde_json::from_str(v.raw()).map_err(|_| refuse(PlanCode::InvalidInput))?;
                let set: BTreeSet<_> = values.iter().cloned().collect();
                if set.len() != values.len() {
                    return Err(refuse(PlanCode::InvalidInput));
                }
                set
            }
            None => BTreeSet::new(),
        };
    if mandatory != policy.mandatory_dimensions {
        return Err(refuse(PlanCode::BindingMismatch));
    }
    let mut actions = Vec::new();
    let mut positions = BTreeSet::new();
    for dimension in [
        "connector",
        "payment_method",
        "payment_method_type",
        "mandate_type",
    ] {
        let name: Name = dimension
            .to_owned()
            .try_into()
            .map_err(|_| refuse(PlanCode::InvalidInput))?;
        let rule = policy
            .dimensions
            .get(&name)
            .ok_or_else(|| refuse(PlanCode::MissingDimension))?;
        let live = inventory
            .dimensions()
            .get(&name)
            .ok_or_else(|| refuse(PlanCode::MissingDimension))?;
        let approved: BTreeSet<_> = desired
            .catalog
            .sdk
            .dimensions
            .get(&name)
            .ok_or_else(|| refuse(PlanCode::BindingMismatch))?
            .iter()
            .cloned()
            .collect();
        if approved != rule.vocabulary
            || live.projection().position != rule.position
            || !(0..=30).contains(&rule.position)
            || !positions.insert(rule.position)
            || live.projection().mandatory != rule.mandatory
            || !matches!(live.projection().kind, DimensionKind::Regular)
            || !checked(checked(live.raw.get("dependency_graph"))?.object())?.is_empty()
        {
            return Err(refuse(PlanCode::UnsupportedSchema));
        }
        no_functions(&live.raw)?;
        let schema = checked(live.raw.get("schema"))?;
        let current = checked(checked(schema.get("enum"))?.array())?;
        if current.is_empty() {
            return Err(refuse(PlanCode::UnsupportedSchema));
        }
        let mut values = Vec::new();
        let mut seen = BTreeSet::new();
        for v in current {
            let value = checked(v.string())?;
            if !seen.insert(value.clone()) {
                return Err(refuse(PlanCode::UnsupportedSchema));
            }
            schema_accepts(schema, v)?;
            values.push(value);
        }
        let needed = desired
            .desired
            .prerequisites
            .dimension_labels
            .get(&name)
            .cloned()
            .unwrap_or_default();
        if !needed.is_subset(&rule.vocabulary) {
            return Err(refuse(PlanCode::BindingMismatch));
        }
        let missing: Vec<_> = needed
            .iter()
            .filter(|v| !seen.contains(v.as_str()))
            .map(|v| v.as_str().to_owned())
            .collect();
        if !missing.is_empty() {
            if !rule.allow_enum_extension {
                return Err(refuse(PlanCode::MissingDimension));
            }
            values.extend(missing);
            let projected_schema = checked(schema.replacing("enum", json(&values)?))?;
            let after = checked(live.raw.replacing("schema", projected_schema))?;
            actions.push(PlannedAction {
                kind: ActionKind::ExtendDimension,
                server_id: Some(name),
                condition: None,
                before: Some(live.raw.clone()),
                after: Some(after),
                preservation_hash: hash(&checked(live.raw.replacing("schema", json(&())?))?)?,
                ownership_intent: None,
            });
        }
    }
    let mut defaults = BTreeMap::new();
    for (key, row) in inventory.defaults() {
        defaults.insert(
            key.as_str().to_owned(),
            checked(row.raw.get("value"))?.clone(),
        );
    }
    let mut projected = defaults.clone();
    let mut declarations = BTreeMap::new();
    for declaration in &policy.create_defaults {
        let key = key_text(&declaration.key)?;
        if declarations.insert(key, declaration).is_some() {
            return Err(refuse(PlanCode::InvalidInput));
        }
    }
    for (key, values) in managed_values {
        let name: Name = key
            .clone()
            .try_into()
            .map_err(|_| refuse(PlanCode::InvalidInput))?;
        if let Some(row) = inventory.defaults().get(&name) {
            no_functions(&row.raw)?;
            let schema = checked(row.raw.get("schema"))?;
            schema_accepts(schema, checked(row.raw.get("value"))?)?;
            for value in values {
                schema_accepts(schema, value)?;
            }
        } else {
            let declaration = declarations
                .get(key)
                .ok_or_else(|| refuse(PlanCode::MissingDefault))?;
            let value = managed(&declaration.value)?;
            let schema = match declaration.value {
                ManagedValue::Required(_) => json(&serde_json::json!({"type":"boolean"}))?,
                ManagedValue::Options(_) => {
                    json(&serde_json::json!({"type":"array","items":{"type":"string"}}))?
                }
            };
            for value in values {
                schema_accepts(&schema, value)?;
            }
            let mut row = BTreeMap::new();
            row.insert("key".to_owned(), json(key)?);
            row.insert("value".to_owned(), value.clone());
            row.insert("schema".to_owned(), schema);
            row.insert("description".to_owned(), json(&declaration.description)?);
            actions.push(PlannedAction {
                kind: ActionKind::CreateDefault,
                server_id: None,
                condition: None,
                before: None,
                after: Some(checked(Json::members(&row))?),
                preservation_hash: digest(&())?,
                ownership_intent: None,
            });
            projected.insert(key.clone(), value);
        }
    }
    // Every live default used by the case model must be inert, including presentation metadata.
    let fields: BTreeSet<_> = managed_values
        .keys()
        .map(|key| field_key(key))
        .collect::<PlanResult<_>>()?;
    for field in &fields {
        let prefix = format!("dynamic_fields.{}._", field.as_str());
        for (key, row) in inventory
            .defaults()
            .iter()
            .filter(|(key, _)| key.as_str().starts_with(&prefix))
        {
            let _ = key;
            no_functions(&row.raw)?;
        }
    }
    Ok(LiveState {
        defaults,
        projected_defaults: projected,
        actions,
    })
}

pub(super) fn validate_fields(
    values: &BTreeMap<String, Json>,
    fields: &BTreeSet<Name>,
    desired: &VerifiedDesired,
    policy: &PrerequisitePolicy,
) -> PlanResult<()> {
    for field in fields {
        let contract = desired
            .catalog
            .fields
            .get(field)
            .ok_or_else(|| refuse(PlanCode::MissingPresentation))?
            .approved()
            .map_err(|_| refuse(PlanCode::MissingPresentation))?;
        let rule = policy
            .fields
            .get(field)
            .ok_or_else(|| refuse(PlanCode::MissingPresentation))?;
        let prefix = format!("dynamic_fields.{}._", field.as_str());
        for (attr, value) in &rule.metadata {
            let valid = match attr.as_str() {
                "field_render_type"
                | "confirm_request_write_path"
                | "intent_data_read_path"
                | "default_label_text"
                | "default_placeholder_text"
                | "validation_rule_type"
                | "validation_regex_pattern"
                | "label_localization_key"
                | "placeholder_localization_key"
                | "input_format_pattern"
                | "html_autocomplete_attribute"
                | "keyboard_type"
                | "layout_row_id"
                | "merchant_provided_display_name"
                | "merchant_provided_placeholder_text" => value.is_string(),
                "field_display_order" | "max_input_length" => {
                    value.as_i64().is_some_and(|n| i32::try_from(n).is_ok())
                }
                "render_when_prefilled" => value.is_boolean(),
                "layout_width_ratio" => value.as_f64().is_some_and(f64::is_finite),
                // Known unused browser metadata remains exact and is never substituted for options.
                "extended_dropdown_options" => true,
                _ => false,
            };
            if !valid {
                return Err(refuse(PlanCode::IncompatiblePresentation));
            }
        }
        for attr in [
            "field_render_type",
            "confirm_request_write_path",
            "intent_data_read_path",
            "default_label_text",
            "field_display_order",
        ] {
            if !rule.metadata.contains_key(attr) || !values.contains_key(&format!("{prefix}{attr}"))
            {
                return Err(refuse(PlanCode::MissingPresentation));
            }
        }
        let render = rule
            .metadata
            .get("field_render_type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| refuse(PlanCode::IncompatiblePresentation))?;
        let matches = match contract.render_kind {
            RenderKind::Dropdown => matches!(render, "Dropdown" | "CryptoCurrency"),
            RenderKind::Country => render == "Country",
            RenderKind::State => render == "State",
            RenderKind::BankNames => render == "BankNamesSelect",
            RenderKind::LanguagePreference => render == "LanguagePreference",
            RenderKind::Text => matches!(
                render,
                "Text"
                    | "Generic"
                    | "Phone"
                    | "Date"
                    | "CardNumber"
                    | "Cvc"
                    | "Email"
                    | "CardHolderName"
                    | "CardExpiryMonth"
                    | "CardExpiryYear"
                    | "CardNetwork"
                    | "PhoneCountryCode"
                    | "FirstName"
                    | "LastName"
                    | "CryptoNetwork"
                    | "DateOfBirth"
            ),
        };
        if !matches
            || rule
                .metadata
                .get("confirm_request_write_path")
                .and_then(serde_json::Value::as_str)
                != Some(contract.write_path.as_str())
            || rule
                .metadata
                .get("intent_data_read_path")
                .and_then(serde_json::Value::as_str)
                != Some(contract.read_path.as_str())
        {
            return Err(refuse(PlanCode::IncompatiblePresentation));
        }
        for (key, value) in values.iter().filter(|(key, _)| key.starts_with(&prefix)) {
            let attr = key
                .strip_prefix(&prefix)
                .ok_or_else(|| refuse(PlanCode::InvalidInput))?;
            if attr == "is_required" || attr == "dropdown_options" {
                continue;
            }
            let expected = rule
                .metadata
                .get(attr)
                .ok_or_else(|| refuse(PlanCode::IncompatiblePresentation))?;
            if checked(value.canonical())? != checked(json(expected)?.canonical())? {
                return Err(refuse(PlanCode::IncompatiblePresentation));
            }
        }
        for attr in rule.metadata.keys() {
            if !values.contains_key(&format!("{prefix}{attr}")) {
                return Err(refuse(PlanCode::MissingPresentation));
            }
        }
        let required_key = format!("{prefix}is_required");
        let required = match values.get(&required_key) {
            Some(v) => checked(v.boolean())?,
            None if policy.create_defaults.iter().any(|d| {
                key_text(&d.key).ok().as_deref() == Some(required_key.as_str())
                    && matches!(d.value, ManagedValue::Required(_))
            }) =>
            {
                false
            }
            None => return Err(refuse(PlanCode::MissingDefault)),
        };
        let options = values
            .get(&format!("{prefix}dropdown_options"))
            .map(|v| checked(v.array()))
            .transpose()?;
        if let Some(options) = options {
            for value in options {
                let option = checked(value.string())?;
                if option.is_empty()
                    || option.chars().any(char::is_control)
                    || rule
                        .allowed_options
                        .as_ref()
                        .is_some_and(|allowed| !allowed.contains(&option))
                {
                    return Err(refuse(PlanCode::InvalidOptions));
                }
            }
        }
        if required
            && matches!(
                contract.render_kind,
                RenderKind::Dropdown | RenderKind::Country | RenderKind::BankNames
            )
            && options.is_none_or(|v| v.is_empty())
        {
            return Err(refuse(PlanCode::InvalidOptions));
        }
        if matches!(
            contract.render_kind,
            RenderKind::Country | RenderKind::State | RenderKind::BankNames | RenderKind::Dropdown
        ) && rule.allowed_options.is_none()
        {
            return Err(refuse(PlanCode::InvalidOptions));
        }
    }
    Ok(())
}
