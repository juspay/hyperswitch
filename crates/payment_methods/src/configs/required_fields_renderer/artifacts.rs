//! Versioned, transport-free contracts. All arrays are ordered unless a type explicitly uses a set.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

pub const SCHEMA_VERSION: u16 = 1;
/// v2 uses the pinned SDK's `_dropdown_options` key and explicit renderer compatibility inputs.
pub const DESIRED_SCHEMA_VERSION: u16 = 2;
pub const CANONICAL_VERSION: u16 = 1;
pub const MAX_CONTRACT_BYTES: usize = 128 * 1024;
pub type InputResult<T> = error_stack::Result<T, InputError>;

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("invalid required-fields input")]
    InputInvalid,
    #[error("required-fields mapping requires an explicit reviewed decision")]
    MappingUnresolved,
    #[error("required-fields sync supports v1 only")]
    UnsupportedBuild,
}

/// A bounded label/path/reference, never a credential or arbitrary JSON payload.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Name(String);

impl TryFrom<String> for Name {
    type Error = InputError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty()
            || value.len() > 160
            || value.trim() != value
            || !value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b" _-./:".contains(&c))
        {
            return Err(InputError::InputInvalid);
        }
        Ok(Self(value))
    }
}

impl Name {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Digest(String);

impl TryFrom<String> for Digest {
    type Error = InputError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.strip_prefix("sha256:").is_some_and(|s| is_hex(s, 64)) {
            Ok(Self(value))
        } else {
            Err(InputError::InputInvalid)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Revision(String);

impl TryFrom<String> for Revision {
    type Error = InputError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if is_hex(&value, 40) {
            Ok(Self(value))
        } else {
            Err(InputError::InputInvalid)
        }
    }
}

fn is_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HashDomain {
    EffectiveSettings,
    RetainedRules,
    MappingPolicy,
    FieldCatalog,
    Exclusions,
    InputManifest,
    DesiredState,
    Bundle,
    Plan,
}

/// UTF-8 JSON, lexically sorted object keys, intact arrays, integer numbers only.
/// Explicitly sort even if serde_json's preserve_order feature is unified by another crate.
pub fn canonical_bytes<T: Serialize>(value: &T) -> InputResult<Vec<u8>> {
    fn write(value: &serde_json::Value, out: &mut Vec<u8>) -> InputResult<()> {
        match value {
            serde_json::Value::Object(map) => {
                out.push(b'{');
                for (index, (key, value)) in
                    map.iter().collect::<BTreeMap<_, _>>().iter().enumerate()
                {
                    if index > 0 {
                        out.push(b',');
                    }
                    serde_json::to_writer(&mut *out, key).map_err(|_| InputError::InputInvalid)?;
                    out.push(b':');
                    write(value, out)?;
                }
                out.push(b'}');
            }
            serde_json::Value::Array(values) => {
                out.push(b'[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(b',');
                    }
                    write(value, out)?;
                }
                out.push(b']');
            }
            serde_json::Value::Number(number) if !number.is_i64() && !number.is_u64() => {
                return Err(InputError::InputInvalid.into());
            }
            _ => serde_json::to_writer(out, value).map_err(|_| InputError::InputInvalid)?,
        }
        Ok(())
    }
    let value = serde_json::to_value(value).map_err(|_| InputError::InputInvalid)?;
    let mut out = Vec::new();
    write(&value, &mut out)?;
    Ok(out)
}

pub fn canonical_hash<T: Serialize>(domain: HashDomain, value: &T) -> InputResult<Digest> {
    #[derive(Serialize)]
    struct Envelope<'a, T> {
        canonical_version: u16,
        domain: HashDomain,
        value: &'a T,
    }
    let bytes = canonical_bytes(&Envelope {
        canonical_version: CANONICAL_VERSION,
        domain,
        value,
    })?;
    Ok(Digest(format!("sha256:{:x}", Sha256::digest(bytes))))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Decision<T> {
    Approved { value: T, review: Name },
    Refused { reason: Name },
}

impl<T> Decision<T> {
    pub fn approved(&self) -> InputResult<&T> {
        match self {
            Self::Approved { value, .. } => Ok(value),
            Self::Refused { .. } => Err(InputError::MappingUnresolved.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleGroup {
    Common,
    Mandate,
    NonMandate,
}

/// Outer source-map identity is retained independently of the inner required_field path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub payment_method: Name,
    pub payment_method_type: Name,
    pub connector: Name,
    pub group: RuleGroup,
    pub path: Name,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub source: SourceIdentity,
    pub field: Decision<Name>,
    /// Required for an alias. Binds approval to the inner path as well as the outer source key.
    #[serde(default)]
    pub expected_required_field: Option<Name>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingPolicy {
    pub schema_version: u16,
    pub contract_id: Name,
    pub connectors: BTreeMap<Name, Decision<Name>>,
    pub payment_methods: BTreeMap<Name, Decision<Name>>,
    pub payment_method_types: BTreeMap<Name, Decision<Name>>,
    pub bindings: Vec<Binding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SdkContract {
    pub contract_id: Name,
    pub web_revision: Revision,
    pub shared_revision: Revision,
    pub evaluator_hash: Digest,
    pub dimensions: BTreeMap<Name, Vec<Name>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderKind {
    Text,
    Dropdown,
    Country,
    State,
    BankNames,
    LanguagePreference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldContract {
    pub render_kind: RenderKind,
    pub write_path: Name,
    pub read_path: Name,
    pub review: Name,
    /// Empty legacy catalogs can still be parsed, but cannot authorize rendering a field.
    #[serde(default)]
    pub source_types: BTreeSet<Name>,
    /// Reviewed presentation default, used only to validate an inherited required select.
    /// It is never emitted or substituted for an explicit source option vector.
    #[serde(default)]
    pub default_options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldContractCatalog {
    pub schema_version: u16,
    pub contract_id: Name,
    pub sdk: SdkContract,
    pub fields: BTreeMap<Name, Decision<FieldContract>>,
    pub unrestricted_countries: Decision<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exclusion {
    pub source: SourceIdentity,
    pub reason: Name,
    pub review: Name,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExclusionPolicy {
    pub schema_version: u16,
    pub contract_id: Name,
    pub sources: Vec<Exclusion>,
}

pub trait Contract: serde::de::DeserializeOwned {
    fn validate(&self) -> InputResult<()>;
}

/// No file/env access, no default business choices, bounded allocation, sanitized parse errors.
pub fn parse_contract<T: Contract>(text: &str) -> InputResult<T> {
    if text.len() > MAX_CONTRACT_BYTES {
        return Err(InputError::InputInvalid.into());
    }
    let parsed: T = toml::from_str(text).map_err(|_| InputError::InputInvalid)?;
    parsed.validate()?;
    Ok(parsed)
}

fn version(version: u16) -> InputResult<()> {
    if version != SCHEMA_VERSION {
        return Err(InputError::InputInvalid.into());
    }
    Ok(())
}

impl Contract for MappingPolicy {
    fn validate(&self) -> InputResult<()> {
        version(self.schema_version)?;
        let mut sources = BTreeSet::new();
        for binding in &self.bindings {
            if !sources.insert(&binding.source) {
                return Err(InputError::InputInvalid.into());
            }
        }
        Ok(())
    }
}

impl Contract for FieldContractCatalog {
    fn validate(&self) -> InputResult<()> {
        version(self.schema_version)?;
        if let Decision::Approved {
            value: countries, ..
        } = &self.unrestricted_countries
        {
            if countries.is_empty()
                || countries
                    .iter()
                    .any(|code| code.len() != 2 || !code.bytes().all(|c| c.is_ascii_uppercase()))
                || countries.iter().collect::<BTreeSet<_>>().len() != countries.len()
            {
                return Err(InputError::InputInvalid.into());
            }
        }
        Ok(())
    }
}

impl Contract for ExclusionPolicy {
    fn validate(&self) -> InputResult<()> {
        version(self.schema_version)?;
        if self
            .sources
            .iter()
            .map(|e| &e.source)
            .collect::<BTreeSet<_>>()
            .len()
            != self.sources.len()
        {
            return Err(InputError::InputInvalid.into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactIdentity {
    Offline,
    Image {
        index_digest: Digest,
        platform_digests: BTreeMap<Name, Digest>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseIdentity {
    pub application_revision: Revision,
    /// Binds local edits too; a Git SHA alone is insufficient for a dirty checkout.
    pub source_tree_hash: Digest,
    pub artifact: ArtifactIdentity,
    pub renderer_implementation: Name,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildIdentity {
    release: ReleaseIdentity,
    renderer_schema_version: u16,
    /// Cargo's normalized CARGO_FEATURE_* suffixes, including defaults and selected backend.
    router_feature_keys: BTreeSet<Name>,
    payment_methods_feature_keys: BTreeSet<Name>,
}

impl BuildIdentity {
    fn checked_features(value: &str) -> InputResult<BTreeSet<Name>> {
        let features = value
            .split(',')
            .map(|v| Name::try_from(v.to_owned()).map_err(Into::into))
            .collect::<InputResult<BTreeSet<_>>>()?;
        let has = |key| features.iter().any(|v| v.as_str() == key);
        if !has("V1") || has("V2") || !has("REQUIRED_FIELDS_SYNC") {
            return Err(InputError::UnsupportedBuild.into());
        }
        Ok(features)
    }

    /// A payment_methods-only example/test has no compiled router to attest. Keep that feature
    /// set empty, require an explicitly offline artifact, and pass this package's compiled keys.
    pub fn for_offline_renderer(
        release: ReleaseIdentity,
        payment_methods: &str,
    ) -> InputResult<Self> {
        if !matches!(release.artifact, ArtifactIdentity::Offline) {
            return Err(InputError::UnsupportedBuild.into());
        }
        Ok(Self {
            release,
            renderer_schema_version: DESIRED_SCHEMA_VERSION,
            router_feature_keys: BTreeSet::new(),
            payment_methods_feature_keys: Self::checked_features(payment_methods)?,
        })
    }

    pub fn from_feature_keys(
        release: ReleaseIdentity,
        router: &str,
        payment_methods: &str,
    ) -> InputResult<Self> {
        let router_feature_keys = Self::checked_features(router)?;
        let payment_methods_feature_keys = Self::checked_features(payment_methods)?;
        Ok(Self {
            release,
            renderer_schema_version: DESIRED_SCHEMA_VERSION,
            router_feature_keys,
            payment_methods_feature_keys,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InputManifest {
    pub schema_version: u16,
    pub canonical_version: u16,
    pub build: BuildIdentity,
    pub settings_provenance: SettingsProvenance,
    pub effective_settings_hash: Digest,
    pub retained_rules_hash: Digest,
    pub mapping_hash: Digest,
    pub catalog_hash: Digest,
    pub exclusions_hash: Digest,
}

/// Effective values alone are not file/ConfigMap/pod attestation. The later CLI/release boundary
/// must supply verified provenance before a bundle can be used for a hosted plan or apply.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsProvenance {
    EffectiveValuesOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandateGroup {
    Mandate,
    NonMandate,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextKey {
    pub connector: Name,
    pub payment_method: Name,
    pub payment_method_type: Name,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mandate_type: Option<MandateGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ManagedValue {
    Required(bool),
    Options(Vec<String>),
}

/// Only the two owned dynamic-field attributes can appear in desired contributions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ManagedKey(String);

impl TryFrom<String> for ManagedKey {
    type Error = InputError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let field = value.strip_prefix("dynamic_fields.").and_then(|v| {
            v.strip_suffix("._is_required")
                .or_else(|| v.strip_suffix("._dropdown_options"))
        });
        if value.len() > 256
            || !field.is_some_and(|field| {
                field.split('.').all(|part| {
                    !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                })
            })
        {
            return Err(InputError::InputInvalid);
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PrerequisiteRequirements {
    pub dimension_labels: BTreeMap<Name, BTreeSet<Name>>,
    pub field_contracts: BTreeSet<Name>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MappingDiagnostic {
    pub source: SourceIdentity,
    pub code: Name,
}

#[derive(Debug, Clone, Serialize)]
pub struct DesiredState {
    pub schema_version: u16,
    #[serde(serialize_with = "context_entries")]
    pub contexts: BTreeMap<ContextKey, BTreeMap<ManagedKey, ManagedValue>>,
    pub prerequisites: PrerequisiteRequirements,
    pub input_manifest: InputManifest,
    pub diagnostics: Vec<MappingDiagnostic>,
}

// JSON object keys cannot be compound ContextKeys. Ordered entry tuples preserve the typed identity.
fn context_entries<S: serde::Serializer>(
    contexts: &BTreeMap<ContextKey, BTreeMap<ManagedKey, ManagedValue>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    contexts.iter().collect::<Vec<_>>().serialize(serializer)
}

#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod tests;
