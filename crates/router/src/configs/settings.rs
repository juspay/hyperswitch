use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
};

use api_models::enums;
use diesel_models::settings as storage_settings;
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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PaymentMethodTokenFilter {
    #[serde(deserialize_with = "pm_deser")]
    pub payment_method: HashSet<diesel_models::enums::PaymentMethod>,
    pub payment_method_type: Option<PaymentMethodTypeTokenFilter>,
    pub long_lived_token: bool,
}

fn pm_deser<'a, D>(
    deserializer: D,
) -> Result<HashSet<diesel_models::enums::PaymentMethod>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <String>::deserialize(deserializer)?;
    value
        .trim()
        .split(',')
        .map(diesel_models::enums::PaymentMethod::from_str)
        .collect::<Result<_, _>>()
        .map_err(D::Error::custom)
}
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
