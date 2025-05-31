use std::{collections::HashMap, sync::Arc};

// use actix_web::{web, Scope};
// #[cfg(all(feature = "olap", feature = "v1"))]
// use api_models::routing::RoutingRetrieveQuery;
// #[cfg(feature = "olap")]
// use common_enums::TransactionType;
// #[cfg(feature = "partial-auth")]
use common_utils::crypto::Blake3;
use common_utils::id_type;
// #[cfg(feature = "email")]
// use external_services::email::{
//     no_email::NoEmailClient, ses::AwsSes, smtp::SmtpServer, EmailClientConfigs, EmailService,
// };
use external_services::{
    file_storage::FileStorageInterface,
    grpc_client::{GrpcClients, GrpcHeaders},
};
use hyperswitch_interfaces::{
    crm::CrmInterface,
    encryption_interface::EncryptionManagementInterface,
    secrets_interface::secret_state::{RawSecret, SecuredSecret},
};
use router_env::tracing_actix_web::RequestId;
use scheduler::SchedulerInterface;
use storage_impl::{config::TenantConfig, redis::RedisStore, MockDb};
use tokio::sync::oneshot;

use settings::Tenant;
// #[cfg(any(feature = "olap", feature = "oltp"))]
// use super::currency;
// #[cfg(feature = "dummy_connector")]
// use super::dummy_connector::*;
// #[cfg(all(any(feature = "v1", feature = "v2"), feature = "oltp"))]
// use super::ephemeral_key::*;
// #[cfg(any(feature = "olap", feature = "oltp"))]
// use super::payment_methods;
// #[cfg(feature = "payouts")]
// use super::payout_link::*;
// #[cfg(feature = "payouts")]
// use super::payouts::*;
// #[cfg(all(
//     feature = "oltp",
//     any(feature = "v1", feature = "v2"),
//     not(feature = "customer_v2")
// ))]
// use super::pm_auth;
// #[cfg(feature = "oltp")]
// use super::poll;
// #[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
// use super::proxy;
// #[cfg(all(feature = "v2", feature = "revenue_recovery", feature = "oltp"))]
// use super::recovery_webhooks::*;
// #[cfg(all(feature = "oltp", feature = "v2"))]
// use super::refunds;
// #[cfg(feature = "olap")]
// use super::routing;
// #[cfg(all(feature = "oltp", feature = "v2"))]
// use super::tokenization as tokenization_routes;
// #[cfg(all(feature = "olap", feature = "v1"))]
// use super::verification::{apple_pay_merchant_registration, retrieve_apple_pay_verified_domains};
// #[cfg(feature = "oltp")]
// use super::webhooks::*;
// use super::{
//     admin, api_keys, cache::*, connector_onboarding, disputes, files, gsm, health::*, profiles,
//     relay, user, user_role,
// };
// #[cfg(feature = "v1")]
// use super::{apple_pay_certificates_migration, blocklist, payment_link, webhook_events};
// #[cfg(any(feature = "olap", feature = "oltp"))]
// use super::{configs::*, customers, payments};
// #[cfg(all(any(feature = "olap", feature = "oltp"), feature = "v1"))]
// use super::{mandates::*, refunds::*};
// #[cfg(feature = "olap")]
pub use crate::analytics::opensearch::OpenSearchClient;
// #[cfg(feature = "olap")]
use crate::{analytics::AnalyticsProvider};
// #[cfg(feature = "partial-auth")]
use crate::errors::RouterResult;
// #[cfg(feature = "v1")]
// use crate::routes::cards_info::{
//     card_iin_info, create_cards_info, migrate_cards_info, update_cards_info,
// };
// #[cfg(all(feature = "olap", feature = "v1"))]
// use crate::routes::feature_matrix;
// #[cfg(all(feature = "frm", feature = "oltp"))]
// use crate::routes::fraud_check as frm_routes;
// #[cfg(all(feature = "recon", feature = "olap"))]
// use crate::routes::recon as recon_routes;
pub use crate::{
    configs::settings,
    db::{
        AccountsStorageInterface, CommonStorageInterface, GlobalStorageInterface, StorageImpl,
        StorageInterface,
    },
    events::EventsHandler,
};
use crate::{
    configs::{secrets_transformers, Settings},
    db::kafka_store::{KafkaStore, TenantID},
};

#[derive(Clone)]
pub struct ReqState {
    pub event_context: events::EventContext<crate::events::EventType, EventsHandler>,
}

#[derive(Clone)]
pub struct SessionState {
    pub store: Box<dyn StorageInterface>,
    /// Global store is used for global schema operations in tables like Users and Tenants
    pub global_store: Box<dyn GlobalStorageInterface>,
    pub accounts_store: Box<dyn AccountsStorageInterface>,
    pub conf: Arc<settings::Settings<RawSecret>>,
    pub api_client: Box<dyn crate::services::ApiClient>,
    pub event_handler: EventsHandler,
    #[cfg(feature = "email")]
    pub email_client: Arc<Box<dyn EmailService>>,
    #[cfg(feature = "olap")]
    pub pool: AnalyticsProvider,
    pub file_storage_client: Arc<dyn FileStorageInterface>,
    pub request_id: Option<RequestId>,
    pub base_url: String,
    pub tenant: Tenant,
    #[cfg(feature = "olap")]
    pub opensearch_client: Option<Arc<OpenSearchClient>>,
    pub grpc_client: Arc<GrpcClients>,
    pub theme_storage_client: Arc<dyn FileStorageInterface>,
    pub locale: String,
    pub crm_client: Arc<dyn CrmInterface>,
}
impl scheduler::SchedulerSessionState for SessionState {
    fn get_db(&self) -> Box<dyn SchedulerInterface> {
        self.store.get_scheduler_db()
    }
}
impl SessionState {
    pub fn get_req_state(&self) -> ReqState {
        ReqState {
            event_context: events::EventContext::new(self.event_handler.clone()),
        }
    }
    pub fn get_grpc_headers(&self) -> GrpcHeaders {
        GrpcHeaders {
            tenant_id: self.tenant.tenant_id.get_string_repr().to_string(),
            request_id: self.request_id.map(|req_id| (*req_id).to_string()),
        }
    }
}

pub trait SessionStateInfo {
    fn conf(&self) -> settings::Settings<RawSecret>;
    fn store(&self) -> Box<dyn StorageInterface>;
    fn event_handler(&self) -> EventsHandler;
    fn get_request_id(&self) -> Option<String>;
    fn add_request_id(&mut self, request_id: RequestId);
    #[cfg(feature = "partial-auth")]
    fn get_detached_auth(&self) -> RouterResult<(Blake3, &[u8])>;
    fn session_state(&self) -> SessionState;
    fn global_store(&self) -> Box<dyn GlobalStorageInterface>;
}

impl SessionStateInfo for SessionState {
    fn store(&self) -> Box<dyn StorageInterface> {
        self.store.to_owned()
    }
    fn conf(&self) -> settings::Settings<RawSecret> {
        self.conf.as_ref().to_owned()
    }
    fn event_handler(&self) -> EventsHandler {
        self.event_handler.clone()
    }
    fn get_request_id(&self) -> Option<String> {
        self.api_client.get_request_id()
    }
    fn add_request_id(&mut self, request_id: RequestId) {
        self.api_client.add_request_id(request_id);
        self.store.add_request_id(request_id.to_string());
        self.request_id.replace(request_id);
    }

    #[cfg(feature = "partial-auth")]
    fn get_detached_auth(&self) -> RouterResult<(Blake3, &[u8])> {
        use error_stack::ResultExt;
        use hyperswitch_domain_models::errors::api_error_response as errors;
        use masking::prelude::PeekInterface as _;
        use router_env::logger;

        let output = CHECKSUM_KEY.get_or_try_init(|| {
            let conf = self.conf();
            let context = conf
                .api_keys
                .get_inner()
                .checksum_auth_context
                .peek()
                .clone();
            let key = conf.api_keys.get_inner().checksum_auth_key.peek();
            hex::decode(key).map(|key| {
                (
                    masking::StrongSecret::new(context),
                    masking::StrongSecret::new(key),
                )
            })
        });

        match output {
            Ok((context, key)) => Ok((Blake3::new(context.peek().clone()), key.peek())),
            Err(err) => {
                logger::error!("Failed to get checksum key");
                Err(err).change_context(errors::ApiErrorResponse::InternalServerError)
            }
        }
    }
    fn session_state(&self) -> SessionState {
        self.clone()
    }
    fn global_store(&self) -> Box<(dyn GlobalStorageInterface)> {
        self.global_store.to_owned()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub flow_name: String,
    pub global_store: Box<dyn GlobalStorageInterface>,
    // TODO: use a separate schema for accounts_store
    pub accounts_store: HashMap<id_type::TenantId, Box<dyn AccountsStorageInterface>>,
    pub stores: HashMap<id_type::TenantId, Box<dyn StorageInterface>>,
    pub conf: Arc<settings::Settings<RawSecret>>,
    pub event_handler: EventsHandler,
    #[cfg(feature = "email")]
    pub email_client: Arc<Box<dyn EmailService>>,
    pub api_client: Box<dyn crate::services::ApiClient>,
    #[cfg(feature = "olap")]
    pub pools: HashMap<id_type::TenantId, AnalyticsProvider>,
    #[cfg(feature = "olap")]
    pub opensearch_client: Option<Arc<OpenSearchClient>>,
    pub request_id: Option<RequestId>,
    pub file_storage_client: Arc<dyn FileStorageInterface>,
    pub encryption_client: Arc<dyn EncryptionManagementInterface>,
    pub grpc_client: Arc<GrpcClients>,
    pub theme_storage_client: Arc<dyn FileStorageInterface>,
    pub crm_client: Arc<dyn CrmInterface>,
}
impl scheduler::SchedulerAppState for AppState {
    fn get_tenants(&self) -> Vec<id_type::TenantId> {
        self.conf.multitenancy.get_tenant_ids()
    }
}
pub trait AppStateInfo {
    fn conf(&self) -> settings::Settings<RawSecret>;
    fn event_handler(&self) -> EventsHandler;
    #[cfg(feature = "email")]
    fn email_client(&self) -> Arc<Box<dyn EmailService>>;
    fn add_request_id(&mut self, request_id: RequestId);
    fn add_flow_name(&mut self, flow_name: String);
    fn get_request_id(&self) -> Option<String>;
}

#[cfg(feature = "partial-auth")]
static CHECKSUM_KEY: once_cell::sync::OnceCell<(
    masking::StrongSecret<String>,
    masking::StrongSecret<Vec<u8>>,
)> = once_cell::sync::OnceCell::new();

impl AppStateInfo for AppState {
    fn conf(&self) -> settings::Settings<RawSecret> {
        self.conf.as_ref().to_owned()
    }
    #[cfg(feature = "email")]
    fn email_client(&self) -> Arc<Box<dyn EmailService>> {
        self.email_client.to_owned()
    }
    fn event_handler(&self) -> EventsHandler {
        self.event_handler.clone()
    }
    fn add_request_id(&mut self, request_id: RequestId) {
        self.api_client.add_request_id(request_id);
        self.request_id.replace(request_id);
    }

    fn add_flow_name(&mut self, flow_name: String) {
        self.api_client.add_flow_name(flow_name);
    }
    fn get_request_id(&self) -> Option<String> {
        self.api_client.get_request_id()
    }
}

impl AsRef<Self> for AppState {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AppState {
    /// # Panics
    ///
    /// Panics if Store can't be created or JWE decryption fails
    pub async fn with_storage(
        conf: settings::Settings<SecuredSecret>,
        storage_impl: StorageImpl,
        shut_down_signal: oneshot::Sender<()>,
        api_client: Box<dyn crate::services::ApiClient>,
    ) -> Self {
        #[allow(clippy::expect_used)]
        let secret_management_client = conf
            .secrets_management
            .get_secret_management_client()
            .await
            .expect("Failed to create secret management client");

        let conf = Box::pin(secrets_transformers::fetch_raw_secrets(
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
            let testable = storage_impl == StorageImpl::PostgresqlTest;
            #[allow(clippy::expect_used)]
            let event_handler = conf
                .events
                .get_event_handler()
                .await
                .expect("Failed to create event handler");

            #[allow(clippy::expect_used)]
            #[cfg(feature = "olap")]
            let opensearch_client = conf
                .opensearch
                .get_opensearch_client()
                .await
                .expect("Failed to initialize OpenSearch client.")
                .map(Arc::new);

            #[allow(clippy::expect_used)]
            let cache_store = crate::services::get_cache_store(&conf.clone(), shut_down_signal, testable)
                .await
                .expect("Failed to create store");
            let global_store: Box<dyn GlobalStorageInterface> = Self::get_store_interface(
                &storage_impl,
                &event_handler,
                &conf,
                &conf.multitenancy.global_tenant,
                Arc::clone(&cache_store),
                testable,
            )
            .await
            .get_global_storage_interface();
            #[cfg(feature = "olap")]
            let pools = conf
                .multitenancy
                .tenants
                .get_pools_map(conf.analytics.get_inner())
                .await;
            let stores = conf
                .multitenancy
                .tenants
                .get_store_interface_map(&storage_impl, &conf, Arc::clone(&cache_store), testable)
                .await;
            let accounts_store = conf
                .multitenancy
                .tenants
                .get_accounts_store_interface_map(
                    &storage_impl,
                    &conf,
                    Arc::clone(&cache_store),
                    testable,
                )
                .await;

            #[cfg(feature = "email")]
            let email_client = Arc::new(create_email_client(&conf).await);

            let file_storage_client = conf.file_storage.get_file_storage_client().await;
            let theme_storage_client = conf.theme.storage.get_file_storage_client().await;
            let crm_client = conf.crm.get_crm_client().await;

            let grpc_client = conf.grpc_client.get_grpc_client_interface().await;

            Self {
                flow_name: String::from("default"),
                stores,
                global_store,
                accounts_store,
                conf: Arc::new(conf),
                #[cfg(feature = "email")]
                email_client,
                api_client,
                event_handler,
                #[cfg(feature = "olap")]
                pools,
                #[cfg(feature = "olap")]
                opensearch_client,
                request_id: None,
                file_storage_client,
                encryption_client,
                grpc_client,
                theme_storage_client,
                crm_client,
            }
        })
        .await
    }

    /// # Panics
    ///
    /// Panics if Failed to create store
    pub async fn get_store_interface(
        storage_impl: &StorageImpl,
        event_handler: &EventsHandler,
        conf: &Settings,
        tenant: &dyn TenantConfig,
        cache_store: Arc<RedisStore>,
        testable: bool,
    ) -> Box<dyn CommonStorageInterface> {
        match storage_impl {
            StorageImpl::Postgresql | StorageImpl::PostgresqlTest => match event_handler {
                EventsHandler::Kafka(kafka_client) => Box::new(
                    KafkaStore::new(
                        #[allow(clippy::expect_used)]
                        crate::services::get_store(&conf.clone(), tenant, Arc::clone(&cache_store), testable)
                            .await
                            .expect("Failed to create store"),
                        kafka_client.clone(),
                        TenantID(tenant.get_tenant_id().get_string_repr().to_owned()),
                        tenant,
                    )
                    .await,
                ),
                EventsHandler::Logs(_) => Box::new(
                    #[allow(clippy::expect_used)]
                    crate::services::get_store(conf, tenant, Arc::clone(&cache_store), testable)
                        .await
                        .expect("Failed to create store"),
                ),
            },
            #[allow(clippy::expect_used)]
            StorageImpl::Mock => Box::new(
                MockDb::new(&conf.redis)
                    .await
                    .expect("Failed to create mock store"),
            ),
        }
    }

    pub async fn new(
        conf: settings::Settings<SecuredSecret>,
        shut_down_signal: oneshot::Sender<()>,
        api_client: Box<dyn crate::services::ApiClient>,
    ) -> Self {
        Box::pin(Self::with_storage(
            conf,
            StorageImpl::Postgresql,
            shut_down_signal,
            api_client,
        ))
        .await
    }

    pub fn get_session_state<E, F>(
        self: Arc<Self>,
        tenant: &id_type::TenantId,
        locale: Option<String>,
        err: F,
    ) -> Result<SessionState, E>
    where
        F: FnOnce() -> E + Copy,
    {
        let tenant_conf = self.conf.multitenancy.get_tenant(tenant).ok_or_else(err)?;
        let mut event_handler = self.event_handler.clone();
        event_handler.add_tenant(tenant_conf);
        let store = self.stores.get(tenant).ok_or_else(err)?.clone();
        Ok(SessionState {
            store,
            global_store: self.global_store.clone(),
            accounts_store: self.accounts_store.get(tenant).ok_or_else(err)?.clone(),
            conf: Arc::clone(&self.conf),
            api_client: self.api_client.clone(),
            event_handler,
            #[cfg(feature = "olap")]
            pool: self.pools.get(tenant).ok_or_else(err)?.clone(),
            file_storage_client: self.file_storage_client.clone(),
            request_id: self.request_id,
            base_url: tenant_conf.base_url.clone(),
            tenant: tenant_conf.clone(),
            #[cfg(feature = "email")]
            email_client: Arc::clone(&self.email_client),
            #[cfg(feature = "olap")]
            opensearch_client: self.opensearch_client.clone(),
            grpc_client: Arc::clone(&self.grpc_client),
            theme_storage_client: self.theme_storage_client.clone(),
            locale: locale.unwrap_or(common_utils::consts::DEFAULT_LOCALE.to_string()),
            crm_client: self.crm_client.clone(),
        })
    }
}

