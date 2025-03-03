use serde::{self, Serialize, Deserialize};
use api_models::enums;
use std::collections::{HashMap,HashSet};
use common_enums::enums::PaymentMethod;
use hyperswitch_interfaces::secrets_interface::secret_state::{
    RawSecret, SecretState, SecretStateContainer, SecuredSecret,
};
use hyperswitch_interfaces::configs::Connectors;
use masking::Secret;
use crate::core::domain::api::RequiredFieldInfo;

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct ConnectorFilters(pub HashMap<String, PaymentMethodFilters>);

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct PaymentMethodFilters(pub HashMap<PaymentMethodFilterKey, CurrencyCountryFlowFilter>);

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum PaymentMethodFilterKey {
    PaymentMethodType(enums::PaymentMethodType),
    CardNetwork(enums::CardNetwork),
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct CurrencyCountryFlowFilter {
    #[serde(deserialize_with = "deserialize_optional_hashset")]
    pub currency: Option<HashSet<enums::Currency>>,
    #[serde(deserialize_with = "deserialize_optional_hashset")]
    pub country: Option<HashSet<enums::CountryAlpha2>>,
    pub not_available_flows: Option<NotAvailableFlows>,
}

#[derive(Debug, Deserialize, Copy, Clone, Default)]
#[serde(default)]
pub struct NotAvailableFlows {
    pub capture_method: Option<enums::CaptureMethod>,
}

fn deserialize_hashset_inner<T>(value: impl AsRef<str>) -> Result<HashSet<T>, String>
where
    T: Eq + std::str::FromStr + std::hash::Hash,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let (values, errors) = value
        .as_ref()
        .trim()
        .split(',')
        .map(|s| {
            T::from_str(s.trim()).map_err(|error| {
                format!(
                    "Unable to deserialize `{}` as `{}`: {error}",
                    s.trim(),
                    std::any::type_name::<T>()
                )
            })
        })
        .fold(
            (HashSet::new(), Vec::new()),
            |(mut values, mut errors), result| match result {
                Ok(t) => {
                    values.insert(t);
                    (values, errors)
                }
                Err(error) => {
                    errors.push(error);
                    (values, errors)
                }
            },
        );
    if !errors.is_empty() {
        Err(format!("Some errors occurred:\n{}", errors.join("\n")))
    } else {
        Ok(values)
    }
}

fn deserialize_hashset<'a, D, T>(deserializer: D) -> Result<HashSet<T>, D::Error>
where
    D: serde::Deserializer<'a>,
    T: Eq + std::str::FromStr + std::hash::Hash,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    deserialize_hashset_inner(<String>::deserialize(deserializer)?).map_err(D::Error::custom)
}

fn deserialize_optional_hashset<'a, D, T>(deserializer: D) -> Result<Option<HashSet<T>>, D::Error>
where
    D: serde::Deserializer<'a>,
    T: Eq + std::str::FromStr + std::hash::Hash,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    <Option<String>>::deserialize(deserializer).map(|value| {
        value.map_or(Ok(None), |inner: String| {
            let list = deserialize_hashset_inner(inner).map_err(D::Error::custom)?;
            match list.len() {
                0 => Ok(None),
                _ => Ok(Some(list)),
            }
        })
    })?
}


#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Settings<S: SecretState> {
    pub locker: Locker,//
    pub connectors: Connectors,//
    pub jwekey: SecretStateContainer<Jwekey, S>,//
    pub pm_filters: ConnectorFilters,//
    pub bank_config: BankRedirectConfig,//
    pub mandates: Mandates,//
    pub required_fields: RequiredFields,//
    pub payment_method_auth: SecretStateContainer<PaymentMethodAuth, S>,//
    pub saved_payment_methods: EligiblePaymentMethods,//
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "v2", derive(Default))] // Configs are read from the config file in config/payment_required_fields.toml
pub struct RequiredFields(pub HashMap<enums::PaymentMethod, PaymentMethodType>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentMethodType(pub HashMap<enums::PaymentMethodType, ConnectorFields>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectorFields {
    pub fields: HashMap<enums::Connector, RequiredFieldFinal>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequiredFieldFinal {
    pub mandate: HashMap<String, RequiredFieldInfo>,
    pub non_mandate: HashMap<String, RequiredFieldInfo>,
    pub common: HashMap<String, RequiredFieldInfo>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Deserialize, Clone)]
pub struct RequiredFieldFinal {
    pub mandate: Option<Vec<RequiredFieldInfo>>,
    pub non_mandate: Option<Vec<RequiredFieldInfo>>,
    pub common: Option<Vec<RequiredFieldInfo>>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PaymentMethodAuth {
    pub redis_expiry: i64,
    pub pm_auth_key: Secret<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EligiblePaymentMethods {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub sdk_eligible_payment_methods: HashSet<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Jwekey {
    pub vault_encryption_key: Secret<String>,
    pub rust_locker_encryption_key: Secret<String>,
    pub vault_private_key: Secret<String>,
    pub tunnel_private_key: Secret<String>,
}


#[derive(Debug, Deserialize, Clone)]
pub struct SupportedPaymentMethodsForMandate(
    pub HashMap<enums::PaymentMethod, SupportedPaymentMethodTypesForMandate>,
);

#[derive(Debug, Deserialize, Clone)]
pub struct SupportedPaymentMethodTypesForMandate(
    pub HashMap<enums::PaymentMethodType, SupportedConnectorsForMandate>,
);

#[derive(Debug, Deserialize, Clone)]
pub struct SupportedConnectorsForMandate {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connector_list: HashSet<enums::Connector>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PaymentMethodTokenFilter {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub payment_method: HashSet<diesel_models::enums::PaymentMethod>,
    pub payment_method_type: Option<PaymentMethodTypeTokenFilter>,
    pub long_lived_token: bool,
    pub apple_pay_pre_decrypt_flow: Option<ApplePayPreDecryptFlow>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ApplePayPreDecryptFlow {
    #[default]
    ConnectorTokenization,
    NetworkTokenization,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TempLockerEnablePaymentMethodFilter {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub payment_method: HashSet<diesel_models::enums::PaymentMethod>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(
    deny_unknown_fields,
    tag = "type",
    content = "list",
    rename_all = "snake_case"
)]
pub enum PaymentMethodTypeTokenFilter {
    #[serde(deserialize_with = "deserialize_hashset")]
    EnableOnly(HashSet<diesel_models::enums::PaymentMethodType>),
    #[serde(deserialize_with = "deserialize_hashset")]
    DisableOnly(HashSet<diesel_models::enums::PaymentMethodType>),
    #[default]
    AllAccepted,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct BankRedirectConfig(pub HashMap<enums::PaymentMethodType, ConnectorBankNames>);
#[derive(Debug, Deserialize, Clone)]
pub struct ConnectorBankNames(pub HashMap<String, BanksVector>);

#[derive(Debug, Deserialize, Clone)]
pub struct BanksVector {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub banks: HashSet<common_enums::enums::BankNames>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Locker {
    pub host: String,
    pub host_rs: String,
    pub mock_locker: bool,
    pub basilisk_host: String,
    pub locker_signing_key_id: String,
    pub locker_enabled: bool,
    pub ttl_for_storage_in_secs: i64,
    pub decryption_scheme: DecryptionScheme,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub enum DecryptionScheme {
    #[default]
    #[serde(rename = "RSA-OAEP")]
    RsaOaep,
    #[serde(rename = "RSA-OAEP-256")]
    RsaOaep256,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Mandates {
    pub supported_payment_methods: SupportedPaymentMethodsForMandate,
    pub update_mandate_supported: SupportedPaymentMethodsForMandate,
}

impl Default for super::settings::Locker {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            host_rs: "localhost".into(),
            mock_locker: true,
            basilisk_host: "localhost".into(),
            locker_signing_key_id: "1".into(),
            //true or false
            locker_enabled: true,
            //Time to live for storage entries in locker
            ttl_for_storage_in_secs: 60 * 60 * 24 * 365 * 7,
            decryption_scheme: Default::default(),
        }
    }
}