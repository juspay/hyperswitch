use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
};

use api_models::enums;

#[cfg(feature = "email")]
use external_services::email::EmailSettings;
#[cfg(feature = "kms")]
use external_services::kms;
use storage_models::settings as storage_settings;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use serde::{de::Error, Deserialize, Deserializer};

pub use self::storage_settings::*;

#[derive(clap::Parser, Default)]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,

    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(clap::Parser)]
pub enum Subcommand {
    #[cfg(feature = "openapi")]
    /// Generate the OpenAPI specification file from code.
    GenerateOpenapiSpec,
}

#[cfg(feature = "kms")]
/// Store the decrypted kms secret values for active use in the application
/// Currently using `StrongSecret` won't have any effect as this struct have smart pointers to heap
/// allocations.
/// note: we can consider adding such behaviour in the future with custom implementation
#[derive(Clone)]
pub struct ActiveKmsSecrets {
    pub jwekey: masking::Secret<Jwekey>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct TokenizationConfig(pub HashMap<String, PaymentMethodTokenFilter>);

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ConnectorCustomer {
    #[serde(deserialize_with = "connector_deser")]
    pub connector_list: HashSet<api_models::enums::Connector>,
}

fn connector_deser<'a, D>(
    deserializer: D,
) -> Result<HashSet<api_models::enums::Connector>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <String>::deserialize(deserializer)?;
    Ok(value
        .trim()
        .split(',')
        .flat_map(api_models::enums::Connector::from_str)
        .collect())
}

#[cfg(feature = "dummy_connector")]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct DummyConnector {
    pub payment_ttl: i64,
    pub payment_duration: u64,
    pub payment_tolerance: u64,
    pub payment_retrieve_duration: u64,
    pub payment_retrieve_tolerance: u64,
    pub refund_ttl: i64,
    pub refund_duration: u64,
    pub refund_tolerance: u64,
    pub refund_retrieve_duration: u64,
    pub refund_retrieve_tolerance: u64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PaymentMethodTokenFilter {
    #[serde(deserialize_with = "pm_deser")]
    pub payment_method: HashSet<storage_models::enums::PaymentMethod>,
    pub payment_method_type: Option<PaymentMethodTypeTokenFilter>,
    pub long_lived_token: bool,
}

fn pm_deser<'a, D>(
    deserializer: D,
) -> Result<HashSet<storage_models::enums::PaymentMethod>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <String>::deserialize(deserializer)?;
    value
        .trim()
        .split(',')
        .map(storage_models::enums::PaymentMethod::from_str)
        .collect::<Result<_, _>>()
        .map_err(D::Error::custom)
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct BankRedirectConfig(
    pub HashMap<api_models::enums::PaymentMethodType, ConnectorBankNames>,
);
#[derive(Debug, Deserialize, Clone)]
pub struct ConnectorBankNames(pub HashMap<String, BanksVector>);

#[derive(Debug, Deserialize, Clone)]
pub struct BanksVector {
    #[serde(deserialize_with = "bank_vec_deser")]
    pub banks: HashSet<api_models::enums::BankNames>,
}

#[derive(Debug, Deserialize, Copy, Clone, Default)]
#[serde(default)]
pub struct NotAvailableFlows {
    pub capture_method: Option<enums::CaptureMethod>,
}


fn bank_vec_deser<'a, D>(deserializer: D) -> Result<HashSet<api_models::enums::BankNames>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <String>::deserialize(deserializer)?;
    Ok(value
        .trim()
        .split(',')
        .flat_map(api_models::enums::BankNames::from_str)
        .collect())
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Refund {
    pub max_attempts: usize,
    pub max_age: i64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct EphemeralConfig {
    pub validity: i64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct SupportedConnectors {
    pub wallets: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ConnectorParamsWithMoreUrls {
    pub base_url: String,
    pub base_url_bank_redirects: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ConnectorParamsWithFileUploadUrl {
    pub base_url: String,
    pub base_url_file_upload: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WebhooksSettings {
    pub outgoing_enabled: bool,
}

#[cfg(feature = "s3")]
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct FileUploadConfig {
    /// The AWS region to send file uploads
    pub region: String,
    /// The AWS s3 bucket to send file uploads
    pub bucket_name: String,
}
#[cfg(test)]
mod payment_method_deserialization_test {
    #![allow(clippy::unwrap_used)]
    use serde::de::{
        value::{Error as ValueError, StrDeserializer},
        IntoDeserializer,
    };

    use super::*;

    #[test]
    fn test_pm_deserializer() {
        let deserializer: StrDeserializer<'_, ValueError> = "wallet,card".into_deserializer();
        let test_pm = pm_deser(deserializer);
        assert!(test_pm.is_ok())
    }
}
