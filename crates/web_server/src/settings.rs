

use std::sync::Arc;
use std::collections::HashMap;
use serde::Deserialize;
use tokio::sync::oneshot;


use common_utils::id_type;
use common_utils::types::theme;
use router_env::{tracing_actix_web::RequestId, Env, config as router_config};
use redis_interface;
use hyperswitch_interfaces::{secrets_interface::secret_state, encryption_interface, configs as interface_configs};
use external_services::{
    file_storage,
    grpc_client,
    managers::{encryption_management, secrets_management},
};
use storage_impl::{self, errors, config as storage_config};
use storage_impl::redis::RedisStore;
use storage_impl::redis::kv_store;
use storage_impl::{RouterStore};
use masking::{self, ExposeInterface};
use sample::{StorageInterface, GlobalStorageInterface, CommonStorageInterface, ApiClient};
use hyperswitch_domain_models::errors::StorageResult;

use crate::secret_handler;


#[derive(Clone)]
pub struct SessionState {
    pub store: Box<dyn StorageInterface<errors::StorageError>>,
//     /// Global store is used for global schema operations in tables like Users and Tenants
    pub global_store: Box<dyn GlobalStorageInterface<errors::StorageError>>,
//     pub conf: Arc<settings::Settings<RawSecret>>,
    pub api_client: Box<dyn ApiClient<State = SessionState>>,
    // pub event_handler: EventsHandler,
//     #[cfg(feature = "email")]
//     pub email_client: Arc<Box<dyn EmailService>>,
//     #[cfg(feature = "olap")]
//     pub pool: AnalyticsProvider,
    pub file_storage_client: Arc<dyn file_storage::FileStorageInterface>,
    pub request_id: Option<RequestId>,
    pub base_url: String,
//     pub tenant: Tenant,
//     #[cfg(feature = "olap")]
//     pub opensearch_client: Arc<OpenSearchClient>,
    pub grpc_client: Arc<grpc_client::GrpcClients>,
    pub theme_storage_client: Arc<dyn file_storage::FileStorageInterface>,
    pub locale: String,
}

// why this required? 
pub trait SessionStateInfo {
    // fn conf(&self) -> settings::Settings<RawSecret>;
    fn store(&self) -> Box<dyn StorageInterface<errors::StorageError>>;
    // fn event_handler(&self) -> EventsHandler;
    fn get_request_id(&self) -> Option<String>;
    fn add_request_id(&mut self, request_id: RequestId);
    // #[cfg(feature = "partial-auth")]
    // fn get_detached_auth(&self) -> RouterResult<(Blake3, &[u8])>;

    // this should be self, or else this should be removed
    fn session_state(&self) -> SessionState;
}

#[derive(Clone)]
pub struct AppState {
    pub flow_name: String,
    pub global_store: Box<dyn GlobalStorageInterface<errors::StorageError>>,
    pub stores: HashMap<id_type::TenantId, Box<dyn StorageInterface<errors::StorageError>>>,
    // pub conf: Arc<settings::Settings<RawSecret>>,
    // pub event_handler: EventsHandler,
    // #[cfg(feature = "email")]
    // pub email_client: Arc<Box<dyn EmailService>>,
    pub api_client: Box<dyn ApiClient<State = SessionState>>,
    // #[cfg(feature = "olap")]
    // pub pools: HashMap<id_type::TenantId, AnalyticsProvider>,
    // #[cfg(feature = "olap")]
    // pub opensearch_client: Arc<OpenSearchClient>,
    pub request_id: Option<RequestId>,
    pub file_storage_client: Arc<dyn file_storage::FileStorageInterface>,
    pub encryption_client: Arc<dyn encryption_interface::EncryptionManagementInterface>,
    pub grpc_client: Arc<grpc_client::GrpcClients>,
    pub theme_storage_client: Arc<dyn file_storage::FileStorageInterface>,
}

// why this required? 
pub trait AppStateInfo {
    // fn conf(&self) -> settings::Settings<RawSecret>;
    // fn event_handler(&self) -> EventsHandler;
    // #[cfg(feature = "email")]
    // fn email_client(&self) -> Arc<Box<dyn EmailService>>;
    fn add_request_id(&mut self, request_id: RequestId);
    fn add_flow_name(&mut self, flow_name: String);
    fn get_request_id(&self) -> Option<String>;
}

// #[derive(Debug, Deserialize, Clone, Default)]
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct Settings<S: secret_state::SecretState> {
    pub server: Server,
    pub proxy: Proxy,
    pub env: Env,
    pub master_database: secret_state::SecretStateContainer<Database, S>,
    #[cfg(feature = "olap")]
    pub replica_database: secret_state::SecretStateContainer<Database, S>,
    pub redis: redis_interface::RedisSettings,
    pub log: router_config::Log,
    pub secrets: secret_state::SecretStateContainer<Secrets, S>,
    pub locker: Locker,
    pub key_manager: secret_state::SecretStateContainer<KeyManagerConfig, S>,
    pub connectors: interface_configs::Connectors,
    pub forex_api: secret_state::SecretStateContainer<ForexApi, S>,
    // pub refund: Refund,
    // pub eph_key: EphemeralConfig,
    // pub scheduler: Option<SchedulerSettings>,
    // #[cfg(feature = "kv_store")]
    // pub drainer: DrainerSettings,
    // pub jwekey: SecretStateContainer<Jwekey, S>,
    // pub webhooks: WebhooksSettings,
    // pub pm_filters: ConnectorFilters,
    // pub bank_config: BankRedirectConfig,
    // pub api_keys: SecretStateContainer<ApiKeys, S>,
    pub file_storage: file_storage::FileStorageConfig,
    pub encryption_management: encryption_management::EncryptionManagementConfig,
    pub secrets_management: secrets_management::SecretsManagementConfig,
    // pub tokenization: TokenizationConfig,
    // pub connector_customer: ConnectorCustomer,
    // #[cfg(feature = "dummy_connector")]
    // pub dummy_connector: DummyConnector,
    // #[cfg(feature = "email")]
    // pub email: EmailSettings,
    // pub user: UserSettings,
    // pub cors: CorsSettings,
    // pub mandates: Mandates,
    // pub network_transaction_id_supported_connectors: NetworkTransactionIdSupportedConnectors,
    // pub required_fields: RequiredFields,
    // pub delayed_session_response: DelayedSessionConfig,
    // pub webhook_source_verification_call: WebhookSourceVerificationCall,
    // pub payment_method_auth: SecretStateContainer<PaymentMethodAuth, S>,
    // pub connector_request_reference_id_config: ConnectorRequestReferenceIdConfig,
    // #[cfg(feature = "payouts")]
    // pub payouts: Payouts,
    // pub payout_method_filters: ConnectorFilters,
    // pub applepay_decrypt_keys: SecretStateContainer<ApplePayDecryptConfig, S>,
    // pub paze_decrypt_keys: Option<SecretStateContainer<PazeDecryptConfig, S>>,
    // pub google_pay_decrypt_keys: Option<SecretStateContainer<GooglePayDecryptConfig, S>>,
    // pub multiple_api_version_supported_connectors: MultipleApiVersionSupportedConnectors,
    // pub applepay_merchant_configs: SecretStateContainer<ApplepayMerchantConfigs, S>,
    // pub lock_settings: LockSettings,
    // pub temp_locker_enable_config: TempLockerEnableConfig,
    // pub generic_link: GenericLink,
    // pub payment_link: PaymentLink,
    // #[cfg(feature = "olap")]
    // pub analytics: SecretStateContainer<AnalyticsConfig, S>,
    // #[cfg(feature = "kv_store")]
    // pub kv_config: KvConfig,
    // #[cfg(feature = "frm")]
    // pub frm: Frm,
    // #[cfg(feature = "olap")]
    // pub report_download_config: ReportConfig,
    // #[cfg(feature = "olap")]
    // pub opensearch: OpenSearchConfig,
    // pub events: EventsConfig,
    // #[cfg(feature = "olap")]
    // pub connector_onboarding: SecretStateContainer<ConnectorOnboarding, S>,
    // pub unmasked_headers: UnmaskedHeaders,
    pub multitenancy: Multitenancy,
    // pub saved_payment_methods: EligiblePaymentMethods,
    // pub user_auth_methods: SecretStateContainer<UserAuthMethodSettings, S>,
    // pub decision: Option<DecisionConfig>,
    // pub locker_based_open_banking_connectors: LockerBasedRecipientConnectorList,
    pub grpc_client: grpc_client::GrpcClientSettings,
    // #[cfg(feature = "v2")]
    // pub cell_information: CellInformation,
    // pub network_tokenization_supported_card_networks: NetworkTokenizationSupportedCardNetworks,
    // pub network_tokenization_service: Option<SecretStateContainer<NetworkTokenizationService, S>>,
    // pub network_tokenization_supported_connectors: NetworkTokenizationSupportedConnectors,
    pub theme: ThemeSettings,
    // pub platform: Platform,
}

#[derive(Debug, Deserialize, Clone)]
// #[serde(default)]
pub struct Server {
    pub port: u16,
    pub workers: usize,
    pub host: String,
    pub request_body_limit: usize,
    pub shutdown_timeout: u64,
    // #[cfg(feature = "tls")]
    // pub tls: Option<ServerTls>,
}

#[derive(Debug, Deserialize, Clone)]
// #[serde(default)]
pub struct Proxy {
    pub http_url: Option<String>,
    pub https_url: Option<String>,
    pub idle_pool_connection_timeout: Option<u64>,
    pub bypass_proxy_hosts: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
// #[serde(default)]
pub struct Database {
    pub username: String,
    pub password: masking::Secret<String>,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub queue_strategy: storage_config::QueueStrategy,
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

// why this is not from locker?
#[derive(Debug, Deserialize, Clone)]
// #[serde(default)]
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

// #[derive(Debug, Deserialize, Clone, Default)]
#[derive(Debug, Deserialize, Clone)]
pub enum DecryptionScheme {
    // #[default]
    #[serde(rename = "RSA-OAEP")]
    RsaOaep,
    #[serde(rename = "RSA-OAEP-256")]
    RsaOaep256,
}


// #[derive(Debug, Deserialize, Clone, Default)]
#[derive(Debug, Deserialize, Clone)]
// #[serde(default)]
pub struct ForexApi {
    pub api_key: masking::Secret<String>,
    pub fallback_api_key: masking::Secret<String>,
    /// in s
    pub call_delay: i64,
    /// in s
    pub redis_lock_timeout: u64,
}

// #[derive(Debug, Deserialize, Clone, Default)]
#[derive(Debug, Deserialize, Clone)]
// #[serde(default)]
pub struct KeyManagerConfig {
    pub enabled: bool,
    pub url: String,
    #[cfg(feature = "keymanager_mtls")]
    pub cert: masking::Secret<String>,
    #[cfg(feature = "keymanager_mtls")]
    pub ca: masking::Secret<String>,
}

// #[derive(Debug, Default, Deserialize, Clone)]
#[derive(Debug, Deserialize, Clone)]
// #[serde(default)]
pub struct Secrets {
    pub jwt_secret: masking::Secret<String>,
    pub admin_api_key: masking::Secret<String>,
    pub master_enc_key: masking::Secret<String>,
}

// #[derive(Debug, Deserialize, Clone, Default)]
#[derive(Debug, Deserialize, Clone)]
pub struct ThemeSettings {
    pub storage:file_storage::FileStorageConfig,
    pub email_config: theme::EmailThemeConfig,
}


// #[derive(Debug, Clone, Default)]
#[derive(Debug, Clone)]
pub struct TenantConfig(pub HashMap<id_type::TenantId, Tenant>);

impl<'de> Deserialize<'de> for TenantConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Inner {
            base_url: String,
            schema: String,
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

// #[derive(Debug, Clone, Default, Deserialize)]
#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Deserialize, Clone)]
pub struct GlobalTenant {
    // #[serde(default = "id_type::TenantId::get_default_global_tenant_id")]
    pub tenant_id: id_type::TenantId,
    pub schema: String,
    pub redis_key_prefix: String,
    pub clickhouse_database: String,
}

impl storage_impl::config::TenantConfig for GlobalTenant {
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

#[derive(Debug, Clone)]
pub struct Tenant {
    pub tenant_id: id_type::TenantId,
    pub base_url: String,
    pub schema: String,
    pub redis_key_prefix: String,
    pub clickhouse_database: String,
    pub user: TenantUserConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TenantUserConfig {
    pub control_center_url: String,
}


pub type SettingsRaw = Settings<secret_state::RawSecret>;

    /// # Panics
    ///
    /// Panics if Store can't be created or JWE decryption fails
impl AppState {

    pub async fn with_storage(
        conf: Settings<secret_state::SecuredSecret>,
        storage_impl: sample::StorageImpl,
        shut_down_signal: oneshot::Sender<()>,
        api_client: Box<dyn ApiClient<State = SessionState>>, // why this should be session-state?
    ) -> Self {
        #[allow(clippy::expect_used)]
        let secret_management_client = conf
            .secrets_management
            .get_secret_management_client()
            .await
            .expect("Failed to create secret management client");

        let conf = Box::pin(secret_handler::fetch_raw_secrets(
            conf,
            &*secret_management_client,
        ))
        .await;

        #[allow(clippy::expect_used)]
        let encryption_client = conf
            .encryption_management
            .get_encryption_management_client()
            .await
            .expect("Failed to create encryption client");

        Box::pin(async move {
            let testable = storage_impl == sample::StorageImpl::PostgresqlTest;
            // #[allow(clippy::expect_used)]
            // let event_handler = conf
            //     .events
            //     .get_event_handler()
            //     .await
            //     .expect("Failed to create event handler");

            // #[allow(clippy::expect_used)]
            // #[cfg(feature = "olap")]
            // let opensearch_client = Arc::new(
            //     conf.opensearch
            //         .get_opensearch_client()
            //         .await
            //         .expect("Failed to create opensearch client"),
            // );

            // #[cfg(feature = "olap")]
            // let mut pools: HashMap<id_type::TenantId, AnalyticsProvider> = HashMap::new();
            let mut stores = HashMap::new();
            #[allow(clippy::expect_used)]
            let cache_store = get_cache_store(&conf.clone(), shut_down_signal, testable)
                .await
                .expect("Failed to create store");
            let global_store: Box<dyn GlobalStorageInterface<errors::StorageError>> = Self::get_store_interface(
                &storage_impl,
                // &event_handler,
                &conf,
                &conf.multitenancy.global_tenant,
                Arc::clone(&cache_store),
                testable,
            )
            .await
            .get_global_storage_interface();
            // for (tenant_name, tenant) in conf.clone().multitenancy.get_tenants() {
            //     let store: Box<dyn StorageInterface> = Self::get_store_interface(
            //         &storage_impl,
            //         &event_handler,
            //         &conf,
            //         tenant,
            //         Arc::clone(&cache_store),
            //         testable,
            //     )
            //     .await
            //     .get_storage_interface();
            //     stores.insert(tenant_name.clone(), store);
            //     #[cfg(feature = "olap")]
            //     let pool = AnalyticsProvider::from_conf(conf.analytics.get_inner(), tenant).await;
            //     #[cfg(feature = "olap")]
            //     pools.insert(tenant_name.clone(), pool);
            // }

            // #[cfg(feature = "email")]
            // let email_client = Arc::new(create_email_client(&conf).await);

            let file_storage_client = conf.file_storage.get_file_storage_client().await;
            let theme_storage_client = conf.theme.storage.get_file_storage_client().await;

            let grpc_client = conf.grpc_client.get_grpc_client_interface().await;

            Self {
                flow_name: String::from("default"),
                stores,
                global_store,
                // conf: Arc::new(conf),
                // #[cfg(feature = "email")]
                // email_client,
                api_client,
                // event_handler,
                // #[cfg(feature = "olap")]
                // pools,
                // #[cfg(feature = "olap")]
                // opensearch_client,
                request_id: None,
                file_storage_client,
                encryption_client,
                grpc_client,
                theme_storage_client,
            }
        })
        .await
    }

    async fn get_store_interface(
        storage_impl: &sample::StorageImpl,
        // event_handler: &EventsHandler,
        conf: &SettingsRaw,
        tenant: &dyn storage_config::TenantConfig,
        cache_store: Arc<RedisStore>,
        testable: bool,
    ) -> Box<dyn sample::CommonStorageInterface<errors::StorageError>> {
        Box::new(
            #[allow(clippy::expect_used)]
            get_store(conf, tenant, Arc::clone(&cache_store), testable)
                .await
                .expect("Failed to create store"),
        )
        // match storage_impl {
        //     sample::StorageImpl::Postgresql | sample::StorageImpl::PostgresqlTest => match event_handler {
        //         EventsHandler::Kafka(kafka_client) => Box::new(
        //             KafkaStore::new(
        //                 #[allow(clippy::expect_used)]
        //                 get_store(&conf.clone(), tenant, Arc::clone(&cache_store), testable)
        //                     .await
        //                     .expect("Failed to create store"),
        //                 kafka_client.clone(),
        //                 TenantID(tenant.get_schema().to_string()),
        //                 tenant,
        //             )
        //             .await,
        //         ),
        //         EventsHandler::Logs(_) => Box::new(
        //             #[allow(clippy::expect_used)]
        //             get_store(conf, tenant, Arc::clone(&cache_store), testable)
        //                 .await
        //                 .expect("Failed to create store"),
        //         ),
        //     },
        //     #[allow(clippy::expect_used)]
        //     sample::StorageImpl::Mock => Box::new(
        //         MockDb::new(&conf.redis)
        //             .await
        //             .expect("Failed to create mock store"),
        //     ),
        // }
    }

    pub async fn new(
        conf: Settings<secret_state::SecuredSecret>,
        shut_down_signal: oneshot::Sender<()>,
        api_client: Box<dyn ApiClient<State = SessionState>>,
    ) -> Self {
        Box::pin(Self::with_storage(
            conf,
            sample::StorageImpl::Postgresql,
            shut_down_signal,
            api_client,
        ))
        .await
    }

    // pub fn get_session_state<E, F>(
    //     self: Arc<Self>,
    //     tenant: &id_type::TenantId,
    //     locale: Option<String>,
    //     err: F,
    // ) -> Result<SessionState, E>
    // where
    //     F: FnOnce() -> E + Copy,
    // {
    //     let tenant_conf = self.conf.multitenancy.get_tenant(tenant).ok_or_else(err)?;
    //     let mut event_handler = self.event_handler.clone();
    //     event_handler.add_tenant(tenant_conf);
    //     Ok(SessionState {
    //         store: self.stores.get(tenant).ok_or_else(err)?.clone(),
    //         global_store: self.global_store.clone(),
    //         conf: Arc::clone(&self.conf),
    //         api_client: self.api_client.clone(),
    //         event_handler,
    //         #[cfg(feature = "olap")]
    //         pool: self.pools.get(tenant).ok_or_else(err)?.clone(),
    //         file_storage_client: self.file_storage_client.clone(),
    //         request_id: self.request_id,
    //         base_url: tenant_conf.base_url.clone(),
    //         tenant: tenant_conf.clone(),
    //         #[cfg(feature = "email")]
    //         email_client: Arc::clone(&self.email_client),
    //         #[cfg(feature = "olap")]
    //         opensearch_client: Arc::clone(&self.opensearch_client),
    //         grpc_client: Arc::clone(&self.grpc_client),
    //         theme_storage_client: self.theme_storage_client.clone(),
    //         locale: locale.unwrap_or(common_utils::consts::DEFAULT_LOCALE.to_string()),
    //     })
    // }
}

#[cfg(not(feature = "olap"))]
pub type StoreType = storage_impl::database::store::Store;
#[cfg(feature = "olap")]
pub type StoreType = storage_impl::database::store::ReplicaStore;

#[cfg(not(feature = "kv_store"))]
pub type Store = RouterStore<StoreType>;
#[cfg(feature = "kv_store")]
pub type Store = KVRouterStore<StoreType>;

// #[async_trait::async_trait]
// impl scheduler::SchedulerInterface for Store {}

/// # Panics
///
/// Will panic if hex decode of master key fails
#[allow(clippy::expect_used)]
pub async fn get_store(
    config: &SettingsRaw,
    tenant: &dyn storage_config::TenantConfig,
    cache_store: Arc<RedisStore>,
    test_transaction: bool,
) -> StorageResult<Store> {
    let master_config = config.master_database.clone().into_inner();

    #[cfg(feature = "olap")]
    let replica_config = config.replica_database.clone().into_inner();

    #[allow(clippy::expect_used)]
    let master_enc_key = hex::decode(config.secrets.get_inner().master_enc_key.clone().expose())
        .map(masking::StrongSecret::new)
        .expect("Failed to decode master key from hex");

    #[cfg(not(feature = "olap"))]
    let conf = master_config.into();
    #[cfg(feature = "olap")]
    // this would get abstracted, for all cases
    #[allow(clippy::useless_conversion)]
    let conf = (master_config.into(), replica_config.into());

    let store: storage_impl::RouterStore<StoreType> = if test_transaction {
        storage_impl::RouterStore::test_store(conf, tenant, &config.redis, master_enc_key).await?
    } else {
        storage_impl::RouterStore::from_config(
            conf,
            tenant,
            master_enc_key,
            cache_store,
            storage_impl::redis::cache::IMC_INVALIDATION_CHANNEL,
        )
        .await?
    };

    #[cfg(feature = "kv_store")]
    let store = KVRouterStore::from_store(
        store,
        config.drainer.stream_name.clone(),
        config.drainer.num_partitions,
        config.kv_config.ttl,
        config.kv_config.soft_kill,
    );

    Ok(store)
}

#[allow(clippy::expect_used)]
pub async fn get_cache_store(
    config: &SettingsRaw,
    shut_down_signal: oneshot::Sender<()>,
    _test_transaction: bool,
) -> StorageResult<Arc<RedisStore>> {
    storage_impl::RouterStore::<StoreType>::cache_store(&config.redis, shut_down_signal).await
}

// impl sample::address::AddressInterface for Store {}
// impl sample::user::UserInterface for Store {}