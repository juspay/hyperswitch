//! Target-independent rendering. An error never contains a partial desired bundle.

use std::collections::{BTreeMap, BTreeSet};

use api_models::enums::{Connector, FieldType, PaymentMethod, PaymentMethodType};
use serde::Serialize;

use super::{vocabulary, *};

const MAX_REPORTED_DIAGNOSTICS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    InvalidContract,
    DuplicateBinding,
    MissingBinding,
    UnapprovedBinding,
    SourceUnavailable,
    BindingExcluded,
    MissingDimensionMapping,
    UnapprovedDimensionMapping,
    UnsupportedDimension,
    InvalidDimensionLabel,
    MissingFieldContract,
    UnapprovedFieldContract,
    UnsupportedFieldType,
    InvalidPath,
    AliasApprovalMissing,
    InnerPathChanged,
    MetadataPathMismatch,
    MalformedOptions,
    UnsupportedOptions,
    CountryCatalogUnapproved,
    UnsupportedCountry,
    EmptyRequiredOptions,
    MissingRequiredOptions,
    DuplicateOutput,
    DuplicateWritePath,
    IncompatibleInheritance,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RenderDiagnostic {
    pub source: Option<SourceIdentity>,
    pub code: DiagnosticCode,
}

#[derive(Debug, thiserror::Error, Serialize)]
#[error("required-fields render refused; inspect the bounded diagnostics")]
pub struct RenderError {
    pub total_diagnostics: usize,
    pub counts: BTreeMap<DiagnosticCode, usize>,
    pub diagnostics: Vec<RenderDiagnostic>,
}

impl RenderError {
    fn from_diagnostics(mut diagnostics: Vec<RenderDiagnostic>) -> Self {
        diagnostics.sort();
        diagnostics.dedup();
        let mut counts = BTreeMap::new();
        for diagnostic in &diagnostics {
            *counts.entry(diagnostic.code).or_default() += 1;
        }
        let total_diagnostics = diagnostics.len();
        // Keep at least one source for each refusal code. A large missing-binding profile
        // must not hide every malformed-source example behind the sample limit.
        let mut seen_codes = BTreeSet::new();
        let mut reported: BTreeSet<_> = diagnostics
            .iter()
            .filter(|item| seen_codes.insert(item.code))
            .cloned()
            .collect();
        for item in diagnostics {
            if reported.len() >= MAX_REPORTED_DIAGNOSTICS {
                break;
            }
            reported.insert(item);
        }
        Self {
            total_diagnostics,
            counts,
            diagnostics: reported.into_iter().collect(),
        }
    }
}

/// Neither an exclusion nor an empty/feature-absent source grants retirement of an existing row.
#[derive(Debug, Clone, Serialize)]
pub struct RenderScope {
    pub source_rules: usize,
    pub contributed_rules: usize,
    pub excluded_sources: Vec<SourceIdentity>,
    pub retirement_authority: RetirementAuthority,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementAuthority {
    NotGranted,
}

/// Retained inputs are the same sanitized, provenance-normalized inputs used for all hashes.
/// Profile names, output filenames, clocks and execution state are deliberately outside this bundle.
#[derive(Debug, Clone, Serialize)]
pub struct RenderedBundle {
    pub schema_version: u16,
    pub desired: DesiredState,
    pub inputs: EffectiveInputs,
    pub policy: MappingPolicy,
    pub catalog: FieldContractCatalog,
    pub exclusions: ExclusionPolicy,
    pub scope: RenderScope,
}

impl RenderedBundle {
    pub fn canonical_bytes(&self) -> InputResult<Vec<u8>> {
        canonical_bytes(self)
    }

    pub fn hash(&self) -> InputResult<Digest> {
        canonical_hash(HashDomain::Bundle, self)
    }

    pub fn desired_hash(&self) -> InputResult<Digest> {
        // Desired identity is target-independent; input/build identity is bound by the bundle.
        canonical_hash(
            HashDomain::DesiredState,
            &(
                DESIRED_SCHEMA_VERSION,
                &self.desired.contexts.iter().collect::<Vec<_>>(),
                &self.desired.prerequisites,
            ),
        )
    }
}

fn diagnostic(source: Option<&SourceIdentity>, code: DiagnosticCode) -> RenderDiagnostic {
    RenderDiagnostic {
        source: source.cloned(),
        code,
    }
}

fn path_valid(path: &str) -> bool {
    !path.is_empty()
        && path.split('.').all(|part| {
            !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        })
}

fn country_type(field: &FieldType) -> bool {
    matches!(
        field,
        FieldType::UserCountry { .. }
            | FieldType::UserAddressCountry { .. }
            | FieldType::UserShippingAddressCountry { .. }
    )
}

fn literal_options(rule: &RetainedRule) -> Result<(), DiagnosticCode> {
    if let Some(options) = field_options(&rule.field_type) {
        if options
            .iter()
            .any(|v| v.trim().is_empty() || v.chars().any(char::is_control))
        {
            return Err(DiagnosticCode::MalformedOptions);
        }
        if options.iter().any(|v| v == "ALL")
            && (!country_type(&rule.field_type) || options != ["ALL"])
        {
            return Err(DiagnosticCode::UnsupportedOptions);
        }
        if country_type(&rule.field_type)
            && options != ["ALL"]
            && options
                .iter()
                .any(|v| v.len() != 2 || !v.bytes().all(|c| c.is_ascii_uppercase()))
        {
            return Err(DiagnosticCode::MalformedOptions);
        }
        if matches!(rule.field_type, FieldType::UserBankType { .. })
            && options.iter().any(|v| v != "checking" && v != "savings")
        {
            return Err(DiagnosticCode::UnsupportedOptions);
        }
    }
    Ok(())
}

fn dimension(
    source: &Name,
    expected: &str,
    mapping: &BTreeMap<Name, Decision<Name>>,
    dimension: &str,
    catalog: &FieldContractCatalog,
) -> Result<Name, DiagnosticCode> {
    let decision = mapping
        .get(source)
        .ok_or(DiagnosticCode::MissingDimensionMapping)?;
    let label = decision
        .approved()
        .map_err(|_| DiagnosticCode::UnapprovedDimensionMapping)?;
    let values = catalog
        .sdk
        .dimensions
        .iter()
        .find(|(key, _)| key.as_str() == dimension)
        .map(|(_, values)| values)
        .ok_or(DiagnosticCode::InvalidDimensionLabel)?;
    if label.as_str() != expected || !values.contains(label) {
        return Err(DiagnosticCode::InvalidDimensionLabel);
    }
    Ok(label.clone())
}

fn context(rule: &RetainedRule, inputs: &RenderInputs<'_>) -> Result<ContextKey, DiagnosticCode> {
    let method: PaymentMethod = rule
        .source
        .payment_method
        .as_str()
        .parse()
        .map_err(|_| DiagnosticCode::UnsupportedDimension)?;
    let kind: PaymentMethodType = rule
        .source
        .payment_method_type
        .as_str()
        .parse()
        .map_err(|_| DiagnosticCode::UnsupportedDimension)?;
    let connector: Connector = rule
        .source
        .connector
        .as_str()
        .parse()
        .map_err(|_| DiagnosticCode::UnsupportedDimension)?;
    let label = vocabulary::type_label(kind).ok_or(DiagnosticCode::UnsupportedDimension)?;
    let mandate_type = match rule.source.group {
        RuleGroup::Common => None,
        RuleGroup::Mandate => Some(MandateGroup::Mandate),
        RuleGroup::NonMandate => Some(MandateGroup::NonMandate),
    };
    if let Some(group) = &mandate_type {
        let expected = match group {
            MandateGroup::Mandate => "mandate",
            MandateGroup::NonMandate => "non_mandate",
        };
        if !inputs.fields.sdk.dimensions.iter().any(|(key, values)| {
            key.as_str() == "mandate_type" && values.iter().any(|value| value.as_str() == expected)
        }) {
            return Err(DiagnosticCode::InvalidDimensionLabel);
        }
    }
    Ok(ContextKey {
        connector: dimension(
            &rule.source.connector,
            &vocabulary::connector_label(connector),
            &inputs.policy.connectors,
            "connector",
            inputs.fields,
        )?,
        payment_method: dimension(
            &rule.source.payment_method,
            vocabulary::method_label(method),
            &inputs.policy.payment_methods,
            "payment_method",
            inputs.fields,
        )?,
        payment_method_type: dimension(
            &rule.source.payment_method_type,
            label,
            &inputs.policy.payment_method_types,
            "payment_method_type",
            inputs.fields,
        )?,
        mandate_type,
    })
}

fn options(
    rule: &RetainedRule,
    field: &FieldContract,
    catalog: &FieldContractCatalog,
) -> Result<Option<Vec<String>>, DiagnosticCode> {
    let source = field_options(&rule.field_type);
    let required_select = matches!(
        field.render_kind,
        RenderKind::Dropdown | RenderKind::Country | RenderKind::BankNames
    );
    if source.is_some() && field.render_kind == RenderKind::Text {
        return Err(DiagnosticCode::UnsupportedOptions);
    }
    if matches!(rule.field_type, FieldType::LanguagePreference { .. })
        != (field.render_kind == RenderKind::LanguagePreference)
    {
        return Err(DiagnosticCode::UnsupportedFieldType);
    }
    let effective = source.or(field.default_options.as_deref());
    if required_select && effective.is_none() {
        return Err(DiagnosticCode::MissingRequiredOptions);
    }
    if required_select && effective.is_some_and(<[String]>::is_empty) {
        return Err(DiagnosticCode::EmptyRequiredOptions);
    }
    if effective.is_some_and(|values| {
        values
            .iter()
            .any(|v| v.trim().is_empty() || v.chars().any(char::is_control))
    }) {
        return Err(DiagnosticCode::MalformedOptions);
    }
    if effective.is_some_and(|values| values.iter().any(|v| v == "ALL"))
        && !(country_type(&rule.field_type) && source == Some(["ALL".to_owned()].as_slice()))
    {
        return Err(DiagnosticCode::UnsupportedOptions);
    }
    // Nonempty State allowlists can silently fall back to all states on zero matches. No
    // country/asset-scoped acceptance contract exists here, so do not invent one.
    if field.render_kind == RenderKind::State && effective.is_some_and(|values| !values.is_empty())
    {
        return Err(DiagnosticCode::UnsupportedOptions);
    }
    if field.render_kind == RenderKind::Country {
        let countries = catalog
            .unrestricted_countries
            .approved()
            .map_err(|_| DiagnosticCode::CountryCatalogUnapproved)?;
        if let Some(values) = source {
            if values == ["ALL"] && country_type(&rule.field_type) {
                return Ok(Some(countries.clone()));
            }
        }
        if effective.is_some_and(|values| values.iter().any(|v| !countries.contains(v))) {
            return Err(DiagnosticCode::UnsupportedCountry);
        }
    } else if country_type(&rule.field_type) {
        return Err(DiagnosticCode::UnsupportedFieldType);
    }
    // Default options only validate inheritance. Missing source options remain absent.
    Ok(source.map(<[String]>::to_vec))
}

struct Contribution<'a> {
    rule: &'a RetainedRule,
    field_id: &'a Name,
    field: &'a FieldContract,
    context: ContextKey,
    options: Option<Vec<String>>,
}

fn contribution<'a>(
    rule: &'a RetainedRule,
    binding: &'a Binding,
    inputs: &'a RenderInputs<'_>,
) -> Result<Contribution<'a>, DiagnosticCode> {
    let field_id = binding
        .field
        .approved()
        .map_err(|_| DiagnosticCode::UnapprovedBinding)?;
    let ordinary = rule
        .required_field
        .as_str()
        .strip_prefix("payment_method_data.")
        .unwrap_or(rule.required_field.as_str());
    if !path_valid(ordinary)
        || !path_valid(field_id.as_str())
        || !path_valid(rule.source.path.as_str())
    {
        return Err(DiagnosticCode::InvalidPath);
    }
    if let Some(expected) = &binding.expected_required_field {
        if expected != &rule.required_field {
            return Err(DiagnosticCode::InnerPathChanged);
        }
    } else if ordinary != field_id.as_str() {
        return Err(DiagnosticCode::AliasApprovalMissing);
    }
    let field = inputs
        .fields
        .fields
        .get(field_id)
        .ok_or(DiagnosticCode::MissingFieldContract)?
        .approved()
        .map_err(|_| DiagnosticCode::UnapprovedFieldContract)?;
    let source_type = rule.field_type.to_string();
    if !field
        .source_types
        .iter()
        .any(|kind| kind.as_str() == source_type)
    {
        return Err(DiagnosticCode::UnsupportedFieldType);
    }
    if !path_valid(field.write_path.as_str()) || !path_valid(field.read_path.as_str()) {
        return Err(DiagnosticCode::InvalidPath);
    }
    if ordinary == field_id.as_str() && field.write_path != rule.required_field {
        return Err(DiagnosticCode::MetadataPathMismatch);
    }
    Ok(Contribution {
        rule,
        field_id,
        field,
        context: context(rule, inputs)?,
        options: options(rule, field, inputs.fields)?,
    })
}

fn same_scope(left: &ContextKey, right: &ContextKey) -> bool {
    left.connector == right.connector
        && left.payment_method == right.payment_method
        && left.payment_method_type == right.payment_method_type
}

/// No source, policy, catalog or exclusion is fetched or inferred here. Every present rule is
/// either explicitly excluded or produces a checked contribution. Any refusal prevents a bundle.
pub fn render(inputs: RenderInputs<'_>) -> error_stack::Result<RenderedBundle, RenderError> {
    let mut diagnostics = Vec::new();
    let mut bindings = BTreeMap::new();
    for binding in &inputs.policy.bindings {
        match bindings.entry(&binding.source) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(binding);
            }
            std::collections::btree_map::Entry::Occupied(_) => diagnostics.push(diagnostic(
                Some(&binding.source),
                DiagnosticCode::DuplicateBinding,
            )),
        }
    }
    let manifest = inputs.manifest().map_err(|_| {
        diagnostics.push(diagnostic(None, DiagnosticCode::InvalidContract));
        RenderError::from_diagnostics(diagnostics.clone())
    })?;
    let sources: BTreeSet<_> = inputs
        .effective
        .rules()
        .iter()
        .map(|rule| &rule.source)
        .collect();
    let excluded: BTreeSet<_> = inputs
        .exclusions
        .sources
        .iter()
        .map(|exclusion| &exclusion.source)
        .collect();
    for source in bindings.keys().chain(excluded.iter()) {
        if !sources.contains(source) {
            diagnostics.push(diagnostic(Some(source), DiagnosticCode::SourceUnavailable));
        }
    }
    let mut contributions = Vec::new();
    let mut excluded_sources = Vec::new();
    for rule in inputs.effective.rules() {
        if excluded.contains(&rule.source) {
            if bindings.contains_key(&rule.source) {
                diagnostics.push(diagnostic(
                    Some(&rule.source),
                    DiagnosticCode::BindingExcluded,
                ));
            }
            excluded_sources.push(rule.source.clone());
            continue;
        }
        // Report malformed literal data even when the profile is missing an approval/catalog.
        if let Err(code) = literal_options(rule) {
            diagnostics.push(diagnostic(Some(&rule.source), code));
        }
        match bindings.get(&rule.source) {
            None => diagnostics.push(diagnostic(
                Some(&rule.source),
                DiagnosticCode::MissingBinding,
            )),
            Some(binding) => match contribution(rule, binding, &inputs) {
                Ok(value) => contributions.push(value),
                Err(code) => diagnostics.push(diagnostic(Some(&rule.source), code)),
            },
        }
    }

    let mut contexts: BTreeMap<ContextKey, BTreeMap<ManagedKey, ManagedValue>> = BTreeMap::new();
    let mut prerequisites = PrerequisiteRequirements {
        dimension_labels: BTreeMap::new(),
        field_contracts: BTreeSet::new(),
    };
    for (index, item) in contributions.iter().enumerate() {
        for prior in contributions
            .iter()
            .take(index)
            .filter(|prior| same_scope(&prior.context, &item.context))
        {
            let overlapping = prior.context.mandate_type.is_none()
                || item.context.mandate_type.is_none()
                || prior.context.mandate_type == item.context.mandate_type;
            if overlapping
                && prior.field_id != item.field_id
                && prior.field.write_path == item.field.write_path
            {
                diagnostics.push(diagnostic(
                    Some(&item.rule.source),
                    DiagnosticCode::DuplicateWritePath,
                ));
            }
            if overlapping
                && prior.field_id == item.field_id
                && prior.context != item.context
                && (prior.rule.field_type.to_string() != item.rule.field_type.to_string()
                    || prior.rule.required_field != item.rule.required_field
                    || prior.field.render_kind != item.field.render_kind
                    || prior.field.read_path != item.field.read_path
                    || (prior.context.mandate_type.is_none()
                        && prior.options.is_some()
                        && item.options.is_none())
                    || (item.context.mandate_type.is_none()
                        && item.options.is_some()
                        && prior.options.is_none()))
            {
                diagnostics.push(diagnostic(
                    Some(&item.rule.source),
                    DiagnosticCode::IncompatibleInheritance,
                ));
            }
        }
        let required_key =
            format!("dynamic_fields.{}._is_required", item.field_id.as_str()).try_into();
        let options_key = format!(
            "dynamic_fields.{}._dropdown_options",
            item.field_id.as_str()
        )
        .try_into();
        let (Ok(required_key), Ok(options_key)) = (required_key, options_key) else {
            diagnostics.push(diagnostic(
                Some(&item.rule.source),
                DiagnosticCode::InvalidPath,
            ));
            continue;
        };
        let values = contexts.entry(item.context.clone()).or_default();
        // Detect collisions before any insertion, even identical duplicate producers.
        if values.contains_key(&required_key) || values.contains_key(&options_key) {
            diagnostics.push(diagnostic(
                Some(&item.rule.source),
                DiagnosticCode::DuplicateOutput,
            ));
            continue;
        }
        values.insert(required_key, ManagedValue::Required(true));
        if let Some(options) = &item.options {
            values.insert(options_key, ManagedValue::Options(options.clone()));
        }
        prerequisites.field_contracts.insert(item.field_id.clone());
        let mut dimensions = vec![
            ("connector", item.context.connector.clone()),
            ("payment_method", item.context.payment_method.clone()),
            (
                "payment_method_type",
                item.context.payment_method_type.clone(),
            ),
        ];
        if let Some(group) = &item.context.mandate_type {
            let label = match group {
                MandateGroup::Mandate => "mandate",
                MandateGroup::NonMandate => "non_mandate",
            };
            let label = label.to_owned().try_into().map_err(|_| {
                RenderError::from_diagnostics(vec![diagnostic(
                    None,
                    DiagnosticCode::InvalidContract,
                )])
            })?;
            dimensions.push(("mandate_type", label));
        }
        for (key, value) in dimensions {
            let key = key.to_owned().try_into().map_err(|_| {
                RenderError::from_diagnostics(vec![diagnostic(
                    None,
                    DiagnosticCode::InvalidContract,
                )])
            })?;
            prerequisites
                .dimension_labels
                .entry(key)
                .or_default()
                .insert(value);
        }
    }
    if !diagnostics.is_empty() {
        return Err(RenderError::from_diagnostics(diagnostics).into());
    }
    Ok(RenderedBundle {
        schema_version: DESIRED_SCHEMA_VERSION,
        desired: DesiredState {
            schema_version: DESIRED_SCHEMA_VERSION,
            contexts,
            prerequisites,
            input_manifest: manifest,
            diagnostics: Vec::new(),
        },
        inputs: inputs.effective.clone(),
        policy: inputs.policy.clone(),
        catalog: inputs.fields.clone(),
        exclusions: inputs.exclusions.clone(),
        scope: RenderScope {
            source_rules: sources.len(),
            contributed_rules: contributions.len(),
            excluded_sources,
            retirement_authority: RetirementAuthority::NotGranted,
        },
    })
}
