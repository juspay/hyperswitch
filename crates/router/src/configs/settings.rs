use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

#[cfg(feature = "olap")]
use analytics::{opensearch::OpenSearchConfig, ReportConfig};
use api_models::{enums, payment_methods::RequiredFieldInfo};
use common_utils::{ext_traits::ConfigExt, id_type, types::theme::EmailThemeConfig};
use config::{Environment, File};
use error_stack::ResultExt;
#[cfg(feature = "email")]
use external_services::email::EmailSettings;
use external_services::{
    file_storage::FileStorageConfig,
    grpc_client::GrpcClientSettings,
    managers::{
        encryption_management::EncryptionManagementConfig,
        secrets_management::SecretsManagementConfig,
    },
};
pub use hyperswitch_interfaces::configs::Connectors;
use hyperswitch_interfaces::secrets_interface::secret_state::{
    RawSecret, SecretState, SecretStateContainer, SecuredSecret,
};
use masking::Secret;
use redis_interface::RedisSettings;
pub use router_env::config::{Log, LogConsole, LogFile, LogTelemetry};
use rust_decimal::Decimal;
use scheduler::SchedulerSettings;
use serde::Deserialize;
use storage_impl::config::QueueStrategy;

#[cfg(feature = "olap")]
use crate::analytics::{AnalyticsConfig, AnalyticsProvider};
use crate::{
    configs,
    core::errors::{ApplicationError, ApplicationResult},
    env::{self, Env},
    events::EventsConfig,
    routes::app,
    AppState,
};

pub const REQUIRED_FIELDS_CONFIG_FILE: &str = "payment_required_fields_v2.toml";

#[derive(clap::Parser, Default)]
#[cfg_attr(feature = "vergen", command(version = router_env::version!()))]
pub struct CmdLineConf {
    /// Config file.
    /// Application will look for "config/config.toml" if this option isn't specified.
    #[arg(short = 'f', long, value_name = "FILE")]
    pub config_path: Option<PathBuf>,
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
    pub key_manager: SecretStateContainer<KeyManagerConfig, S>,
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
    pub user: UserSettings,
    pub cors: CorsSettings,
    pub mandates: Mandates,
    pub network_transaction_id_supported_connectors: NetworkTransactionIdSupportedConnectors,
    pub required_fields: RequiredFields,
    pub delayed_session_response: DelayedSessionConfig,
    pub webhook_source_verification_call: WebhookSourceVerificationCall,
    pub payment_method_auth: SecretStateContainer<PaymentMethodAuth, S>,
    pub connector_request_reference_id_config: ConnectorRequestReferenceIdConfig,
    #[cfg(feature = "payouts")]
    pub payouts: Payouts,
    pub payout_method_filters: ConnectorFilters,
    pub applepay_decrypt_keys: SecretStateContainer<ApplePayDecryptConfig, S>,
    pub paze_decrypt_keys: Option<SecretStateContainer<PazeDecryptConfig, S>>,
    pub google_pay_decrypt_keys: Option<GooglePayDecryptConfig>,
    pub multiple_api_version_supported_connectors: MultipleApiVersionSupportedConnectors,
    pub applepay_merchant_configs: SecretStateContainer<ApplepayMerchantConfigs, S>,
    pub lock_settings: LockSettings,
    pub temp_locker_enable_config: TempLockerEnableConfig,
    pub generic_link: GenericLink,
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
    pub opensearch: OpenSearchConfig,
    pub events: EventsConfig,
    #[cfg(feature = "olap")]
    pub connector_onboarding: SecretStateContainer<ConnectorOnboarding, S>,
    pub unmasked_headers: UnmaskedHeaders,
    pub multitenancy: Multitenancy,
    pub saved_payment_methods: EligiblePaymentMethods,
    pub user_auth_methods: SecretStateContainer<UserAuthMethodSettings, S>,
    pub decision: Option<DecisionConfig>,
    pub locker_based_open_banking_connectors: LockerBasedRecipientConnectorList,
    pub grpc_client: GrpcClientSettings,
    #[cfg(feature = "v2")]
    pub cell_information: CellInformation,
    pub network_tokenization_supported_card_networks: NetworkTokenizationSupportedCardNetworks,
    pub network_tokenization_service: Option<SecretStateContainer<NetworkTokenizationService, S>>,
    pub network_tokenization_supported_connectors: NetworkTokenizationSupportedConnectors,
    pub theme: ThemeSettings,
    pub platform: Platform,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Platform {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Multitenancy {
    pub tenants: TenantConfig,
    pub enabled: bool,
    pub global_tenant: GlobalTenant,
}

impl Multitenancy {
    pub fn get_tenants(&self) -> &HashMap<id_type::TenantId, Tenant> {
        &self.tenants.0
    }
    pub fn get_tenant_ids(&self) -> Vec<id_type::TenantId> {
        self.tenants
            .0
            .values()
            .map(|tenant| tenant.tenant_id.clone())
            .collect()
    }
    pub fn get_tenant(&self, tenant_id: &id_type::TenantId) -> Option<&Tenant> {
        self.tenants.0.get(tenant_id)
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DecisionConfig {
    pub base_url: String,
}

#[derive(Debug, Clone, Default)]
pub struct TenantConfig(pub HashMap<id_type::TenantId, Tenant>);

impl TenantConfig {
    /// # Panics
    ///
    /// Panics if Failed to create event handler
    pub async fn get_store_interface_map(
        &self,
        storage_impl: &app::StorageImpl,
        conf: &configs::Settings,
        cache_store: Arc<storage_impl::redis::RedisStore>,
        testable: bool,
    ) -> HashMap<id_type::TenantId, Box<dyn app::StorageInterface>> {
        #[allow(clippy::expect_used)]
        let event_handler = conf
            .events
            .get_event_handler()
            .await
            .expect("Failed to create event handler");
        futures::future::join_all(self.0.iter().map(|(tenant_name, tenant)| async {
            let store = AppState::get_store_interface(
                storage_impl,
                &event_handler,
                conf,
                tenant,
                cache_store.clone(),
                testable,
            )
            .await
            .get_storage_interface();
            (tenant_name.clone(), store)
        }))
        .await
        .into_iter()
        .collect()
    }
    /// # Panics
    ///
    /// Panics if Failed to create event handler
    pub async fn get_accounts_store_interface_map(
        &self,
        storage_impl: &app::StorageImpl,
        conf: &configs::Settings,
        cache_store: Arc<storage_impl::redis::RedisStore>,
        testable: bool,
    ) -> HashMap<id_type::TenantId, Box<dyn app::AccountsStorageInterface>> {
        #[allow(clippy::expect_used)]
        let event_handler = conf
            .events
            .get_event_handler()
            .await
            .expect("Failed to create event handler");
        futures::future::join_all(self.0.iter().map(|(tenant_name, tenant)| async {
            let store = AppState::get_store_interface(
                storage_impl,
                &event_handler,
                conf,
                tenant,
                cache_store.clone(),
                testable,
            )
            .await
            .get_accounts_storage_interface();
            (tenant_name.clone(), store)
        }))
        .await
        .into_iter()
        .collect()
    }
    #[cfg(feature = "olap")]
    pub async fn get_pools_map(
        &self,
        analytics_config: &AnalyticsConfig,
    ) -> HashMap<id_type::TenantId, AnalyticsProvider> {
        futures::future::join_all(self.0.iter().map(|(tenant_name, tenant)| async {
            (
                tenant_name.clone(),
                AnalyticsProvider::from_conf(analytics_config, tenant).await,
            )
        }))
        .await
        .into_iter()
        .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Tenant {
    pub tenant_id: id_type::TenantId,
    pub base_url: String,
    pub schema: String,
    pub accounts_schema: String,
    pub redis_key_prefix: String,
    pub clickhouse_database: String,
    pub user: TenantUserConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TenantUserConfig {
    pub control_center_url: String,
}

impl storage_impl::config::TenantConfig for Tenant {
    fn get_tenant_id(&self) -> &id_type::TenantId {
        &self.tenant_id
    }
    fn get_accounts_schema(&self) -> &str {
        self.accounts_schema.as_str()
    }
    fn get_schema(&self) -> &str {
        self.schema.as_str()
    }
    fn get_redis_key_prefix(&self) -> &str {
        self.redis_key_prefix.as_str()
    }
    fn get_clickhouse_database(&self) -> &str {
        self.clickhouse_database.as_str()
    }
}

// Todo: Global tenant should not be part of tenant config(https://github.com/juspay/hyperswitch/issues/7237)
#[derive(Debug, Deserialize, Clone)]
pub struct GlobalTenant {
    #[serde(default = "id_type::TenantId::get_default_global_tenant_id")]
    pub tenant_id: id_type::TenantId,
    pub schema: String,
    pub redis_key_prefix: String,
    pub clickhouse_database: String,
}
// Todo: Global tenant should not be part of tenant config
impl storage_impl::config::TenantConfig for GlobalTenant {
    fn get_tenant_id(&self) -> &id_type::TenantId {
        &self.tenant_id
    }
    fn get_accounts_schema(&self) -> &str {
        self.schema.as_str()
    }
    fn get_schema(&self) -> &str {
        self.schema.as_str()
    }
    fn get_redis_key_prefix(&self) -> &str {
        self.redis_key_prefix.as_str()
    }
    fn get_clickhouse_database(&self) -> &str {
        self.clickhouse_database.as_str()
    }
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
    pub soft_kill: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct KeyManagerConfig {
    pub enabled: bool,
    pub url: String,
    #[cfg(feature = "keymanager_mtls")]
    pub cert: Secret<String>,
    #[cfg(feature = "keymanager_mtls")]
    pub ca: Secret<String>,
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
pub struct PaymentLink {
    pub sdk_url: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ForexApi {
    pub api_key: Secret<String>,
    pub fallback_api_key: Secret<String>,
    /// in s
    pub call_delay: i64,
    /// in s
    pub redis_lock_timeout: u64,
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
    pub supported_connectors: HashSet<enums::Connector>,
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
    pub connector_list: HashSet<enums::Connector>,
    #[cfg(feature = "payouts")]
    #[serde(deserialize_with = "deserialize_hashset")]
    pub payout_connector_list: HashSet<enums::PayoutConnectors>,
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

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NetworkTransactionIdSupportedConnectors {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connector_list: HashSet<enums::Connector>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NetworkTokenizationSupportedCardNetworks {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub card_networks: HashSet<enums::CardNetwork>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkTokenizationService {
    pub generate_token_url: url::Url,
    pub fetch_token_url: url::Url,
    pub token_service_api_key: Secret<String>,
    pub public_key: Secret<String>,
    pub private_key: Secret<String>,
    pub key_id: String,
    pub delete_token_url: url::Url,
    pub check_token_status_url: url::Url,
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

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "v2", derive(Default))] // Configs are read from the config file in config/payout_required_fields.toml
pub struct PayoutRequiredFields(pub HashMap<enums::PaymentMethod, PaymentMethodType>);

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "v2", derive(Default))] // Configs are read from the config file in config/payment_required_fields.toml
pub struct RequiredFields(pub HashMap<enums::PaymentMethod, PaymentMethodType>);

#[derive(Debug, Deserialize, Clone)]
pub struct PaymentMethodType(pub HashMap<enums::PaymentMethodType, ConnectorFields>);

#[derive(Debug, Deserialize, Clone)]
pub struct ConnectorFields {
    pub fields: HashMap<enums::Connector, RequiredFieldFinal>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct Secrets {
    pub jwt_secret: Secret<String>,
    pub admin_api_key: Secret<String>,
    pub master_enc_key: Secret<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UserSettings {
    pub password_validity_in_days: u16,
    pub two_factor_auth_expiry_in_secs: i64,
    pub totp_issuer_name: String,
    pub base_url: String,
    pub force_two_factor_auth: bool,
    pub force_cookies: bool,
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
    pub bypass_proxy_hosts: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Server {
    pub port: u16,
    pub workers: usize,
    pub host: String,
    pub request_body_limit: usize,
    pub shutdown_timeout: u64,
    #[cfg(feature = "tls")]
    pub tls: Option<ServerTls>,
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

    #[cfg(feature = "partial-auth")]
    pub checksum_auth_context: Secret<String>,

    #[cfg(feature = "partial-auth")]
    pub checksum_auth_key: Secret<String>,

    #[cfg(feature = "partial-auth")]
    pub enable_partial_auth: bool,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DelayedSessionConfig {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connectors_with_delayed_session_response: HashSet<enums::Connector>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct WebhookSourceVerificationCall {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connectors_with_webhook_source_verification_call: HashSet<enums::Connector>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ApplePayDecryptConfig {
    pub apple_pay_ppc: Secret<String>,
    pub apple_pay_ppc_key: Secret<String>,
    pub apple_pay_merchant_cert: Secret<String>,
    pub apple_pay_merchant_cert_key: Secret<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PazeDecryptConfig {
    pub paze_private_key: Secret<String>,
    pub paze_private_key_passphrase: Secret<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GooglePayDecryptConfig {
    pub google_pay_root_signing_keys: Secret<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct LockerBasedRecipientConnectorList {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connector_list: HashSet<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ConnectorRequestReferenceIdConfig {
    pub merchant_ids_send_payment_id_as_connector_request_id: HashSet<id_type::MerchantId>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct UserAuthMethodSettings {
    pub encryption_key: Secret<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NetworkTokenizationSupportedConnectors {
    #[serde(deserialize_with = "deserialize_hashset")]
    pub connector_list: HashSet<enums::Connector>,
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

        let config = router_env::Config::builder(&environment.to_string())
            .change_context(ApplicationError::ConfigurationError)?
            .add_source(File::from(config_path).required(false));

        #[cfg(feature = "v2")]
        let config = {
            let required_fields_config_file =
                router_env::Config::get_config_directory().join(REQUIRED_FIELDS_CONFIG_FILE);
            config.add_source(File::from(required_fields_config_file).required(false))
        };

        let config = config
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
            .build()
            .change_context(ApplicationError::ConfigurationError)?;

        serde_path_to_error::deserialize(config)
            .attach_printable("Unable to deserialize application configuration")
            .change_context(ApplicationError::ConfigurationError)
    }

    pub fn validate(&self) -> ApplicationResult<()> {
        self.server.validate()?;
        self.master_database.get_inner().validate()?;
        #[cfg(feature = "olap")]
        self.replica_database.get_inner().validate()?;

        // The logger may not yet be initialized when validating the application configuration
        #[allow(clippy::print_stderr)]
        self.redis.validate().map_err(|error| {
            eprintln!("{error}");
            ApplicationError::InvalidConfigurationValueError("Redis configuration".into())
        })?;

        if self.log.file.enabled {
            if self.log.file.file_name.is_default_or_empty() {
                return Err(error_stack::Report::from(
                    ApplicationError::InvalidConfigurationValueError(
                        "log file name must not be empty".into(),
                    ),
                ));
            }

            if self.log.file.path.is_default_or_empty() {
                return Err(error_stack::Report::from(
                    ApplicationError::InvalidConfigurationValueError(
                        "log directory path must not be empty".into(),
                    ),
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

        #[cfg(feature = "olap")]
        self.opensearch.validate()?;

        self.encryption_management
            .validate()
            .map_err(|err| ApplicationError::InvalidConfigurationValueError(err.into()))?;

        self.secrets_management
            .validate()
            .map_err(|err| ApplicationError::InvalidConfigurationValueError(err.into()))?;
        self.generic_link.payment_method_collect.validate()?;
        self.generic_link.payout_link.validate()?;

        #[cfg(feature = "v2")]
        self.cell_information.validate()?;
        self.network_tokenization_service
            .as_ref()
            .map(|x| x.get_inner().validate())
            .transpose()?;

        self.paze_decrypt_keys
            .as_ref()
            .map(|x| x.get_inner().validate())
            .transpose()?;

        self.google_pay_decrypt_keys
            .as_ref()
            .map(|x| x.validate())
            .transpose()?;

        self.key_manager.get_inner().validate()?;
        #[cfg(feature = "email")]
        self.email
            .validate()
            .map_err(|err| ApplicationError::InvalidConfigurationValueError(err.into()))?;

        self.theme
            .storage
            .validate()
            .map_err(|err| ApplicationError::InvalidConfigurationValueError(err.to_string()))?;

        Ok(())
    }
}

impl Settings<RawSecret> {
    #[cfg(feature = "kv_store")]
    pub fn is_kv_soft_kill_mode(&self) -> bool {
        self.kv_config.soft_kill.unwrap_or(false)
    }

    #[cfg(not(feature = "kv_store"))]
    pub fn is_kv_soft_kill_mode(&self) -> bool {
        false
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Payouts {
    pub payout_eligibility: bool,
    #[serde(default)]
    pub required_fields: PayoutRequiredFields,
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

#[cfg(feature = "tls")]
#[derive(Debug, Deserialize, Clone)]
pub struct ServerTls {
    /// Port to host the TLS secure server on
    pub port: u16,
    /// Use a different host (optional) (defaults to the host provided in [`Server`] config)
    pub host: Option<String>,
    /// private key file path associated with TLS (path to the private key file (`pem` format))
    pub private_key: PathBuf,
    /// certificate file associated with TLS (path to the certificate file (`pem` format))
    pub certificate: PathBuf,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CellInformation {
    pub id: id_type::CellId,
}

#[cfg(feature = "v2")]
impl Default for CellInformation {
    fn default() -> Self {
        // We provide a static default cell id for constructing application settings.
        // This will only panic at application startup if we're unable to construct the default,
        // around the time of deserializing application settings.
        // And a panic at application startup is considered acceptable.
        #[allow(clippy::expect_used)]
        let cell_id =
            id_type::CellId::from_string("defid").expect("Failed to create a default for Cell Id");
        Self { id: cell_id }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ThemeSettings {
    pub storage: FileStorageConfig,
    pub email_config: EmailThemeConfig,
}

fn deserialize_hashmap_inner<K, V>(
    value: HashMap<String, String>,
) -> Result<HashMap<K, HashSet<V>>, String>
where
    K: Eq + std::str::FromStr + std::hash::Hash,
    V: Eq + std::str::FromStr + std::hash::Hash,
    <K as std::str::FromStr>::Err: std::fmt::Display,
    <V as std::str::FromStr>::Err: std::fmt::Display,
{
    let (values, errors) = value
        .into_iter()
        .map(
            |(k, v)| match (K::from_str(k.trim()), deserialize_hashset_inner(v)) {
                (Err(error), _) => Err(format!(
                    "Unable to deserialize `{}` as `{}`: {error}",
                    k,
                    std::any::type_name::<K>()
                )),
                (_, Err(error)) => Err(error),
                (Ok(key), Ok(value)) => Ok((key, value)),
            },
        )
        .fold(
            (HashMap::new(), Vec::new()),
            |(mut values, mut errors), result| match result {
                Ok((key, value)) => {
                    values.insert(key, value);
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

fn deserialize_hashmap<'a, D, K, V>(deserializer: D) -> Result<HashMap<K, HashSet<V>>, D::Error>
where
    D: serde::Deserializer<'a>,
    K: Eq + std::str::FromStr + std::hash::Hash,
    V: Eq + std::str::FromStr + std::hash::Hash,
    <K as std::str::FromStr>::Err: std::fmt::Display,
    <V as std::str::FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;
    deserialize_hashmap_inner(<HashMap<String, String>>::deserialize(deserializer)?)
        .map_err(D::Error::custom)
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

impl<'de> Deserialize<'de> for TenantConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Inner {
            base_url: String,
            schema: String,
            accounts_schema: String,
            redis_key_prefix: String,
            clickhouse_database: String,
            user: TenantUserConfig,
        }

        let hashmap = <HashMap<id_type::TenantId, Inner>>::deserialize(deserializer)?;

        Ok(Self(
            hashmap
                .into_iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        Tenant {
                            tenant_id: key,
                            base_url: value.base_url,
                            schema: value.schema,
                            accounts_schema: value.accounts_schema,
                            redis_key_prefix: value.redis_key_prefix,
                            clickhouse_database: value.clickhouse_database,
                            user: value.user,
                        },
                    )
                })
                .collect(),
        ))
    }
}

#[cfg(test)]
mod hashmap_deserialization_test {
    #![allow(clippy::unwrap_used)]
    use std::collections::{HashMap, HashSet};

    use serde::de::{
        value::{Error as ValueError, MapDeserializer},
        IntoDeserializer,
    };

    use super::deserialize_hashmap;

    #[test]
    fn test_payment_method_and_payment_method_types() {
        use diesel_models::enums::{PaymentMethod, PaymentMethodType};

        let input_map: HashMap<String, String> = HashMap::from([
            ("bank_transfer".to_string(), "ach,bacs".to_string()),
            ("wallet".to_string(), "paypal,venmo".to_string()),
        ]);
        let deserializer: MapDeserializer<
            '_,
            std::collections::hash_map::IntoIter<String, String>,
            ValueError,
        > = input_map.into_deserializer();
        let result = deserialize_hashmap::<'_, _, PaymentMethod, PaymentMethodType>(deserializer);
        let expected_result = HashMap::from([
            (
                PaymentMethod::BankTransfer,
                HashSet::from([PaymentMethodType::Ach, PaymentMethodType::Bacs]),
            ),
            (
                PaymentMethod::Wallet,
                HashSet::from([PaymentMethodType::Paypal, PaymentMethodType::Venmo]),
            ),
        ]);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_result);
    }

    #[test]
    fn test_payment_method_and_payment_method_types_with_spaces() {
        use diesel_models::enums::{PaymentMethod, PaymentMethodType};

        let input_map: HashMap<String, String> = HashMap::from([
            (" bank_transfer ".to_string(), " ach , bacs ".to_string()),
            ("wallet ".to_string(), " paypal , pix , venmo ".to_string()),
        ]);
        let deserializer: MapDeserializer<
            '_,
            std::collections::hash_map::IntoIter<String, String>,
            ValueError,
        > = input_map.into_deserializer();
        let result = deserialize_hashmap::<'_, _, PaymentMethod, PaymentMethodType>(deserializer);
        let expected_result = HashMap::from([
            (
                PaymentMethod::BankTransfer,
                HashSet::from([PaymentMethodType::Ach, PaymentMethodType::Bacs]),
            ),
            (
                PaymentMethod::Wallet,
                HashSet::from([
                    PaymentMethodType::Paypal,
                    PaymentMethodType::Pix,
                    PaymentMethodType::Venmo,
                ]),
            ),
        ]);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_result);
    }

    #[test]
    fn test_payment_method_deserializer_error() {
        use diesel_models::enums::{PaymentMethod, PaymentMethodType};

        let input_map: HashMap<String, String> = HashMap::from([
            ("unknown".to_string(), "ach,bacs".to_string()),
            ("wallet".to_string(), "paypal,unknown".to_string()),
        ]);
        let deserializer: MapDeserializer<
            '_,
            std::collections::hash_map::IntoIter<String, String>,
            ValueError,
        > = input_map.into_deserializer();
        let result = deserialize_hashmap::<'_, _, PaymentMethod, PaymentMethodType>(deserializer);

        assert!(result.is_err());
    }
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
