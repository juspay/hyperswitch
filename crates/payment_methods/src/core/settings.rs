use std::collections::{HashMap, HashSet};

use api_models::enums;
use common_utils::{
    ext_traits::ConfigExt,
    transformers::{ForeignFrom, ForeignInto},
};
use hyperswitch_interfaces::{
    configs::Connectors,
    secrets_interface::{
        secret_handler::SecretsHandler,
        secret_state::{RawSecret, SecretState, SecretStateContainer, SecuredSecret},
        SecretManagementInterface, SecretsManagementError,
    },
};
use kgraph_utils::types;
use masking::Secret;
use serde::{self, Deserialize, Serialize};

use crate::core::{
    domain::{api::RequiredFieldInfo, enums::ApplicationError},
    errors::CustomResult,
    utils::deserialize_hashmap,
};

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
    pub locker: Locker,
    pub connectors: Connectors,
    pub jwekey: SecretStateContainer<Jwekey, S>,
    pub pm_filters: ConnectorFilters,
    pub bank_config: BankRedirectConfig,
    pub mandates: Mandates,
    pub required_fields: RequiredFields,
    pub payment_method_auth: SecretStateContainer<PaymentMethodAuth, S>,
    pub saved_payment_methods: EligiblePaymentMethods,
    pub generic_link: GenericLink,
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
pub struct GenericLink {
    pub payment_method_collect: GenericLinkEnvConfig,
    pub payout_link: GenericLinkEnvConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GenericLinkEnvConfig {
    pub sdk_url: url::Url,
    pub expiry: u32,
    pub ui_config: GenericLinkEnvUiConfig,
    #[serde(deserialize_with = "deserialize_hashmap")]
    pub enabled_payment_methods: HashMap<enums::PaymentMethod, HashSet<enums::PaymentMethodType>>,
}

impl Default for GenericLinkEnvConfig {
    fn default() -> Self {
        Self {
            #[allow(clippy::expect_used)]
            sdk_url: url::Url::parse("http://localhost:9050/HyperLoader.js")
                .expect("Failed to parse default SDK URL"),
            expiry: 900,
            ui_config: GenericLinkEnvUiConfig::default(),
            enabled_payment_methods: HashMap::default(),
        }
    }
}

impl GenericLinkEnvConfig {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(self.expiry == 0, || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "link's expiry should not be 0".into(),
            ))
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct GenericLinkEnvUiConfig {
    pub logo: url::Url,
    pub merchant_name: Secret<String>,
    pub theme: String,
}

#[allow(clippy::panic)]
impl Default for GenericLinkEnvUiConfig {
    fn default() -> Self {
        Self {
            #[allow(clippy::expect_used)]
            logo: url::Url::parse("https://hyperswitch.io/favicon.ico")
                .expect("Failed to parse default logo URL"),
            merchant_name: Secret::new("HyperSwitch".to_string()),
            theme: "#4285F4".to_string(),
        }
    }
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

impl Default for Locker {
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
impl Locker {
    pub fn validate(&self) -> Result<(), ApplicationError> {
        use common_utils::fp_utils::when;

        when(!self.mock_locker && self.host.is_default_or_empty(), || {
            Err(ApplicationError::InvalidConfigurationValueError(
                "locker host must not be empty when mock locker is disabled".into(),
            ))
        })?;

        when(
            !self.mock_locker && self.basilisk_host.is_default_or_empty(),
            || {
                Err(ApplicationError::InvalidConfigurationValueError(
                    "basilisk host must not be empty when mock locker is disabled".into(),
                ))
            },
        )
    }
}

#[async_trait::async_trait]
impl SecretsHandler for Jwekey {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let jwekey = value.get_inner();
        let (
            vault_encryption_key,
            rust_locker_encryption_key,
            vault_private_key,
            tunnel_private_key,
        ) = tokio::try_join!(
            secret_management_client.get_secret(jwekey.vault_encryption_key.clone()),
            secret_management_client.get_secret(jwekey.rust_locker_encryption_key.clone()),
            secret_management_client.get_secret(jwekey.vault_private_key.clone()),
            secret_management_client.get_secret(jwekey.tunnel_private_key.clone())
        )?;
        Ok(value.transition_state(|_| Self {
            vault_encryption_key,
            rust_locker_encryption_key,
            vault_private_key,
            tunnel_private_key,
        }))
    }
}

#[async_trait::async_trait]
impl SecretsHandler for PaymentMethodAuth {
    async fn convert_to_raw_secret(
        value: SecretStateContainer<Self, SecuredSecret>,
        secret_management_client: &dyn SecretManagementInterface,
    ) -> CustomResult<SecretStateContainer<Self, RawSecret>, SecretsManagementError> {
        let payment_method_auth = value.get_inner();

        let pm_auth_key = secret_management_client
            .get_secret(payment_method_auth.pm_auth_key.clone())
            .await?;

        Ok(value.transition_state(|payment_method_auth| Self {
            pm_auth_key,
            ..payment_method_auth
        }))
    }
}

impl ForeignFrom<PaymentMethodFilterKey> for types::PaymentMethodFilterKey {
    fn foreign_from(from: PaymentMethodFilterKey) -> Self {
        match from {
            PaymentMethodFilterKey::PaymentMethodType(pmt) => Self::PaymentMethodType(pmt),
            PaymentMethodFilterKey::CardNetwork(cn) => Self::CardNetwork(cn),
        }
    }
}
impl ForeignFrom<CurrencyCountryFlowFilter> for types::CurrencyCountryFlowFilter {
    fn foreign_from(from: CurrencyCountryFlowFilter) -> Self {
        Self {
            currency: from.currency,
            country: from.country,
            not_available_flows: from.not_available_flows.map(ForeignInto::foreign_into),
        }
    }
}
impl ForeignFrom<NotAvailableFlows> for types::NotAvailableFlows {
    fn foreign_from(from: NotAvailableFlows) -> Self {
        Self {
            capture_method: from.capture_method,
        }
    }
}
impl ForeignFrom<PaymentMethodFilters> for types::PaymentMethodFilters {
    fn foreign_from(from: PaymentMethodFilters) -> Self {
        let iter_map = from
            .0
            .into_iter()
            .map(|(key, val)| (key.foreign_into(), val.foreign_into()))
            .collect::<HashMap<_, _>>();
        Self(iter_map)
    }
}
