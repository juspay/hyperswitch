use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[cfg(feature = "olap")]
use analytics::{OpensearchConfig, ReportConfig};
use api_models::{enums, payment_methods::RequiredFieldInfo};
use common_utils::ext_traits::ConfigExt;
use config::{Environment, File};
#[cfg(feature = "email")]
use external_services::email::EmailSettings;
use external_services::{
    file_storage::FileStorageConfig,
    managers::{
        encryption_management::EncryptionManagementConfig,
        secrets_management::SecretsManagementConfig,
    },
};
use hyperswitch_interfaces::secrets_interface::secret_state::{
    SecretState, SecretStateContainer, SecuredSecret,
};
use masking::Secret;
use redis_interface::RedisSettings;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use rust_decimal::Decimal;
use scheduler::SchedulerSettings;
use serde::Deserialize;
use storage_impl::config::QueueStrategy;

#[cfg(feature = "olap")]
use crate::analytics::AnalyticsConfig;
use crate::{
    core::errors::{ApplicationError, ApplicationResult},
    env::{self, logger, Env},
    events::EventsConfig,
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

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Settings<S: SecretState> {
    pub server: Server,
    pub proxy: Proxy,
    pub env: Env,
    pub master_database: SecretStateContainer<Database, S>,
    #[cfg(feature = "olap")]
    pub replica_database: SecretStateContainer<Database, S>,
    pub redis: RedisSettings,
    pub log: Log,
    pub secrets: SecretStateContainer<Secrets, S>,
    pub locker: Locker,
    pub connectors: Connectors,
    pub forex_api: SecretStateContainer<ForexApi, S>,
    pub refund: Refund,
    pub eph_key: EphemeralConfig,
    pub scheduler: Option<SchedulerSettings>,
    #[cfg(feature = "kv_store")]
    pub drainer: DrainerSettings,
    pub jwekey: SecretStateContainer<Jwekey, S>,
    pub webhooks: WebhooksSettings,
    pub pm_filters: ConnectorFilters,
    pub bank_config: BankRedirectConfig,
    pub api_keys: SecretStateContainer<ApiKeys, S>,
    pub file_storage: FileStorageConfig,
    pub encryption_management: EncryptionManagementConfig,
    pub secrets_management: SecretsManagementConfig,
    pub tokenization: TokenizationConfig,
    pub connector_customer: ConnectorCustomer,
    #[cfg(feature = "dummy_connector")]
    pub dummy_connector: DummyConnector,
    #[cfg(feature = "email")]
    pub email: EmailSettings,
    pub cors: CorsSettings,
    pub mandates: Mandates,
    pub required_fields: RequiredFields,
    pub delayed_session_response: DelayedSessionConfig,
    pub webhook_source_verification_call: WebhookSourceVerificationCall,
    pub payment_method_auth: SecretStateContainer<PaymentMethodAuth, S>,
    pub connector_request_reference_id_config: ConnectorRequestReferenceIdConfig,
    #[cfg(feature = "payouts")]
    pub payouts: Payouts,
    pub applepay_decrypt_keys: SecretStateContainer<ApplePayDecryptConifg, S>,
    pub multiple_api_version_supported_connectors: MultipleApiVersionSupportedConnectors,
    pub applepay_merchant_configs: SecretStateContainer<ApplepayMerchantConfigs, S>,
    pub lock_settings: LockSettings,
    pub temp_locker_enable_config: TempLockerEnableConfig,
    pub payment_link: PaymentLink,
    #[cfg(feature = "olap")]
    pub analytics: SecretStateContainer<AnalyticsConfig, S>,
    #[cfg(feature = "kv_store")]
    pub kv_config: KvConfig,
    #[cfg(feature = "frm")]
    pub frm: Frm,
    #[cfg(feature = "olap")]
    pub report_download_config: ReportConfig,
    #[cfg(feature = "olap")]
    pub opensearch: OpensearchConfig,
    pub events: EventsConfig,
    #[cfg(feature = "olap")]
    pub connector_onboarding: SecretStateContainer<ConnectorOnboarding, S>,
    pub unmasked_headers: UnmaskedHeaders,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct UnmaskedHeaders {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub keys: HashSet<String>,
}

#[cfg(feature = "frm")]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Frm {
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KvConfig {
    pub ttl: u32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PaymentLink {
    pub sdk_url: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ForexApi {
    pub local_fetch_retry_count: u64,
    pub api_key: Secret<String>,
    pub fallback_api_key: Secret<String>,
    /// in ms
    pub call_delay: i64,
    /// in ms
    pub local_fetch_retry_delay: u64,
    /// in ms
    pub api_timeout: u64,
    /// in ms
    pub redis_lock_timeout: u64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PaymentMethodAuth {
    pub redis_expiry: i64,
    pub pm_auth_key: Secret<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DefaultExchangeRates {
    pub base_currency: String,
    pub conversion: HashMap<String, Conversion>,
    pub timestamp: i64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Conversion {
    #[serde(with = "rust_decimal::serde::str")]
    pub to_factor: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub from_factor: Decimal,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ApplepayMerchantConfigs {
    pub merchant_cert: Secret<String>,
    pub merchant_cert_key: Secret<String>,
    pub common_merchant_identifier: Secret<String>,
    pub applepay_endpoint: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MultipleApiVersionSupportedConnectors {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub supported_connectors: HashSet<api_models::enums::Connector>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct TokenizationConfig(pub HashMap<String, PaymentMethodTokenFilter>);

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct TempLockerEnableConfig(pub HashMap<String, TempLockerEnablePaymentMethodFilter>);

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ConnectorCustomer {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connector_list: HashSet<api_models::enums::Connector>,
    #[cfg(feature = "payouts")]
    #[serde(deserialize_with = "deserialize_hashset")]
    pub payout_connector_list: HashSet<api_models::enums::PayoutConnectors>,
}

#[cfg(feature = "dummy_connector")]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct DummyConnector {
    pub enabled: bool,
    pub payment_ttl: i64,
    pub payment_duration: u64,
    pub payment_tolerance: u64,
    pub payment_retrieve_duration: u64,
    pub payment_retrieve_tolerance: u64,
    pub payment_complete_duration: i64,
    pub payment_complete_tolerance: i64,
    pub refund_ttl: i64,
    pub refund_duration: u64,
    pub refund_tolerance: u64,
    pub refund_retrieve_duration: u64,
    pub refund_retrieve_tolerance: u64,
    pub authorize_ttl: i64,
    pub assets_base_url: String,
    pub default_return_url: String,
    pub slack_invite_url: String,
    pub discord_invite_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CorsSettings {
    #[serde(default, deserialize_with = "deserialize_hashset")]
    pub origins: HashSet<String>,
    #[serde(default)]
    pub wildcard_origin: bool,
    pub max_age: usize,
    #[serde(deserialize_with = "deserialize_hashset")]
    pub allowed_methods: HashSet<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Mandates {
    pub supported_payment_methods: SupportedPaymentMethodsForMandate,
    pub update_mandate_supported: SupportedPaymentMethodsForMandate,
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
    pub connector_list: HashSet<api_models::enums::Connector>,
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
pub struct BankRedirectConfig(
    pub HashMap<api_models::enums::PaymentMethodType, ConnectorBankNames>,
);
#[derive(Debug, Deserialize, Clone)]
pub struct ConnectorBankNames(pub HashMap<String, BanksVector>);

#[derive(Debug, Deserialize, Clone)]
pub struct BanksVector {
    #[serde(deserialize_with = "deserialize_hashset")]
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
    #[serde(deserialize_with = "deserialize_optional_hashset")]
    pub currency: Option<HashSet<api_models::enums::Currency>>,
    #[serde(deserialize_with = "deserialize_optional_hashset")]
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
    pub fields: HashMap<enums::Connector, RequiredFieldFinal>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RequiredFieldFinal {
    pub mandate: HashMap<String, RequiredFieldInfo>,
    pub non_mandate: HashMap<String, RequiredFieldInfo>,
    pub common: HashMap<String, RequiredFieldInfo>,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct Secrets {
    pub jwt_secret: Secret<String>,
    pub admin_api_key: Secret<String>,
    pub recon_admin_api_key: Secret<String>,
    pub master_enc_key: Secret<String>,
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
    pub vault_encryption_key: Secret<String>,
    pub rust_locker_encryption_key: Secret<String>,
    pub vault_private_key: Secret<String>,
    pub tunnel_private_key: Secret<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Proxy {
    pub http_url: Option<String>,
    pub https_url: Option<String>,
    pub idle_pool_connection_timeout: Option<u64>,
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
    pub password: Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub queue_strategy: QueueStrategy,
    pub min_idle: Option<u32>,
    pub max_lifetime: Option<u64>,
}

impl From<Database> for storage_impl::config::Database {
    fn from(val: Database) -> Self {
        Self {
            username: val.username,
            password: val.password,
            host: val.host,
            port: val.port,
            dbname: val.dbname,
            pool_size: val.pool_size,
            connection_timeout: val.connection_timeout,
            queue_strategy: val.queue_strategy,
            min_idle: val.min_idle,
            max_lifetime: val.max_lifetime,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct SupportedConnectors {
    pub wallets: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct Connectors {
    pub aci: ConnectorParams,
    #[cfg(feature = "payouts")]
    pub adyen: ConnectorParamsWithSecondaryBaseUrl,
    #[cfg(not(feature = "payouts"))]
    pub adyen: ConnectorParams,
    pub airwallex: ConnectorParams,
    pub applepay: ConnectorParams,
    pub authorizedotnet: ConnectorParams,
    pub bambora: ConnectorParams,
    pub bankofamerica: ConnectorParams,
    pub bitpay: ConnectorParams,
    pub bluesnap: ConnectorParamsWithSecondaryBaseUrl,
    pub boku: ConnectorParams,
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
    pub gocardless: ConnectorParams,
    pub helcim: ConnectorParams,
    pub iatapay: ConnectorParams,
    pub klarna: ConnectorParams,
    pub mollie: ConnectorParams,
    pub multisafepay: ConnectorParams,
    pub nexinets: ConnectorParams,
    pub nmi: ConnectorParams,
    pub noon: ConnectorParamsWithModeType,
    pub nuvei: ConnectorParams,
    pub opayo: ConnectorParams,
    pub opennode: ConnectorParams,
    pub payeezy: ConnectorParams,
    pub payme: ConnectorParams,
    pub paypal: ConnectorParams,
    pub payu: ConnectorParams,
    pub placetopay: ConnectorParams,
    pub powertranz: ConnectorParams,
    pub prophetpay: ConnectorParams,
    pub rapyd: ConnectorParams,
    pub riskified: ConnectorParams,
    pub shift4: ConnectorParams,
    pub signifyd: ConnectorParams,
    pub square: ConnectorParams,
    pub stax: ConnectorParams,
    pub stripe: ConnectorParamsWithFileUploadUrl,
    pub trustpay: ConnectorParamsWithMoreUrls,
    pub tsys: ConnectorParams,
    pub volt: ConnectorParams,
    pub wise: ConnectorParams,
    pub worldline: ConnectorParams,
    pub worldpay: ConnectorParams,
    pub zen: ConnectorParams,
}

#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParams {
    pub base_url: String,
    pub secondary_base_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithModeType {
    pub base_url: String,
    pub secondary_base_url: Option<String>,
    /// Can take values like Test or Live for Noon
    pub key_mode: String,
}

#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithMoreUrls {
    pub base_url: String,
    pub base_url_bank_redirects: String,
}

#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithFileUploadUrl {
    pub base_url: String,
    pub base_url_file_upload: String,
}

#[derive(Debug, Deserialize, Clone, Default, router_derive::ConfigValidate)]
#[serde(default)]
pub struct ConnectorParamsWithSecondaryBaseUrl {
    pub base_url: String,
    pub secondary_base_url: String,
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
    pub ignore_error: WebhookIgnoreErrorSettings,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct WebhookIgnoreErrorSettings {
    pub event_type: Option<bool>,
    pub payment_not_found: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ApiKeys {
    /// Hex-encoded 32-byte long (64 characters long when hex-encoded) key used for calculating
    /// hashes of API keys
    pub hash_key: Secret<String>,

    // Specifies the number of days before API key expiry when email reminders should be sent
    #[cfg(feature = "email")]
    pub expiry_reminder_days: Vec<u8>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DelayedSessionConfig {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connectors_with_delayed_session_response: HashSet<api_models::enums::Connector>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct WebhookSourceVerificationCall {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connectors_with_webhook_source_verification_call: HashSet<api_models::enums::Connector>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ApplePayDecryptConifg {
    pub apple_pay_ppc: Secret<String>,
    pub apple_pay_ppc_key: Secret<String>,
    pub apple_pay_merchant_cert: Secret<String>,
    pub apple_pay_merchant_cert_key: Secret<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ConnectorRequestReferenceIdConfig {
    pub merchant_ids_send_payment_id_as_connector_request_id: HashSet<String>,
}

impl Settings<SecuredSecret> {
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
            .add_source(File::from(config_path).required(false))
            .add_source(
                Environment::with_prefix("ROUTER")
                    .try_parsing(true)
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("log.telemetry.route_to_trace")
                    .with_list_parse_key("redis.cluster_urls")
                    .with_list_parse_key("events.kafka.brokers")
                    .with_list_parse_key("connectors.supported.wallets")
                    .with_list_parse_key("connector_request_reference_id_config.merchant_ids_send_payment_id_as_connector_request_id"),

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
        self.master_database.get_inner().validate()?;
        #[cfg(feature = "olap")]
        self.replica_database.get_inner().validate()?;
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
        self.secrets.get_inner().validate()?;
        self.locker.validate()?;
        self.connectors.validate("connectors")?;

        self.cors.validate()?;

        self.scheduler
            .as_ref()
            .map(|scheduler_settings| scheduler_settings.validate())
            .transpose()?;
        #[cfg(feature = "kv_store")]
        self.drainer.validate()?;
        self.api_keys.get_inner().validate()?;

        self.file_storage
            .validate()
            .map_err(|err| ApplicationError::InvalidConfigurationValueError(err.to_string()))?;

        self.lock_settings.validate()?;
        self.events.validate()?;

        self.encryption_management
            .validate()
            .map_err(|err| ApplicationError::InvalidConfigurationValueError(err.into()))?;

        self.secrets_management
            .validate()
            .map_err(|err| ApplicationError::InvalidConfigurationValueError(err.into()))?;
        Ok(())
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Payouts {
    pub payout_eligibility: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LockSettings {
    pub redis_lock_expiry_seconds: u32,
    pub delay_between_retries_in_milliseconds: u32,
    pub lock_retries: u32,
}

impl<'de> Deserialize<'de> for LockSettings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Inner {
            redis_lock_expiry_seconds: u32,
            delay_between_retries_in_milliseconds: u32,
        }

        let Inner {
            redis_lock_expiry_seconds,
            delay_between_retries_in_milliseconds,
        } = Inner::deserialize(deserializer)?;
        let redis_lock_expiry_milliseconds = redis_lock_expiry_seconds * 1000;
        Ok(Self {
            redis_lock_expiry_seconds,
            delay_between_retries_in_milliseconds,
            lock_retries: redis_lock_expiry_milliseconds / delay_between_retries_in_milliseconds,
        })
    }
}

#[cfg(feature = "olap")]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct ConnectorOnboarding {
    pub paypal: PayPalOnboarding,
}

#[cfg(feature = "olap")]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct PayPalOnboarding {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub partner_id: Secret<String>,
    pub enabled: bool,
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

#[cfg(test)]
mod hashset_deserialization_test {
    #![allow(clippy::unwrap_used)]
    use std::collections::HashSet;

    use serde::de::{
        value::{Error as ValueError, StrDeserializer},
        IntoDeserializer,
    };

    use super::deserialize_hashset;

    #[test]
    fn test_payment_method_hashset_deserializer() {
        use diesel_models::enums::PaymentMethod;

        let deserializer: StrDeserializer<'_, ValueError> = "wallet,card".into_deserializer();
        let payment_methods = deserialize_hashset::<'_, _, PaymentMethod>(deserializer);
        let expected_payment_methods = HashSet::from([PaymentMethod::Wallet, PaymentMethod::Card]);

        assert!(payment_methods.is_ok());
        assert_eq!(payment_methods.unwrap(), expected_payment_methods);
    }

    #[test]
    fn test_payment_method_hashset_deserializer_with_spaces() {
        use diesel_models::enums::PaymentMethod;

        let deserializer: StrDeserializer<'_, ValueError> =
            "wallet, card, bank_debit".into_deserializer();
        let payment_methods = deserialize_hashset::<'_, _, PaymentMethod>(deserializer);
        let expected_payment_methods = HashSet::from([
            PaymentMethod::Wallet,
            PaymentMethod::Card,
            PaymentMethod::BankDebit,
        ]);

        assert!(payment_methods.is_ok());
        assert_eq!(payment_methods.unwrap(), expected_payment_methods);
    }

    #[test]
    fn test_payment_method_hashset_deserializer_error() {
        use diesel_models::enums::PaymentMethod;

        let deserializer: StrDeserializer<'_, ValueError> =
            "wallet, card, unknown".into_deserializer();
        let payment_methods = deserialize_hashset::<'_, _, PaymentMethod>(deserializer);

        assert!(payment_methods.is_err());
    }
}
