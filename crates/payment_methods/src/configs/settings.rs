use std::collections::{HashMap, HashSet};

use api_models::{enums, payment_methods::RequiredFieldInfo};
use common_utils::errors::CustomResult;
use hyperswitch_interfaces::secrets_interface::{
    secret_handler::SecretsHandler,
    secret_state::{RawSecret, SecretStateContainer, SecuredSecret},
    SecretManagementInterface, SecretsManagementError,
};
use masking::Secret;
use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "v2", derive(Default))] // Configs are read from the config file in config/payment_required_fields.toml
pub struct RequiredFields(pub HashMap<enums::PaymentMethod, PaymentMethodType>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentMethodType(pub HashMap<enums::PaymentMethodType, ConnectorFields>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectorFields {
    pub fields: HashMap<enums::Connector, RequiredFieldFinal>,
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

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequiredFieldFinal {
    pub mandate: HashMap<String, RequiredFieldInfo>,
    pub non_mandate: HashMap<String, RequiredFieldInfo>,
    pub common: HashMap<String, RequiredFieldInfo>,
}
#[cfg(feature = "v2")]
#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EligiblePaymentMethods {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub sdk_eligible_payment_methods: HashSet<String>,
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
#[derive(Debug, Deserialize, Clone)]
pub struct Mandates {
    pub supported_payment_methods: SupportedPaymentMethodsForMandate,
    pub update_mandate_supported: SupportedPaymentMethodsForMandate,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ZeroMandates {
    pub supported_payment_methods: SupportedPaymentMethodsForMandate,
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
