use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
};

use api_models::{enums, payment_methods::RequiredFieldInfo};
use common_utils::ext_traits::ConfigExt;
use config::{Environment, File};
#[cfg(feature = "email")]
use external_services::email::EmailSettings;
#[cfg(feature = "kms")]
use external_services::kms;
use redis_interface::RedisSettings;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use serde::{de::Error, Deserialize, Deserializer};

use crate::{
    core::errors::{ApplicationError, ApplicationResult},
    env::{self, logger, Env},
};

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
#[serde(default)]
pub struct Settings {
    pub server: Server,
    pub proxy: Proxy,
    pub env: Env,
    pub master_database: Database,
    #[cfg(feature = "olap")]
    pub replica_database: Database,
    pub redis: RedisSettings,
    pub log: Log,
    pub secrets: Secrets,
    pub locker: Locker,
    pub connectors: Connectors,
    pub refund: Refund,
    pub eph_key: EphemeralConfig,
    pub scheduler: Option<SchedulerSettings>,
    #[cfg(feature = "kv_store")]
    pub drainer: DrainerSettings,
    pub jwekey: Jwekey,
    pub webhooks: WebhooksSettings,
    pub pm_filters: ConnectorFilters,
    pub bank_config: BankRedirectConfig,
    pub api_keys: ApiKeys,
    #[cfg(feature = "kms")]
    pub kms: kms::KmsConfig,
    #[cfg(feature = "s3")]
    pub file_upload_config: FileUploadConfig,
    pub tokenization: TokenizationConfig,
    pub connector_customer: ConnectorCustomer,
    #[cfg(feature = "dummy_connector")]
    pub dummy_connector: DummyConnector,
    #[cfg(feature = "email")]
    pub email: EmailSettings,
    pub required_fields: RequiredFields,
    pub delayed_session_response: DelayedSessionConfig,
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

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(
    deny_unknown_fields,
    tag = "type",
    content = "list",
    rename_all = "snake_case"
)]
pub enum PaymentMethodTypeTokenFilter {
    #[serde(deserialize_with = "pm_type_deser")]
    EnableOnly(HashSet<storage_models::enums::PaymentMethodType>),
    #[serde(deserialize_with = "pm_type_deser")]
    DisableOnly(HashSet<storage_models::enums::PaymentMethodType>),
    #[default]
    AllAccepted,
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

fn pm_type_deser<'a, D>(
    deserializer: D,
) -> Result<HashSet<storage_models::enums::PaymentMethodType>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <String>::deserialize(deserializer)?;
    value
        .trim()
        .split(',')
        .map(storage_models::enums::PaymentMethodType::from_str)
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

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct ConnectorFilters(pub HashMap<String, PaymentMethodFilters>);

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct PaymentMethodFilters(pub HashMap<PaymentMethodFilterKey, CurrencyCountryFlowFilter>);

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum PaymentMethodFilterKey {
    PaymentMethodType(api_models::enums::PaymentMethodType),
    CardNetwork(api_models::enums::CardNetwork),
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct CurrencyCountryFlowFilter {
    #[serde(deserialize_with = "currency_set_deser")]
    pub currency: Option<HashSet<api_models::enums::Currency>>,
    #[serde(deserialize_with = "string_set_deser")]
    pub country: Option<HashSet<api_models::enums::CountryAlpha2>>,
    pub not_available_flows: Option<NotAvailableFlows>,
}

#[derive(Debug, Deserialize, Copy, Clone, Default)]
#[serde(default)]
pub struct NotAvailableFlows {
    pub capture_method: Option<enums::CaptureMethod>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RequiredFields(pub HashMap<enums::PaymentMethod, PaymentMethodType>);

#[derive(Debug, Deserialize, Clone)]
pub struct PaymentMethodType(pub HashMap<enums::PaymentMethodType, ConnectorFields>);

#[derive(Debug, Deserialize, Clone)]
pub struct ConnectorFields {
    pub fields: HashMap<enums::Connector, Vec<RequiredFieldInfo>>,
}

fn string_set_deser<'a, D>(
    deserializer: D,
) -> Result<Option<HashSet<api_models::enums::CountryAlpha2>>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <Option<String>>::deserialize(deserializer)?;
    Ok(value.and_then(|inner| {
        let list = inner
            .trim()
            .split(',')
            .flat_map(api_models::enums::CountryAlpha2::from_str)
            .collect::<HashSet<_>>();
        match list.len() {
            0 => None,
            _ => Some(list),
        }
    }))
}

fn currency_set_deser<'a, D>(
    deserializer: D,
) -> Result<Option<HashSet<api_models::enums::Currency>>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <Option<String>>::deserialize(deserializer)?;
    Ok(value.and_then(|inner| {
        let list = inner
            .trim()
            .split(',')
            .flat_map(api_models::enums::Currency::from_str)
            .collect::<HashSet<_>>();
        match list.len() {
            0 => None,
            _ => Some(list),
        }
    }))
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
pub struct Secrets {
    #[cfg(not(feature = "kms"))]
    pub jwt_secret: String,
    #[cfg(not(feature = "kms"))]
    pub admin_api_key: String,
    pub master_enc_key: String,
    #[cfg(feature = "kms")]
    pub kms_encrypted_jwt_secret: String,
    #[cfg(feature = "kms")]
    pub kms_encrypted_admin_api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Locker {
    pub host: String,
    pub mock_locker: bool,
    pub basilisk_host: String,
    pub locker_setup: LockerSetup,
    pub locker_signing_key_id: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum LockerSetup {
    #[default]
    LegacyLocker,
    BasiliskLocker,
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

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Jwekey {
    pub locker_key_identifier1: String,
    pub locker_key_identifier2: String,
    pub locker_encryption_key1: String,
    pub locker_encryption_key2: String,
    pub locker_decryption_key1: String,
    pub locker_decryption_key2: String,
    pub vault_encryption_key: String,
    pub vault_private_key: String,
    pub tunnel_private_key: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Proxy {
    pub http_url: Option<String>,
    pub https_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Server {
    pub port: u16,
    pub workers: usize,
    pub host: String,
    pub request_body_limit: usize,
    pub base_url: String,
    pub shutdown_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Database {
    pub username: String,
    #[cfg(not(feature = "kms"))]
    pub password: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
    #[cfg(feature = "kms")]
    pub kms_encrypted_password: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct SupportedConnectors {
    pub wallets: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Connectors {
    pub aci: ConnectorParams,
    pub adyen: ConnectorParams,
    pub airwallex: ConnectorParams,
    pub applepay: ConnectorParams,
    pub authorizedotnet: ConnectorParams,
    pub bambora: ConnectorParams,
    pub bitpay: ConnectorParams,
    pub bluesnap: ConnectorParams,
    pub braintree: ConnectorParams,
    pub cashtocode: ConnectorParams,
    pub checkout: ConnectorParams,
    pub coinbase: ConnectorParams,
    pub cryptopay: ConnectorParams,
    pub cybersource: ConnectorParams,
    pub dlocal: ConnectorParams,
    #[cfg(feature = "dummy_connector")]
    pub dummyconnector: ConnectorParams,
    pub fiserv: ConnectorParams,
    pub forte: ConnectorParams,
    pub globalpay: ConnectorParams,
    pub globepay: ConnectorParams,
    pub iatapay: ConnectorParams,
    pub klarna: ConnectorParams,
    pub mollie: ConnectorParams,
    pub multisafepay: ConnectorParams,
    pub nexinets: ConnectorParams,
    pub nmi: ConnectorParams,
    pub noon: ConnectorParams,
    pub nuvei: ConnectorParams,
    pub opayo: ConnectorParams,
    pub opennode: ConnectorParams,
    pub payeezy: ConnectorParams,
    pub payme: ConnectorParams,
    pub paypal: ConnectorParams,
    pub payu: ConnectorParams,
    pub powertranz: ConnectorParams,
    pub rapyd: ConnectorParams,
    pub shift4: ConnectorParams,
    pub stripe: ConnectorParamsWithFileUploadUrl,
    pub trustpay: ConnectorParamsWithMoreUrls,
    pub worldline: ConnectorParams,
    pub worldpay: ConnectorParams,
    pub zen: ConnectorParams,

    // Keep this field separate from the remaining fields
    pub supported: SupportedConnectors,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ConnectorParams {
    pub base_url: String,
    pub secondary_base_url: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerSettings {
    pub stream: String,
    pub producer: ProducerSettings,
    pub consumer: ConsumerSettings,
    pub loop_interval: u64,
    pub graceful_shutdown_interval: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProducerSettings {
    pub upper_fetch_limit: i64,
    pub lower_fetch_limit: i64,

    pub lock_key: String,
    pub lock_ttl: i64,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ConsumerSettings {
    pub disabled: bool,
    pub consumer_group: String,
}

#[cfg(feature = "kv_store")]
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DrainerSettings {
    pub stream_name: String,
    pub num_partitions: u8,
    pub max_read_count: u64,
    pub shutdown_interval: u32, // in milliseconds
    pub loop_interval: u32,     // in milliseconds
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WebhooksSettings {
    pub outgoing_enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ApiKeys {
    /// Base64-encoded (KMS encrypted) ciphertext of the key used for calculating hashes of API
    /// keys
    #[cfg(feature = "kms")]
    pub kms_encrypted_hash_key: String,

    /// Hex-encoded 32-byte long (64 characters long when hex-encoded) key used for calculating
    /// hashes of API keys
    #[cfg(not(feature = "kms"))]
    pub hash_key: String,

    // Specifies the number of days before API key expiry when email reminders should be sent
    #[cfg(feature = "email")]
    pub expiry_reminder_days: Vec<u8>,
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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DelayedSessionConfig {
    #[serde(deserialize_with = "delayed_session_deser")]
    pub connectors_with_delayed_session_response: HashSet<api_models::enums::Connector>,
}

fn delayed_session_deser<'a, D>(
    deserializer: D,
) -> Result<HashSet<api_models::enums::Connector>, D::Error>
where
    D: Deserializer<'a>,
{
    let value = <String>::deserialize(deserializer)?;
    value
        .trim()
        .split(',')
        .map(api_models::enums::Connector::from_str)
        .collect::<Result<_, _>>()
        .map_err(D::Error::custom)
}

impl Settings {
    pub fn new() -> ApplicationResult<Self> {
        Self::with_config_path(None)
    }

    pub fn with_config_path(config_path: Option<PathBuf>) -> ApplicationResult<Self> {
        // Configuration values are picked up in the following priority order (1 being least
        // priority):
        // 1. Defaults from the implementation of the `Default` trait.
        // 2. Values from config file. The config file accessed depends on the environment
        //    specified by the `RUN_ENV` environment variable. `RUN_ENV` can be one of
        //    `development`, `sandbox` or `production`. If nothing is specified for `RUN_ENV`,
        //    `/config/development.toml` file is read.
        // 3. Environment variables prefixed with `ROUTER` and each level separated by double
        //    underscores.
        //
        // Values in config file override the defaults in `Default` trait, and the values set using
        // environment variables override both the defaults and the config file values.

        let environment = env::which();
        let config_path = router_env::Config::config_path(&environment.to_string(), config_path);

        let config = router_env::Config::builder(&environment.to_string())?
            .add_source(File::from(config_path).required(true))
            .add_source(
                Environment::with_prefix("ROUTER")
                    .try_parsing(true)
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("redis.cluster_urls")
                    .with_list_parse_key("connectors.supported.wallets"),
            )
            .build()?;

        serde_path_to_error::deserialize(config).map_err(|error| {
            logger::error!(%error, "Unable to deserialize application configuration");
            eprintln!("Unable to deserialize application configuration: {error}");
            ApplicationError::from(error.into_inner())
        })
    }

    pub fn validate(&self) -> ApplicationResult<()> {
        self.server.validate()?;
        self.master_database.validate()?;
        #[cfg(feature = "olap")]
        self.replica_database.validate()?;
        self.redis.validate().map_err(|error| {
            println!("{error}");
            ApplicationError::InvalidConfigurationValueError("Redis configuration".into())
        })?;
        if self.log.file.enabled {
            if self.log.file.file_name.is_default_or_empty() {
                return Err(ApplicationError::InvalidConfigurationValueError(
                    "log file name must not be empty".into(),
                ));
            }

            if self.log.file.path.is_default_or_empty() {
                return Err(ApplicationError::InvalidConfigurationValueError(
                    "log directory path must not be empty".into(),
                ));
            }
        }
        self.secrets.validate()?;
        self.locker.validate()?;
        self.connectors.validate()?;

        self.scheduler
            .as_ref()
            .map(|scheduler_settings| scheduler_settings.validate())
            .transpose()?;
        #[cfg(feature = "kv_store")]
        self.drainer.validate()?;
        self.api_keys.validate()?;
        #[cfg(feature = "kms")]
        self.kms
            .validate()
            .map_err(|error| ApplicationError::InvalidConfigurationValueError(error.into()))?;
        #[cfg(feature = "s3")]
        self.file_upload_config.validate()?;
        Ok(())
    }
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
