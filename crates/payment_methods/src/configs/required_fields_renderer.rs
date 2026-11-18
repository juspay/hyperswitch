//! Pure input capture, deterministic rendering and artifact contracts.
//! No files, environment, providers, SDK clients or application state are accessed here.

use api_models::enums::FieldType;
use serde::Serialize;

use super::settings::{BankRedirectConfig, RequiredFields};

pub mod artifacts;
pub use artifacts::*;
mod render;
pub use render::{
    render, DiagnosticCode, RenderDiagnostic, RenderError, RenderScope, RenderedBundle,
    RetirementAuthority,
};
mod vocabulary;

const EPS_PATH: &str = "payment_method_data.bank_redirect.eps.bank_name";

/// Constructor input contract v1: RequiredFields::new currently reads only EPS/stripe banks.
/// Extend this projection and constructor-dependency tests together if a setting is added.
/// None (absent) and Some([]) (explicit empty) remain distinguishable.
#[derive(Debug, Clone, Serialize)]
pub struct EffectiveBankInputs {
    stripe_eps_banks: Option<Vec<String>>,
}

impl EffectiveBankInputs {
    pub fn capture(banks: &BankRedirectConfig) -> Self {
        let stripe_eps_banks = banks
            .0
            .get(&api_models::enums::PaymentMethodType::Eps)
            .and_then(|connectors| connectors.0.get("stripe"))
            .map(|banks| {
                let mut values: Vec<_> = banks.banks.iter().map(ToString::to_string).collect();
                values.sort(); // BanksVector::banks is a HashSet, not an ordered source.
                values
            });
        Self { stripe_eps_banks }
    }
}

/// Intentionally has no prefill/value member. Display names are retained as source evidence only.
#[derive(Debug, Clone, Serialize)]
pub struct RetainedRule {
    pub source: SourceIdentity,
    pub required_field: Name,
    pub display_name: String,
    pub field_type: FieldType,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectiveInputs {
    schema_version: u16,
    banks: EffectiveBankInputs,
    rules: Vec<RetainedRule>,
}

impl EffectiveInputs {
    /// Capture the existing Settings.required_fields, never RequiredFields::default().
    /// Verify the single provenance-based normalization against the effective bank source.
    pub fn capture(
        required_fields: &RequiredFields,
        bank_config: &BankRedirectConfig,
    ) -> InputResult<Self> {
        let banks = EffectiveBankInputs::capture(bank_config);
        let mut rules = Vec::new();
        for (method, types) in &required_fields.0 {
            for (method_type, connectors) in &types.0 {
                for (connector, fields) in &connectors.fields {
                    for (group, fields) in [
                        (RuleGroup::Common, &fields.common),
                        (RuleGroup::Mandate, &fields.mandate),
                        (RuleGroup::NonMandate, &fields.non_mandate),
                    ] {
                        for (path, field) in fields {
                            let source = SourceIdentity {
                                payment_method: method.to_string().try_into()?,
                                payment_method_type: method_type.to_string().try_into()?,
                                connector: connector.to_string().try_into()?,
                                group,
                                path: path.clone().try_into()?,
                            };
                            let mut field_type = field.field_type.clone();
                            if source.payment_method.as_str() == "bank_redirect"
                                && source.payment_method_type.as_str() == "eps"
                                && source.connector.as_str() == "stripe"
                                && source.group == RuleGroup::NonMandate
                                && path == EPS_PATH
                            {
                                if field.required_field != EPS_PATH {
                                    return Err(InputError::InputInvalid.into());
                                }
                                let FieldType::UserBankOptions { options } = &mut field_type else {
                                    return Err(InputError::InputInvalid.into());
                                };
                                options.sort();
                                if options.as_slice()
                                    != banks.stripe_eps_banks.as_deref().unwrap_or_default()
                                {
                                    // Do not erase unexpected duplicates or silently accept stale Settings.
                                    return Err(InputError::InputInvalid.into());
                                }
                            }
                            rules.push(RetainedRule {
                                source,
                                required_field: field.required_field.clone().try_into()?,
                                display_name: field.display_name.clone(),
                                field_type,
                            });
                        }
                    }
                }
            }
        }
        rules.sort_by(|left, right| left.source.cmp(&right.source));
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            banks,
            rules,
        })
    }

    pub fn rules(&self) -> &[RetainedRule] {
        &self.rules
    }

    pub fn settings_hash(&self) -> InputResult<Digest> {
        canonical_hash(HashDomain::EffectiveSettings, &self.banks)
    }

    pub fn retained_rules_hash(&self) -> InputResult<Digest> {
        canonical_hash(HashDomain::RetainedRules, &self.rules)
    }
}

/// Explicit extraction prevents custom FieldType::PartialEq from hiding option changes.
/// A new FieldType variant must be classified here; there is deliberately no wildcard.
pub fn field_options(field: &FieldType) -> Option<&[String]> {
    match field {
        FieldType::UserCountry { options }
        | FieldType::UserCurrency { options }
        | FieldType::UserAddressCountry { options }
        | FieldType::UserShippingAddressCountry { options }
        | FieldType::UserDocumentType { options }
        | FieldType::UserBankOptions { options }
        | FieldType::DropDown { options }
        | FieldType::LanguagePreference { options }
        | FieldType::UserBankType { options } => Some(options),
        FieldType::UserCardNumber
        | FieldType::UserGiftCardNumber
        | FieldType::UserCardExpiryMonth
        | FieldType::UserCardExpiryYear
        | FieldType::UserCardCvc
        | FieldType::UserGiftCardPin
        | FieldType::UserCardNetwork
        | FieldType::UserFullName
        | FieldType::UserEmailAddress
        | FieldType::UserPhoneNumber
        | FieldType::UserPhoneNumberCountryCode
        | FieldType::UserCryptoCurrencyNetwork
        | FieldType::UserBillingName
        | FieldType::UserAddressLine1
        | FieldType::UserAddressLine2
        | FieldType::UserAddressCity
        | FieldType::UserAddressPincode
        | FieldType::UserAddressState
        | FieldType::UserShippingName
        | FieldType::UserShippingAddressLine1
        | FieldType::UserShippingAddressLine2
        | FieldType::UserShippingAddressCity
        | FieldType::UserShippingAddressPincode
        | FieldType::UserShippingAddressState
        | FieldType::UserSocialSecurityNumber
        | FieldType::UserBlikCode
        | FieldType::UserBank
        | FieldType::UserBankAccountNumber
        | FieldType::UserSourceBankAccountId
        | FieldType::UserDestinationBankAccountId
        | FieldType::Text
        | FieldType::UserDateOfBirth
        | FieldType::UserVpaId
        | FieldType::UserPixKey
        | FieldType::UserCpf
        | FieldType::UserCnpj
        | FieldType::UserIban
        | FieldType::UserBsbNumber
        | FieldType::UserBankSortCode
        | FieldType::UserBankRoutingNumber
        | FieldType::UserBankAccountHolderName
        | FieldType::UserMsisdn
        | FieldType::UserClientIdentifier
        | FieldType::OrderDetailsProductName
        | FieldType::UserBranchCode
        | FieldType::UserBankIdentifier
        | FieldType::UserPixAccountNumber => None,
    }
}

#[derive(Debug)]
pub struct RenderInputs<'a> {
    pub effective: &'a EffectiveInputs,
    pub policy: &'a MappingPolicy,
    pub fields: &'a FieldContractCatalog,
    pub exclusions: &'a ExclusionPolicy,
    pub build: &'a BuildIdentity,
}

fn source_name<T: std::str::FromStr + ToString>(name: &Name) -> InputResult<()> {
    let value: T = name
        .as_str()
        .parse()
        .map_err(|_| InputError::InputInvalid)?;
    if value.to_string() != name.as_str() {
        return Err(InputError::InputInvalid.into());
    }
    Ok(())
}

fn validate_source(source: &SourceIdentity) -> InputResult<()> {
    source_name::<api_models::enums::PaymentMethod>(&source.payment_method)?;
    source_name::<api_models::enums::PaymentMethodType>(&source.payment_method_type)?;
    source_name::<api_models::enums::Connector>(&source.connector)
}

fn validate_policy_sources(policy: &MappingPolicy) -> InputResult<()> {
    policy.validate()?;
    for name in policy.connectors.keys() {
        source_name::<api_models::enums::Connector>(name)?;
    }
    for name in policy.payment_methods.keys() {
        source_name::<api_models::enums::PaymentMethod>(name)?;
    }
    for name in policy.payment_method_types.keys() {
        source_name::<api_models::enums::PaymentMethodType>(name)?;
    }
    for binding in &policy.bindings {
        validate_source(&binding.source)?;
    }
    Ok(())
}

/// Validate source vocabulary against the actual compiled enums, including feature availability.
/// Output labels still require the independent SDK catalog and Step 4's semantic validation.
pub fn parse_mapping_policy(text: &str) -> InputResult<MappingPolicy> {
    let policy = parse_contract(text)?;
    validate_policy_sources(&policy)?;
    Ok(policy)
}

pub fn parse_exclusion_policy(text: &str) -> InputResult<ExclusionPolicy> {
    let exclusions: ExclusionPolicy = parse_contract(text)?;
    for exclusion in &exclusions.sources {
        validate_source(&exclusion.source)?;
    }
    Ok(exclusions)
}

impl RenderInputs<'_> {
    pub fn manifest(&self) -> InputResult<InputManifest> {
        validate_policy_sources(self.policy)?;
        self.fields.validate()?;
        self.exclusions.validate()?;
        for exclusion in &self.exclusions.sources {
            validate_source(&exclusion.source)?;
        }
        // A retained manifest may record refused choices; it does not authorize rendering/apply.
        Ok(InputManifest {
            schema_version: SCHEMA_VERSION,
            canonical_version: CANONICAL_VERSION,
            build: self.build.clone(),
            settings_provenance: SettingsProvenance::EffectiveValuesOnly,
            effective_settings_hash: self.effective.settings_hash()?,
            retained_rules_hash: self.effective.retained_rules_hash()?,
            mapping_hash: canonical_hash(HashDomain::MappingPolicy, self.policy)?,
            catalog_hash: canonical_hash(HashDomain::FieldCatalog, self.fields)?,
            exclusions_hash: canonical_hash(HashDomain::Exclusions, self.exclusions)?,
        })
    }
}

/// Feature keys are embedded by Cargo at build time, never read from the runtime environment.
pub fn compiled_feature_keys() -> &'static str {
    env!("REQUIRED_FIELDS_FEATURE_KEYS")
}

#[cfg(test)]
#[path = "required_fields_renderer/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "required_fields_renderer/render_tests.rs"]
mod render_tests;
