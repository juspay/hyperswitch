use std::collections::{HashMap, HashSet};

use api_models::{enums, payment_methods::RequiredFieldInfo};
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
