use std::{collections::HashMap, sync::Arc};

use actix_web::{web, Scope};
#[cfg(all(feature = "olap", feature = "v1"))]
use api_models::routing::RoutingRetrieveQuery;
#[cfg(feature = "olap")]
use common_enums::TransactionType;
#[cfg(feature = "partial-auth")]
use common_utils::crypto::Blake3;
use common_utils::id_type;
#[cfg(feature = "email")]
use external_services::email::{
    no_email::NoEmailClient, ses::AwsSes, smtp::SmtpServer, EmailClientConfigs, EmailService,
};
use external_services::{
    file_storage::FileStorageInterface,
    grpc_client::{GrpcClients, GrpcHeaders},
};
use hyperswitch_interfaces::{
    encryption_interface::EncryptionManagementInterface,
    secrets_interface::secret_state::{RawSecret, SecuredSecret},
};
use router_env::tracing_actix_web::RequestId;
use scheduler::SchedulerInterface;
use storage_impl::{config::TenantConfig, redis::RedisStore, MockDb};
use tokio::sync::oneshot;

use self::settings::Tenant;
#[cfg(any(feature = "olap", feature = "oltp"))]
use super::currency;
#[cfg(feature = "dummy_connector")]
use super::dummy_connector::*;
#[cfg(all(any(feature = "v1", feature = "v2"), feature = "oltp"))]
use super::ephemeral_key::*;
#[cfg(any(feature = "olap", feature = "oltp"))]
use super::payment_methods;
#[cfg(feature = "payouts")]
use super::payout_link::*;
#[cfg(feature = "payouts")]
use super::payouts::*;
#[cfg(all(
    feature = "oltp",
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2")
))]
use super::pm_auth;
#[cfg(feature = "oltp")]
use super::poll;
#[cfg(feature = "olap")]
use super::routing;
#[cfg(all(feature = "olap", feature = "v1"))]
use super::verification::{apple_pay_merchant_registration, retrieve_apple_pay_verified_domains};
#[cfg(feature = "oltp")]
use super::webhooks::*;
use super::{
    admin, api_keys, cache::*, connector_onboarding, disputes, files, gsm, health::*, profiles,
    relay, user, user_role,
};
#[cfg(feature = "v1")]
use super::{apple_pay_certificates_migration, blocklist, payment_link, webhook_events};
#[cfg(any(feature = "olap", feature = "oltp"))]
use super::{configs::*, customers, payments};
#[cfg(all(any(feature = "olap", feature = "oltp"), feature = "v1"))]
use super::{mandates::*, refunds::*};
#[cfg(feature = "olap")]
pub use crate::analytics::opensearch::OpenSearchClient;
#[cfg(feature = "olap")]
use crate::analytics::AnalyticsProvider;
#[cfg(feature = "partial-auth")]
use crate::errors::RouterResult;
#[cfg(feature = "v1")]
use crate::routes::cards_info::card_iin_info;
#[cfg(all(feature = "olap", feature = "v1"))]
use crate::routes::feature_matrix;
#[cfg(all(feature = "frm", feature = "oltp"))]
use crate::routes::fraud_check as frm_routes;
#[cfg(all(feature = "recon", feature = "olap"))]
use crate::routes::recon as recon_routes;
pub use crate::{
    configs::settings,
    db::{
        AccountsStorageInterface, CommonStorageInterface, GlobalStorageInterface, StorageImpl,
        StorageInterface,
    },
    events::EventsHandler,
    services::{get_cache_store, get_store},
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
    pub opensearch_client: Arc<OpenSearchClient>,
    pub grpc_client: Arc<GrpcClients>,
    pub theme_storage_client: Arc<dyn FileStorageInterface>,
    pub locale: String,
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
    pub opensearch_client: Arc<OpenSearchClient>,
    pub request_id: Option<RequestId>,
    pub file_storage_client: Arc<dyn FileStorageInterface>,
    pub encryption_client: Arc<dyn EncryptionManagementInterface>,
    pub grpc_client: Arc<GrpcClients>,
    pub theme_storage_client: Arc<dyn FileStorageInterface>,
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

#[cfg(feature = "email")]
pub async fn create_email_client(
    settings: &settings::Settings<RawSecret>,
) -> Box<dyn EmailService> {
    match &settings.email.client_config {
        EmailClientConfigs::Ses { aws_ses } => Box::new(
            AwsSes::create(
                &settings.email,
                aws_ses,
                settings.proxy.https_url.to_owned(),
            )
            .await,
        ),
        EmailClientConfigs::Smtp { smtp } => {
            Box::new(SmtpServer::create(&settings.email, smtp.clone()).await)
        }
        EmailClientConfigs::NoEmailClient => Box::new(NoEmailClient::create().await),
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
            let opensearch_client = Arc::new(
                conf.opensearch
                    .get_opensearch_client()
                    .await
                    .expect("Failed to create opensearch client"),
            );

            #[allow(clippy::expect_used)]
            let cache_store = get_cache_store(&conf.clone(), shut_down_signal, testable)
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
                        get_store(&conf.clone(), tenant, Arc::clone(&cache_store), testable)
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
                    get_store(conf, tenant, Arc::clone(&cache_store), testable)
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
        Ok(SessionState {
            store: self.stores.get(tenant).ok_or_else(err)?.clone(),
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
            opensearch_client: Arc::clone(&self.opensearch_client),
            grpc_client: Arc::clone(&self.grpc_client),
            theme_storage_client: self.theme_storage_client.clone(),
            locale: locale.unwrap_or(common_utils::consts::DEFAULT_LOCALE.to_string()),
        })
    }
}

pub struct Health;

impl Health {
    pub fn server(state: AppState) -> Scope {
        web::scope("health")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::get().to(health)))
            .service(web::resource("/ready").route(web::get().to(deep_health_check)))
    }
}

#[cfg(feature = "dummy_connector")]
pub struct DummyConnector;

#[cfg(all(feature = "dummy_connector", feature = "v1"))]
impl DummyConnector {
    pub fn server(state: AppState) -> Scope {
        let mut routes_with_restricted_access = web::scope("");
        #[cfg(not(feature = "external_access_dc"))]
        {
            routes_with_restricted_access =
                routes_with_restricted_access.guard(actix_web::guard::Host("localhost"));
        }
        routes_with_restricted_access = routes_with_restricted_access
            .service(web::resource("/payment").route(web::post().to(dummy_connector_payment)))
            .service(
                web::resource("/payments/{payment_id}")
                    .route(web::get().to(dummy_connector_payment_data)),
            )
            .service(
                web::resource("/{payment_id}/refund").route(web::post().to(dummy_connector_refund)),
            )
            .service(
                web::resource("/refunds/{refund_id}")
                    .route(web::get().to(dummy_connector_refund_data)),
            );
        web::scope("/dummy-connector")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/authorize/{attempt_id}")
                    .route(web::get().to(dummy_connector_authorize_payment)),
            )
            .service(
                web::resource("/complete/{attempt_id}")
                    .route(web::get().to(dummy_connector_complete_payment)),
            )
            .service(routes_with_restricted_access)
    }
}

pub struct Payments;

#[cfg(all(
    any(feature = "olap", feature = "oltp"),
    feature = "v2",
    feature = "payment_methods_v2",
))]
impl Payments {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/v2/payments").app_data(web::Data::new(state));
        route = route
            .service(
                web::resource("")
                    .route(web::post().to(payments::payments_create_and_confirm_intent)),
            )
            .service(
                web::resource("/create-intent")
                    .route(web::post().to(payments::payments_create_intent)),
            )
            .service(
                web::resource("/aggregate").route(web::get().to(payments::get_payments_aggregates)),
            )
            .service(
                web::resource("/profile/aggregate")
                    .route(web::get().to(payments::get_payments_aggregates_profile)),
            );

        route =
            route
                .service(web::resource("/ref/{merchant_reference_id}").route(
                    web::get().to(payments::payment_get_intent_using_merchant_reference_id),
                ));

        route = route.service(
            web::scope("/{payment_id}")
                .service(
                    web::resource("/confirm-intent")
                        .route(web::post().to(payments::payment_confirm_intent)),
                )
                .service(
                    web::resource("/get-intent")
                        .route(web::get().to(payments::payments_get_intent)),
                )
                .service(
                    web::resource("/update-intent")
                        .route(web::put().to(payments::payments_update_intent)),
                )
                .service(
                    web::resource("/create-external-sdk-tokens")
                        .route(web::post().to(payments::payments_connector_session)),
                )
                .service(web::resource("").route(web::get().to(payments::payment_status)))
                .service(
                    web::resource("/start-redirection")
                        .route(web::get().to(payments::payments_start_redirection)),
                )
                .service(
                    web::resource("/payment-methods")
                        .route(web::get().to(payments::list_payment_methods)),
                )
                .service(
                    web::resource("/finish-redirection/{publishable_key}/{profile_id}")
                        .route(web::get().to(payments::payments_finish_redirection)),
                )
                .service(
                    web::resource("/capture").route(web::post().to(payments::payments_capture)),
                ),
        );

        route
    }
}

pub struct Relay;

#[cfg(feature = "oltp")]
impl Relay {
    pub fn server(state: AppState) -> Scope {
        web::scope("/relay")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(relay::relay)))
            .service(web::resource("/{relay_id}").route(web::get().to(relay::relay_retrieve)))
    }
}

#[cfg(feature = "v1")]
impl Payments {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/payments").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            route = route
                .service(
                    web::resource("/list")
                        .route(web::get().to(payments::payments_list))
                        .route(web::post().to(payments::payments_list_by_filter)),
                )
                .service(
                    web::resource("/profile/list")
                        .route(web::get().to(payments::profile_payments_list))
                        .route(web::post().to(payments::profile_payments_list_by_filter)),
                )
                .service(
                    web::resource("/filter")
                        .route(web::post().to(payments::get_filters_for_payments)),
                )
                .service(
                    web::resource("/v2/filter").route(web::get().to(payments::get_payment_filters)),
                )
                .service(
                    web::resource("/aggregate")
                        .route(web::get().to(payments::get_payments_aggregates)),
                )
                .service(
                    web::resource("/profile/aggregate")
                        .route(web::get().to(payments::get_payments_aggregates_profile)),
                )
                .service(
                    web::resource("/v2/profile/filter")
                        .route(web::get().to(payments::get_payment_filters_profile)),
                )
                .service(
                    web::resource("/{payment_id}/manual-update")
                        .route(web::put().to(payments::payments_manual_update)),
                )
        }
        #[cfg(feature = "oltp")]
        {
            route = route
                .service(web::resource("").route(web::post().to(payments::payments_create)))
                .service(
                    web::resource("/session_tokens")
                        .route(web::post().to(payments::payments_connector_session)),
                )
                .service(
                    web::resource("/sync")
                        .route(web::post().to(payments::payments_retrieve_with_gateway_creds)),
                )
                .service(
                    web::resource("/{payment_id}")
                        .route(web::get().to(payments::payments_retrieve))
                        .route(web::post().to(payments::payments_update)),
                )
                .service(
                    web::resource("/{payment_id}/post_session_tokens").route(web::post().to(payments::payments_post_session_tokens)),
                )
                .service(
                    web::resource("/{payment_id}/confirm").route(web::post().to(payments::payments_confirm)),
                )
                .service(
                    web::resource("/{payment_id}/cancel").route(web::post().to(payments::payments_cancel)),
                )
                .service(
                    web::resource("/{payment_id}/capture").route(web::post().to(payments::payments_capture)),
                )
                .service(
                    web::resource("/{payment_id}/approve")
                        .route(web::post().to(payments::payments_approve)),
                )
                .service(
                    web::resource("/{payment_id}/reject")
                        .route(web::post().to(payments::payments_reject)),
                )
                .service(
                    web::resource("/redirect/{payment_id}/{merchant_id}/{attempt_id}")
                        .route(web::get().to(payments::payments_start)),
                )
                .service(
                    web::resource(
                        "/{payment_id}/{merchant_id}/redirect/response/{connector}/{creds_identifier}",
                    )
                    .route(web::get().to(payments::payments_redirect_response_with_creds_identifier)),
                )
                .service(
                    web::resource("/{payment_id}/{merchant_id}/redirect/response/{connector}")
                        .route(web::get().to(payments::payments_redirect_response))
                        .route(web::post().to(payments::payments_redirect_response))
                )
                .service(
                    web::resource("/{payment_id}/{merchant_id}/redirect/complete/{connector}")
                        .route(web::get().to(payments::payments_complete_authorize_redirect))
                        .route(web::post().to(payments::payments_complete_authorize_redirect)),
                )
                .service(
                    web::resource("/{payment_id}/complete_authorize")
                        .route(web::post().to(payments::payments_complete_authorize)),
                )
                .service(
                    web::resource("/{payment_id}/incremental_authorization").route(web::post().to(payments::payments_incremental_authorization)),
                )
                .service(
                    web::resource("/{payment_id}/{merchant_id}/authorize/{connector}").route(web::post().to(payments::post_3ds_payments_authorize)),
                )
                .service(
                    web::resource("/{payment_id}/3ds/authentication").route(web::post().to(payments::payments_external_authentication)),
                )
                .service(
                    web::resource("/{payment_id}/extended_card_info").route(web::get().to(payments::retrieve_extended_card_info)),
                )
                .service(
                web::resource("{payment_id}/calculate_tax")
                    .route(web::post().to(payments::payments_dynamic_tax_calculation)),
                );
        }
        route
    }
}

#[cfg(any(feature = "olap", feature = "oltp"))]
pub struct Forex;

#[cfg(all(any(feature = "olap", feature = "oltp"), feature = "v1"))]
impl Forex {
    pub fn server(state: AppState) -> Scope {
        web::scope("/forex")
            .app_data(web::Data::new(state.clone()))
            .app_data(web::Data::new(state.clone()))
            .service(web::resource("/rates").route(web::get().to(currency::retrieve_forex)))
            .service(
                web::resource("/convert_from_minor").route(web::get().to(currency::convert_forex)),
            )
    }
}

#[cfg(feature = "olap")]
pub struct Routing;

#[cfg(all(feature = "olap", feature = "v2"))]
impl Routing {
    pub fn server(state: AppState) -> Scope {
        web::scope("/v2/routing-algorithm")
            .app_data(web::Data::new(state.clone()))
            .service(
                web::resource("").route(web::post().to(|state, req, payload| {
                    routing::routing_create_config(state, req, payload, TransactionType::Payment)
                })),
            )
            .service(
                web::resource("/{algorithm_id}")
                    .route(web::get().to(routing::routing_retrieve_config)),
            )
    }
}
#[cfg(all(feature = "olap", feature = "v1"))]
impl Routing {
    pub fn server(state: AppState) -> Scope {
        #[allow(unused_mut)]
        let mut route = web::scope("/routing")
            .app_data(web::Data::new(state.clone()))
            .service(
                web::resource("/active").route(web::get().to(|state, req, query_params| {
                    routing::routing_retrieve_linked_config(
                        state,
                        req,
                        query_params,
                        &TransactionType::Payment,
                    )
                })),
            )
            .service(
                web::resource("")
                    .route(
                        web::get().to(|state, req, path: web::Query<RoutingRetrieveQuery>| {
                            routing::list_routing_configs(
                                state,
                                req,
                                path,
                                &TransactionType::Payment,
                            )
                        }),
                    )
                    .route(web::post().to(|state, req, payload| {
                        routing::routing_create_config(
                            state,
                            req,
                            payload,
                            TransactionType::Payment,
                        )
                    })),
            )
            .service(web::resource("/list/profile").route(web::get().to(
                |state, req, query: web::Query<RoutingRetrieveQuery>| {
                    routing::list_routing_configs_for_profile(
                        state,
                        req,
                        query,
                        &TransactionType::Payment,
                    )
                },
            )))
            .service(
                web::resource("/default").route(web::post().to(|state, req, payload| {
                    routing::routing_update_default_config(
                        state,
                        req,
                        payload,
                        &TransactionType::Payment,
                    )
                })),
            )
            .service(
                web::resource("/deactivate").route(web::post().to(|state, req, payload| {
                    routing::routing_unlink_config(state, req, payload, &TransactionType::Payment)
                })),
            )
            .service(
                web::resource("/decision")
                    .route(web::put().to(routing::upsert_decision_manager_config))
                    .route(web::get().to(routing::retrieve_decision_manager_config))
                    .route(web::delete().to(routing::delete_decision_manager_config)),
            )
            .service(
                web::resource("/decision/surcharge")
                    .route(web::put().to(routing::upsert_surcharge_decision_manager_config))
                    .route(web::get().to(routing::retrieve_surcharge_decision_manager_config))
                    .route(web::delete().to(routing::delete_surcharge_decision_manager_config)),
            )
            .service(
                web::resource("/default/profile/{profile_id}").route(web::post().to(
                    |state, req, path, payload| {
                        routing::routing_update_default_config_for_profile(
                            state,
                            req,
                            path,
                            payload,
                            &TransactionType::Payment,
                        )
                    },
                )),
            )
            .service(
                web::resource("/default/profile").route(web::get().to(|state, req| {
                    routing::routing_retrieve_default_config(state, req, &TransactionType::Payment)
                })),
            );

        #[cfg(feature = "payouts")]
        {
            route = route
                .service(
                    web::resource("/payouts")
                        .route(web::get().to(
                            |state, req, path: web::Query<RoutingRetrieveQuery>| {
                                routing::list_routing_configs(
                                    state,
                                    req,
                                    path,
                                    &TransactionType::Payout,
                                )
                            },
                        ))
                        .route(web::post().to(|state, req, payload| {
                            routing::routing_create_config(
                                state,
                                req,
                                payload,
                                TransactionType::Payout,
                            )
                        })),
                )
                .service(web::resource("/payouts/list/profile").route(web::get().to(
                    |state, req, query: web::Query<RoutingRetrieveQuery>| {
                        routing::list_routing_configs_for_profile(
                            state,
                            req,
                            query,
                            &TransactionType::Payout,
                        )
                    },
                )))
                .service(web::resource("/payouts/active").route(web::get().to(
                    |state, req, query_params| {
                        routing::routing_retrieve_linked_config(
                            state,
                            req,
                            query_params,
                            &TransactionType::Payout,
                        )
                    },
                )))
                .service(
                    web::resource("/payouts/default")
                        .route(web::get().to(|state, req| {
                            routing::routing_retrieve_default_config(
                                state,
                                req,
                                &TransactionType::Payout,
                            )
                        }))
                        .route(web::post().to(|state, req, payload| {
                            routing::routing_update_default_config(
                                state,
                                req,
                                payload,
                                &TransactionType::Payout,
                            )
                        })),
                )
                .service(
                    web::resource("/payouts/{algorithm_id}/activate").route(web::post().to(
                        |state, req, path| {
                            routing::routing_link_config(state, req, path, &TransactionType::Payout)
                        },
                    )),
                )
                .service(web::resource("/payouts/deactivate").route(web::post().to(
                    |state, req, payload| {
                        routing::routing_unlink_config(
                            state,
                            req,
                            payload,
                            &TransactionType::Payout,
                        )
                    },
                )))
                .service(
                    web::resource("/payouts/default/profile/{profile_id}").route(web::post().to(
                        |state, req, path, payload| {
                            routing::routing_update_default_config_for_profile(
                                state,
                                req,
                                path,
                                payload,
                                &TransactionType::Payout,
                            )
                        },
                    )),
                )
                .service(
                    web::resource("/payouts/default/profile").route(web::get().to(|state, req| {
                        routing::routing_retrieve_default_config_for_profiles(
                            state,
                            req,
                            &TransactionType::Payout,
                        )
                    })),
                );
        }

        route = route
            .service(
                web::resource("/{algorithm_id}")
                    .route(web::get().to(routing::routing_retrieve_config)),
            )
            .service(
                web::resource("/{algorithm_id}/activate").route(web::post().to(
                    |state, req, path| {
                        routing::routing_link_config(state, req, path, &TransactionType::Payment)
                    },
                )),
            );
        route
    }
}

pub struct Customers;

#[cfg(all(
    feature = "v2",
    feature = "customer_v2",
    any(feature = "olap", feature = "oltp")
))]
impl Customers {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/v2/customers").app_data(web::Data::new(state));
        #[cfg(all(feature = "olap", feature = "v2", feature = "customer_v2"))]
        {
            route = route
                .service(web::resource("/list").route(web::get().to(customers::customers_list)))
        }
        #[cfg(all(feature = "oltp", feature = "v2", feature = "customer_v2"))]
        {
            route = route
                .service(web::resource("").route(web::post().to(customers::customers_create)))
                .service(
                    web::resource("/{id}")
                        .route(web::put().to(customers::customers_update))
                        .route(web::get().to(customers::customers_retrieve))
                        .route(web::delete().to(customers::customers_delete)),
                )
        }
        route
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "payment_methods_v2"),
    any(feature = "olap", feature = "oltp")
))]
impl Customers {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/customers").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            route = route
                .service(
                    web::resource("/{customer_id}/mandates")
                        .route(web::get().to(customers::get_customer_mandates)),
                )
                .service(web::resource("/list").route(web::get().to(customers::customers_list)))
        }

        #[cfg(feature = "oltp")]
        {
            route = route
                .service(web::resource("").route(web::post().to(customers::customers_create)))
                .service(
                    web::resource("/payment_methods").route(
                        web::get().to(payment_methods::list_customer_payment_method_api_client),
                    ),
                )
                .service(
                    web::resource("/{customer_id}/payment_methods")
                        .route(web::get().to(payment_methods::list_customer_payment_method_api)),
                )
                .service(
                    web::resource("/{customer_id}/payment_methods/{payment_method_id}/default")
                        .route(web::post().to(payment_methods::default_payment_method_set_api)),
                )
                .service(
                    web::resource("/{customer_id}")
                        .route(web::get().to(customers::customers_retrieve))
                        .route(web::post().to(customers::customers_update))
                        .route(web::delete().to(customers::customers_delete)),
                )
        }

        route
    }
}
pub struct Refunds;

#[cfg(all(any(feature = "olap", feature = "oltp"), feature = "v1"))]
impl Refunds {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/refunds").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            route = route
                .service(web::resource("/list").route(web::post().to(refunds_list)))
                .service(web::resource("/profile/list").route(web::post().to(refunds_list_profile)))
                .service(web::resource("/filter").route(web::post().to(refunds_filter_list)))
                .service(web::resource("/v2/filter").route(web::get().to(get_refunds_filters)))
                .service(web::resource("/aggregate").route(web::get().to(get_refunds_aggregates)))
                .service(
                    web::resource("/profile/aggregate")
                        .route(web::get().to(get_refunds_aggregate_profile)),
                )
                .service(
                    web::resource("/v2/profile/filter")
                        .route(web::get().to(get_refunds_filters_profile)),
                )
                .service(
                    web::resource("/{id}/manual-update")
                        .route(web::put().to(refunds_manual_update)),
                );
        }
        #[cfg(feature = "oltp")]
        {
            route = route
                .service(web::resource("").route(web::post().to(refunds_create)))
                .service(web::resource("/sync").route(web::post().to(refunds_retrieve_with_body)))
                .service(
                    web::resource("/{id}")
                        .route(web::get().to(refunds_retrieve))
                        .route(web::post().to(refunds_update)),
                );
        }
        route
    }
}

#[cfg(feature = "payouts")]
pub struct Payouts;

#[cfg(all(feature = "payouts", feature = "v1"))]
impl Payouts {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/payouts").app_data(web::Data::new(state));
        route = route.service(web::resource("/create").route(web::post().to(payouts_create)));

        #[cfg(feature = "olap")]
        {
            route = route
                .service(
                    web::resource("/list")
                        .route(web::get().to(payouts_list))
                        .route(web::post().to(payouts_list_by_filter)),
                )
                .service(
                    web::resource("/profile/list")
                        .route(web::get().to(payouts_list_profile))
                        .route(web::post().to(payouts_list_by_filter_profile)),
                )
                .service(
                    web::resource("/filter")
                        .route(web::post().to(payouts_list_available_filters_for_merchant)),
                )
                .service(
                    web::resource("/profile/filter")
                        .route(web::post().to(payouts_list_available_filters_for_profile)),
                );
        }
        route = route
            .service(
                web::resource("/{payout_id}")
                    .route(web::get().to(payouts_retrieve))
                    .route(web::put().to(payouts_update)),
            )
            .service(web::resource("/{payout_id}/confirm").route(web::post().to(payouts_confirm)))
            .service(web::resource("/{payout_id}/cancel").route(web::post().to(payouts_cancel)))
            .service(web::resource("/{payout_id}/fulfill").route(web::post().to(payouts_fulfill)));
        route
    }
}

#[cfg(all(feature = "oltp", feature = "v2", feature = "payment_methods_v2",))]
impl PaymentMethods {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/v2/payment-methods").app_data(web::Data::new(state));
        route = route
            .service(
                web::resource("").route(web::post().to(payment_methods::create_payment_method_api)),
            )
            .service(
                web::resource("/create-intent")
                    .route(web::post().to(payment_methods::create_payment_method_intent_api)),
            );

        route = route.service(
            web::scope("/{id}")
                .service(
                    web::resource("")
                        .route(web::get().to(payment_methods::payment_method_retrieve_api))
                        .route(web::delete().to(payment_methods::payment_method_delete_api)),
                )
                .service(web::resource("/list-enabled-payment-methods").route(
                    web::get().to(payment_methods::payment_method_session_list_payment_methods),
                ))
                .service(
                    web::resource("/update-saved-payment-method")
                        .route(web::put().to(payment_methods::payment_method_update_api)),
                ),
        );

        route
    }
}
pub struct PaymentMethods;

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    any(feature = "olap", feature = "oltp"),
    not(feature = "customer_v2")
))]
impl PaymentMethods {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/payment_methods").app_data(web::Data::new(state));
        #[cfg(feature = "olap")]
        {
            route =
                route.service(web::resource("/filter").route(
                    web::get().to(
                        payment_methods::list_countries_currencies_for_connector_payment_method,
                    ),
                ));
        }
        #[cfg(feature = "oltp")]
        {
            route = route
                .service(
                    web::resource("")
                        .route(web::post().to(payment_methods::create_payment_method_api))
                        .route(web::get().to(payment_methods::list_payment_method_api)), // TODO : added for sdk compatibility for now, need to deprecate this later
                )
                .service(
                    web::resource("/migrate")
                        .route(web::post().to(payment_methods::migrate_payment_method_api)),
                )
                .service(
                    web::resource("/migrate-batch")
                        .route(web::post().to(payment_methods::migrate_payment_methods)),
                )
                .service(
                    web::resource("/collect")
                        .route(web::post().to(payment_methods::initiate_pm_collect_link_flow)),
                )
                .service(
                    web::resource("/collect/{merchant_id}/{collect_id}")
                        .route(web::get().to(payment_methods::render_pm_collect_link)),
                )
                .service(
                    web::resource("/{payment_method_id}")
                        .route(web::get().to(payment_methods::payment_method_retrieve_api))
                        .route(web::delete().to(payment_methods::payment_method_delete_api)),
                )
                .service(
                    web::resource("/{payment_method_id}/update")
                        .route(web::post().to(payment_methods::payment_method_update_api)),
                )
                .service(
                    web::resource("/{payment_method_id}/save")
                        .route(web::post().to(payment_methods::save_payment_method_api)),
                )
                .service(
                    web::resource("/auth/link").route(web::post().to(pm_auth::link_token_create)),
                )
                .service(
                    web::resource("/auth/exchange").route(web::post().to(pm_auth::exchange_token)),
                )
        }
        route
    }
}

#[cfg(all(feature = "v2", feature = "oltp"))]
pub struct PaymentMethodsSession;

#[cfg(all(feature = "v2", feature = "oltp"))]
impl PaymentMethodsSession {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/v2/payment-methods-session").app_data(web::Data::new(state));
        route = route.service(
            web::resource("")
                .route(web::post().to(payment_methods::payment_methods_session_create)),
        );

        route = route.service(
            web::scope("/{payment_method_session_id}")
                .service(
                    web::resource("")
                        .route(web::get().to(payment_methods::payment_methods_session_retrieve)),
                )
                .service(web::resource("/list-payment-methods").route(
                    web::get().to(payment_methods::payment_method_session_list_payment_methods),
                ))
                .service(
                    web::resource("/update-saved-payment-method").route(
                        web::put().to(
                            payment_methods::payment_method_session_update_saved_payment_method,
                        ),
                    ),
                ),
        );

        route
    }
}

#[cfg(all(feature = "olap", feature = "recon", feature = "v1"))]
pub struct Recon;

#[cfg(all(feature = "olap", feature = "recon", feature = "v1"))]
impl Recon {
    pub fn server(state: AppState) -> Scope {
        web::scope("/recon")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/{merchant_id}/update")
                    .route(web::post().to(recon_routes::update_merchant)),
            )
            .service(web::resource("/token").route(web::get().to(recon_routes::get_recon_token)))
            .service(
                web::resource("/request").route(web::post().to(recon_routes::request_for_recon)),
            )
            .service(
                web::resource("/verify_token")
                    .route(web::get().to(recon_routes::verify_recon_token)),
            )
    }
}

#[cfg(feature = "olap")]
pub struct Blocklist;

#[cfg(all(feature = "olap", feature = "v1"))]
impl Blocklist {
    pub fn server(state: AppState) -> Scope {
        web::scope("/blocklist")
            .app_data(web::Data::new(state))
            .service(
                web::resource("")
                    .route(web::get().to(blocklist::list_blocked_payment_methods))
                    .route(web::post().to(blocklist::add_entry_to_blocklist))
                    .route(web::delete().to(blocklist::remove_entry_from_blocklist)),
            )
            .service(
                web::resource("/toggle").route(web::post().to(blocklist::toggle_blocklist_guard)),
            )
    }
}

#[cfg(feature = "olap")]
pub struct Organization;

#[cfg(all(feature = "olap", feature = "v1"))]
impl Organization {
    pub fn server(state: AppState) -> Scope {
        web::scope("/organization")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(admin::organization_create)))
            .service(
                web::resource("/{id}")
                    .route(web::get().to(admin::organization_retrieve))
                    .route(web::put().to(admin::organization_update)),
            )
    }
}

#[cfg(all(feature = "v2", feature = "olap"))]
impl Organization {
    pub fn server(state: AppState) -> Scope {
        web::scope("/v2/organization")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(admin::organization_create)))
            .service(
                web::scope("/{id}")
                    .service(
                        web::resource("")
                            .route(web::get().to(admin::organization_retrieve))
                            .route(web::put().to(admin::organization_update)),
                    )
                    .service(
                        web::resource("/merchant-accounts")
                            .route(web::get().to(admin::merchant_account_list)),
                    ),
            )
    }
}

pub struct MerchantAccount;

#[cfg(all(feature = "v2", feature = "olap"))]
impl MerchantAccount {
    pub fn server(state: AppState) -> Scope {
        web::scope("/v2/merchant-accounts")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(admin::merchant_account_create)))
            .service(
                web::scope("/{id}")
                    .service(
                        web::resource("")
                            .route(web::get().to(admin::retrieve_merchant_account))
                            .route(web::put().to(admin::update_merchant_account)),
                    )
                    .service(
                        web::resource("/profiles").route(web::get().to(profiles::profiles_list)),
                    ),
            )
    }
}

#[cfg(all(feature = "olap", feature = "v1"))]
impl MerchantAccount {
    pub fn server(state: AppState) -> Scope {
        let mut routes = web::scope("/accounts")
            .service(web::resource("").route(web::post().to(admin::merchant_account_create)))
            .service(web::resource("/list").route(web::get().to(admin::merchant_account_list)))
            .service(
                web::resource("/{id}/kv")
                    .route(web::post().to(admin::merchant_account_toggle_kv))
                    .route(web::get().to(admin::merchant_account_kv_status)),
            )
            .service(
                web::resource("/transfer")
                    .route(web::post().to(admin::merchant_account_transfer_keys)),
            )
            .service(
                web::resource("/kv").route(web::post().to(admin::merchant_account_toggle_all_kv)),
            )
            .service(
                web::resource("/{id}")
                    .route(web::get().to(admin::retrieve_merchant_account))
                    .route(web::post().to(admin::update_merchant_account))
                    .route(web::delete().to(admin::delete_merchant_account)),
            );
        if state.conf.platform.enabled {
            routes = routes.service(
                web::resource("/{id}/platform")
                    .route(web::post().to(admin::merchant_account_enable_platform_account)),
            )
        }
        routes.app_data(web::Data::new(state))
    }
}

pub struct MerchantConnectorAccount;

#[cfg(all(any(feature = "olap", feature = "oltp"), feature = "v2"))]
impl MerchantConnectorAccount {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/v2/connector-accounts").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            use super::admin::*;

            route = route
                .service(web::resource("").route(web::post().to(connector_create)))
                .service(
                    web::resource("/{id}")
                        .route(web::put().to(connector_update))
                        .route(web::get().to(connector_retrieve))
                        .route(web::delete().to(connector_delete)),
                );
        }
        route
    }
}

#[cfg(all(any(feature = "olap", feature = "oltp"), feature = "v1"))]
impl MerchantConnectorAccount {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/account").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            use super::admin::*;

            route = route
                .service(
                    web::resource("/connectors/verify")
                        .route(web::post().to(super::verify_connector::payment_connector_verify)),
                )
                .service(
                    web::resource("/{merchant_id}/connectors")
                        .route(web::post().to(connector_create))
                        .route(web::get().to(connector_list)),
                )
                .service(
                    web::resource("/{merchant_id}/connectors/{merchant_connector_id}")
                        .route(web::get().to(connector_retrieve))
                        .route(web::post().to(connector_update))
                        .route(web::delete().to(connector_delete)),
                );
        }
        #[cfg(feature = "oltp")]
        {
            route = route.service(
                web::resource("/payment_methods")
                    .route(web::get().to(payment_methods::list_payment_method_api)),
            );
        }
        route
    }
}

pub struct EphemeralKey;

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    feature = "oltp"
))]
impl EphemeralKey {
    pub fn server(config: AppState) -> Scope {
        web::scope("/ephemeral_keys")
            .app_data(web::Data::new(config))
            .service(web::resource("").route(web::post().to(ephemeral_key_create)))
            .service(web::resource("/{id}").route(web::delete().to(ephemeral_key_delete)))
    }
}

#[cfg(feature = "v2")]
impl EphemeralKey {
    pub fn server(config: AppState) -> Scope {
        web::scope("/v2/client-secret")
            .app_data(web::Data::new(config))
            .service(web::resource("").route(web::post().to(client_secret_create)))
            .service(web::resource("/{id}").route(web::delete().to(client_secret_delete)))
    }
}

pub struct Mandates;

#[cfg(all(any(feature = "olap", feature = "oltp"), feature = "v1"))]
impl Mandates {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/mandates").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            route =
                route.service(web::resource("/list").route(web::get().to(retrieve_mandates_list)));
            route = route.service(web::resource("/{id}").route(web::get().to(get_mandate)));
        }
        #[cfg(feature = "oltp")]
        {
            route =
                route.service(web::resource("/revoke/{id}").route(web::post().to(revoke_mandate)));
        }
        route
    }
}

pub struct Webhooks;

#[cfg(all(feature = "oltp", feature = "v1"))]
impl Webhooks {
    pub fn server(config: AppState) -> Scope {
        use api_models::webhooks as webhook_type;

        #[allow(unused_mut)]
        let mut route = web::scope("/webhooks")
            .app_data(web::Data::new(config))
            .service(
                web::resource("/{merchant_id}/{connector_id_or_name}")
                    .route(
                        web::post().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>),
                    )
                    .route(web::get().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>))
                    .route(
                        web::put().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>),
                    ),
            );

        #[cfg(feature = "frm")]
        {
            route = route.service(
                web::resource("/frm_fulfillment")
                    .route(web::post().to(frm_routes::frm_fulfillment)),
            );
        }

        route
    }
}

pub struct RelayWebhooks;

#[cfg(feature = "oltp")]
impl RelayWebhooks {
    pub fn server(state: AppState) -> Scope {
        use api_models::webhooks as webhook_type;
        web::scope("/webhooks/relay")
            .app_data(web::Data::new(state))
            .service(web::resource("/{merchant_id}/{connector_id}").route(
                web::post().to(receive_incoming_relay_webhook::<webhook_type::OutgoingWebhook>),
            ))
    }
}

#[cfg(all(feature = "oltp", feature = "v2"))]
impl Webhooks {
    pub fn server(config: AppState) -> Scope {
        use api_models::webhooks as webhook_type;

        #[allow(unused_mut)]
        let mut route = web::scope("/v2/webhooks")
            .app_data(web::Data::new(config))
            .service(
                web::resource("/{merchant_id}/{profile_id}/{connector_id}")
                    .route(
                        web::post().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>),
                    )
                    .route(web::get().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>))
                    .route(
                        web::put().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>),
                    ),
            );

        route
    }
}

pub struct Configs;

#[cfg(any(feature = "olap", feature = "oltp"))]
impl Configs {
    pub fn server(config: AppState) -> Scope {
        web::scope("/configs")
            .app_data(web::Data::new(config))
            .service(web::resource("/").route(web::post().to(config_key_create)))
            .service(
                web::resource("/{key}")
                    .route(web::get().to(config_key_retrieve))
                    .route(web::post().to(config_key_update))
                    .route(web::delete().to(config_key_delete)),
            )
    }
}

pub struct ApplePayCertificatesMigration;

#[cfg(all(feature = "olap", feature = "v1"))]
impl ApplePayCertificatesMigration {
    pub fn server(state: AppState) -> Scope {
        web::scope("/apple_pay_certificates_migration")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(
                web::post().to(apple_pay_certificates_migration::apple_pay_certificates_migration),
            ))
    }
}

pub struct Poll;

#[cfg(all(feature = "oltp", feature = "v1"))]
impl Poll {
    pub fn server(config: AppState) -> Scope {
        web::scope("/poll")
            .app_data(web::Data::new(config))
            .service(
                web::resource("/status/{poll_id}").route(web::get().to(poll::retrieve_poll_status)),
            )
    }
}

pub struct ApiKeys;

#[cfg(all(feature = "olap", feature = "v2"))]
impl ApiKeys {
    pub fn server(state: AppState) -> Scope {
        web::scope("/v2/api-keys")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(api_keys::api_key_create)))
            .service(web::resource("/list").route(web::get().to(api_keys::api_key_list)))
            .service(
                web::resource("/{key_id}")
                    .route(web::get().to(api_keys::api_key_retrieve))
                    .route(web::put().to(api_keys::api_key_update))
                    .route(web::delete().to(api_keys::api_key_revoke)),
            )
    }
}

#[cfg(all(feature = "olap", feature = "v1"))]
impl ApiKeys {
    pub fn server(state: AppState) -> Scope {
        web::scope("/api_keys/{merchant_id}")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(api_keys::api_key_create)))
            .service(web::resource("/list").route(web::get().to(api_keys::api_key_list)))
            .service(
                web::resource("/{key_id}")
                    .route(web::get().to(api_keys::api_key_retrieve))
                    .route(web::post().to(api_keys::api_key_update))
                    .route(web::delete().to(api_keys::api_key_revoke)),
            )
    }
}

pub struct Disputes;

#[cfg(all(feature = "olap", feature = "v1"))]
impl Disputes {
    pub fn server(state: AppState) -> Scope {
        web::scope("/disputes")
            .app_data(web::Data::new(state))
            .service(web::resource("/list").route(web::get().to(disputes::retrieve_disputes_list)))
            .service(
                web::resource("/profile/list")
                    .route(web::get().to(disputes::retrieve_disputes_list_profile)),
            )
            .service(web::resource("/filter").route(web::get().to(disputes::get_disputes_filters)))
            .service(
                web::resource("/profile/filter")
                    .route(web::get().to(disputes::get_disputes_filters_profile)),
            )
            .service(
                web::resource("/accept/{dispute_id}")
                    .route(web::post().to(disputes::accept_dispute)),
            )
            .service(
                web::resource("/aggregate").route(web::get().to(disputes::get_disputes_aggregate)),
            )
            .service(
                web::resource("/profile/aggregate")
                    .route(web::get().to(disputes::get_disputes_aggregate_profile)),
            )
            .service(
                web::resource("/evidence")
                    .route(web::post().to(disputes::submit_dispute_evidence))
                    .route(web::put().to(disputes::attach_dispute_evidence))
                    .route(web::delete().to(disputes::delete_dispute_evidence)),
            )
            .service(
                web::resource("/evidence/{dispute_id}")
                    .route(web::get().to(disputes::retrieve_dispute_evidence)),
            )
            .service(
                web::resource("/{dispute_id}").route(web::get().to(disputes::retrieve_dispute)),
            )
    }
}

pub struct Cards;

#[cfg(feature = "v1")]
impl Cards {
    pub fn server(state: AppState) -> Scope {
        web::scope("/cards")
            .app_data(web::Data::new(state))
            .service(web::resource("/{bin}").route(web::get().to(card_iin_info)))
    }
}

pub struct Files;

#[cfg(all(feature = "olap", feature = "v1"))]
impl Files {
    pub fn server(state: AppState) -> Scope {
        web::scope("/files")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(files::files_create)))
            .service(
                web::resource("/{file_id}")
                    .route(web::delete().to(files::files_delete))
                    .route(web::get().to(files::files_retrieve)),
            )
    }
}

pub struct Cache;

impl Cache {
    pub fn server(state: AppState) -> Scope {
        web::scope("/cache")
            .app_data(web::Data::new(state))
            .service(web::resource("/invalidate/{key}").route(web::post().to(invalidate)))
    }
}

pub struct PaymentLink;

#[cfg(all(feature = "olap", feature = "v1"))]
impl PaymentLink {
    pub fn server(state: AppState) -> Scope {
        web::scope("/payment_link")
            .app_data(web::Data::new(state))
            .service(web::resource("/list").route(web::post().to(payment_link::payments_link_list)))
            .service(
                web::resource("/{payment_link_id}")
                    .route(web::get().to(payment_link::payment_link_retrieve)),
            )
            .service(
                web::resource("{merchant_id}/{payment_id}")
                    .route(web::get().to(payment_link::initiate_payment_link)),
            )
            .service(
                web::resource("s/{merchant_id}/{payment_id}")
                    .route(web::get().to(payment_link::initiate_secure_payment_link)),
            )
            .service(
                web::resource("status/{merchant_id}/{payment_id}")
                    .route(web::get().to(payment_link::payment_link_status)),
            )
    }
}

#[cfg(feature = "payouts")]
pub struct PayoutLink;

#[cfg(all(feature = "payouts", feature = "v1"))]
impl PayoutLink {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/payout_link").app_data(web::Data::new(state));
        route = route.service(
            web::resource("/{merchant_id}/{payout_id}").route(web::get().to(render_payout_link)),
        );
        route
    }
}
pub struct Profile;
#[cfg(all(feature = "olap", feature = "v2"))]
impl Profile {
    pub fn server(state: AppState) -> Scope {
        web::scope("/v2/profiles")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(profiles::profile_create)))
            .service(
                web::scope("/{profile_id}")
                    .service(
                        web::resource("")
                            .route(web::get().to(profiles::profile_retrieve))
                            .route(web::put().to(profiles::profile_update)),
                    )
                    .service(
                        web::resource("/connector-accounts")
                            .route(web::get().to(admin::connector_list)),
                    )
                    .service(
                        web::resource("/fallback-routing")
                            .route(web::get().to(routing::routing_retrieve_default_config))
                            .route(web::patch().to(routing::routing_update_default_config)),
                    )
                    .service(
                        web::resource("/activate-routing-algorithm").route(web::patch().to(
                            |state, req, path, payload| {
                                routing::routing_link_config(
                                    state,
                                    req,
                                    path,
                                    payload,
                                    &TransactionType::Payment,
                                )
                            },
                        )),
                    )
                    .service(
                        web::resource("/deactivate-routing-algorithm").route(web::patch().to(
                            |state, req, path| {
                                routing::routing_unlink_config(
                                    state,
                                    req,
                                    path,
                                    &TransactionType::Payment,
                                )
                            },
                        )),
                    )
                    .service(web::resource("/routing-algorithm").route(web::get().to(
                        |state, req, query_params, path| {
                            routing::routing_retrieve_linked_config(
                                state,
                                req,
                                query_params,
                                path,
                                &TransactionType::Payment,
                            )
                        },
                    )))
                    .service(
                        web::resource("/decision")
                            .route(web::put().to(routing::upsert_decision_manager_config))
                            .route(web::get().to(routing::retrieve_decision_manager_config)),
                    ),
            )
    }
}
#[cfg(all(feature = "olap", feature = "v1"))]
impl Profile {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/account/{account_id}/business_profile")
            .app_data(web::Data::new(state))
            .service(
                web::resource("")
                    .route(web::post().to(profiles::profile_create))
                    .route(web::get().to(profiles::profiles_list)),
            );

        #[cfg(feature = "dynamic_routing")]
        {
            route = route.service(
                web::scope("/{profile_id}/dynamic_routing")
                    .service(
                        web::scope("/success_based")
                            .service(
                                web::resource("/toggle")
                                    .route(web::post().to(routing::toggle_success_based_routing)),
                            )
                            .service(web::resource("/config/{algorithm_id}").route(
                                web::patch().to(|state, req, path, payload| {
                                    routing::success_based_routing_update_configs(
                                        state, req, path, payload,
                                    )
                                }),
                            )),
                    )
                    .service(
                        web::resource("/set_volume_split")
                            .route(web::post().to(routing::set_dynamic_routing_volume_split)),
                    )
                    .service(
                        web::scope("/elimination").service(
                            web::resource("/toggle")
                                .route(web::post().to(routing::toggle_elimination_routing)),
                        ),
                    )
                    .service(
                        web::scope("/contracts")
                            .service(web::resource("/toggle").route(
                                web::post().to(routing::contract_based_routing_setup_config),
                            ))
                            .service(web::resource("/config/{algorithm_id}").route(
                                web::patch().to(|state, req, path, payload| {
                                    routing::contract_based_routing_update_configs(
                                        state, req, path, payload,
                                    )
                                }),
                            )),
                    ),
            );
        }

        route = route.service(
            web::scope("/{profile_id}")
                .service(
                    web::resource("")
                        .route(web::get().to(profiles::profile_retrieve))
                        .route(web::post().to(profiles::profile_update))
                        .route(web::delete().to(profiles::profile_delete)),
                )
                .service(
                    web::resource("/toggle_extended_card_info")
                        .route(web::post().to(profiles::toggle_extended_card_info)),
                )
                .service(
                    web::resource("/toggle_connector_agnostic_mit")
                        .route(web::post().to(profiles::toggle_connector_agnostic_mit)),
                ),
        );

        route
    }
}

pub struct ProfileNew;

#[cfg(feature = "olap")]
impl ProfileNew {
    #[cfg(feature = "v1")]
    pub fn server(state: AppState) -> Scope {
        web::scope("/account/{account_id}/profile")
            .app_data(web::Data::new(state))
            .service(
                web::resource("").route(web::get().to(profiles::profiles_list_at_profile_level)),
            )
            .service(
                web::resource("/connectors").route(web::get().to(admin::connector_list_profile)),
            )
    }
    #[cfg(feature = "v2")]
    pub fn server(state: AppState) -> Scope {
        web::scope("/account/{account_id}/profile").app_data(web::Data::new(state))
    }
}

pub struct Gsm;

#[cfg(all(feature = "olap", feature = "v1"))]
impl Gsm {
    pub fn server(state: AppState) -> Scope {
        web::scope("/gsm")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(gsm::create_gsm_rule)))
            .service(web::resource("/get").route(web::post().to(gsm::get_gsm_rule)))
            .service(web::resource("/update").route(web::post().to(gsm::update_gsm_rule)))
            .service(web::resource("/delete").route(web::post().to(gsm::delete_gsm_rule)))
    }
}

#[cfg(feature = "olap")]
pub struct Verify;

#[cfg(all(feature = "olap", feature = "v1"))]
impl Verify {
    pub fn server(state: AppState) -> Scope {
        web::scope("/verify")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/apple_pay/{merchant_id}")
                    .route(web::post().to(apple_pay_merchant_registration)),
            )
            .service(
                web::resource("/applepay_verified_domains")
                    .route(web::get().to(retrieve_apple_pay_verified_domains)),
            )
    }
}

pub struct User;

#[cfg(all(feature = "olap", feature = "v1"))]
impl User {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/user").app_data(web::Data::new(state));

        route = route
            .service(web::resource("").route(web::get().to(user::get_user_details)))
            .service(web::resource("/signin").route(web::post().to(user::user_signin)))
            .service(web::resource("/v2/signin").route(web::post().to(user::user_signin)))
            // signin/signup with sso using openidconnect
            .service(web::resource("/oidc").route(web::post().to(user::sso_sign)))
            .service(web::resource("/signout").route(web::post().to(user::signout)))
            .service(web::resource("/rotate_password").route(web::post().to(user::rotate_password)))
            .service(web::resource("/change_password").route(web::post().to(user::change_password)))
            .service(
                web::resource("/internal_signup").route(web::post().to(user::internal_user_signup)),
            )
            .service(
                web::resource("/tenant_signup").route(web::post().to(user::create_tenant_user)),
            )
            .service(web::resource("/create_org").route(web::post().to(user::user_org_create)))
            .service(
                web::resource("/create_merchant")
                    .route(web::post().to(user::user_merchant_account_create)),
            )
            // TODO: To be deprecated
            .service(
                web::resource("/permission_info")
                    .route(web::get().to(user_role::get_authorization_info)),
            )
            // TODO: To be deprecated
            .service(
                web::resource("/module/list").route(web::get().to(user_role::get_role_information)),
            )
            .service(
                web::resource("/parent/list")
                    .route(web::get().to(user_role::get_parent_group_info)),
            )
            .service(
                web::resource("/update").route(web::post().to(user::update_user_account_details)),
            )
            .service(
                web::resource("/data")
                    .route(web::get().to(user::get_multiple_dashboard_metadata))
                    .route(web::post().to(user::set_dashboard_metadata)),
            );

        route = route
            .service(web::scope("/key").service(
                web::resource("/transfer").route(web::post().to(user::transfer_user_key)),
            ));

        route = route.service(
            web::scope("/list")
                .service(web::resource("/org").route(web::get().to(user::list_orgs_for_user)))
                .service(
                    web::resource("/merchant")
                        .route(web::get().to(user::list_merchants_for_user_in_org)),
                )
                .service(
                    web::resource("/profile")
                        .route(web::get().to(user::list_profiles_for_user_in_org_and_merchant)),
                )
                .service(
                    web::resource("/invitation")
                        .route(web::get().to(user_role::list_invitations_for_user)),
                ),
        );

        route = route.service(
            web::scope("/switch")
                .service(web::resource("/org").route(web::post().to(user::switch_org_for_user)))
                .service(
                    web::resource("/merchant")
                        .route(web::post().to(user::switch_merchant_for_user_in_org)),
                )
                .service(
                    web::resource("/profile")
                        .route(web::post().to(user::switch_profile_for_user_in_org_and_merchant)),
                ),
        );

        // Two factor auth routes
        route = route.service(
            web::scope("/2fa")
                // TODO: to be deprecated
                .service(web::resource("").route(web::get().to(user::check_two_factor_auth_status)))
                .service(
                    web::resource("/v2")
                        .route(web::get().to(user::check_two_factor_auth_status_with_attempts)),
                )
                .service(
                    web::scope("/totp")
                        .service(web::resource("/begin").route(web::get().to(user::totp_begin)))
                        .service(web::resource("/reset").route(web::get().to(user::totp_reset)))
                        .service(
                            web::resource("/verify")
                                .route(web::post().to(user::totp_verify))
                                .route(web::put().to(user::totp_update)),
                        ),
                )
                .service(
                    web::scope("/recovery_code")
                        .service(
                            web::resource("/verify")
                                .route(web::post().to(user::verify_recovery_code)),
                        )
                        .service(
                            web::resource("/generate")
                                .route(web::get().to(user::generate_recovery_codes)),
                        ),
                )
                .service(
                    web::resource("/terminate")
                        .route(web::get().to(user::terminate_two_factor_auth)),
                ),
        );

        route = route.service(
            web::scope("/auth")
                .service(
                    web::resource("")
                        .route(web::post().to(user::create_user_authentication_method))
                        .route(web::put().to(user::update_user_authentication_method)),
                )
                .service(
                    web::resource("/list")
                        .route(web::get().to(user::list_user_authentication_methods)),
                )
                .service(web::resource("/url").route(web::get().to(user::get_sso_auth_url)))
                .service(
                    web::resource("/select").route(web::post().to(user::terminate_auth_select)),
                ),
        );

        #[cfg(feature = "email")]
        {
            route = route
                .service(web::resource("/from_email").route(web::post().to(user::user_from_email)))
                .service(
                    web::resource("/connect_account")
                        .route(web::post().to(user::user_connect_account)),
                )
                .service(
                    web::resource("/forgot_password").route(web::post().to(user::forgot_password)),
                )
                .service(
                    web::resource("/reset_password").route(web::post().to(user::reset_password)),
                )
                .service(
                    web::resource("/signup_with_merchant_id")
                        .route(web::post().to(user::user_signup_with_merchant_id)),
                )
                .service(web::resource("/verify_email").route(web::post().to(user::verify_email)))
                .service(
                    web::resource("/v2/verify_email").route(web::post().to(user::verify_email)),
                )
                .service(
                    web::resource("/verify_email_request")
                        .route(web::post().to(user::verify_email_request)),
                )
                .service(
                    web::resource("/user/resend_invite").route(web::post().to(user::resend_invite)),
                )
                .service(
                    web::resource("/accept_invite_from_email")
                        .route(web::post().to(user::accept_invite_from_email)),
                );
        }
        #[cfg(not(feature = "email"))]
        {
            route = route.service(web::resource("/signup").route(web::post().to(user::user_signup)))
        }

        // User management
        route = route.service(
            web::scope("/user")
                .service(web::resource("").route(web::post().to(user::list_user_roles_details)))
                // TODO: To be deprecated
                .service(web::resource("/v2").route(web::post().to(user::list_user_roles_details)))
                .service(
                    web::resource("/list").route(web::get().to(user_role::list_users_in_lineage)),
                )
                // TODO: To be deprecated
                .service(
                    web::resource("/v2/list")
                        .route(web::get().to(user_role::list_users_in_lineage)),
                )
                .service(
                    web::resource("/invite_multiple")
                        .route(web::post().to(user::invite_multiple_user)),
                )
                .service(
                    web::scope("/invite/accept")
                        .service(
                            web::resource("")
                                .route(web::post().to(user_role::accept_invitations_v2)),
                        )
                        .service(
                            web::resource("/pre_auth")
                                .route(web::post().to(user_role::accept_invitations_pre_auth)),
                        )
                        .service(
                            web::scope("/v2")
                                .service(
                                    web::resource("")
                                        .route(web::post().to(user_role::accept_invitations_v2)),
                                )
                                .service(
                                    web::resource("/pre_auth").route(
                                        web::post().to(user_role::accept_invitations_pre_auth),
                                    ),
                                ),
                        ),
                )
                .service(
                    web::resource("/update_role")
                        .route(web::post().to(user_role::update_user_role)),
                )
                .service(
                    web::resource("/delete").route(web::delete().to(user_role::delete_user_role)),
                ),
        );

        // Role information
        route =
            route.service(
                web::scope("/role")
                    .service(
                        web::resource("")
                            .route(web::get().to(user_role::get_role_from_token))
                            .route(web::post().to(user_role::create_role)),
                    )
                    .service(web::resource("/v2").route(
                        web::get().to(user_role::get_groups_and_resources_for_role_from_token),
                    ))
                    // TODO: To be deprecated
                    .service(
                        web::resource("/v2/list")
                            .route(web::get().to(user_role::list_roles_with_info)),
                    )
                    .service(
                        web::scope("/list")
                            .service(
                                web::resource("")
                                    .route(web::get().to(user_role::list_roles_with_info)),
                            )
                            .service(web::resource("/invite").route(
                                web::get().to(user_role::list_invitable_roles_at_entity_level),
                            ))
                            .service(web::resource("/update").route(
                                web::get().to(user_role::list_updatable_roles_at_entity_level),
                            )),
                    )
                    .service(
                        web::resource("/{role_id}")
                            .route(web::get().to(user_role::get_role))
                            .route(web::put().to(user_role::update_role)),
                    )
                    .service(
                        web::resource("/{role_id}/v2")
                            .route(web::get().to(user_role::get_parent_info_for_role)),
                    ),
            );

        #[cfg(feature = "dummy_connector")]
        {
            route = route.service(
                web::resource("/sample_data")
                    .route(web::post().to(user::generate_sample_data))
                    .route(web::delete().to(user::delete_sample_data)),
            )
        }

        route = route.service(
            web::scope("/theme")
                .service(
                    web::resource("")
                        .route(web::get().to(user::theme::get_theme_using_lineage))
                        .route(web::post().to(user::theme::create_theme)),
                )
                .service(
                    web::resource("/{theme_id}")
                        .route(web::get().to(user::theme::get_theme_using_theme_id))
                        .route(web::put().to(user::theme::update_theme))
                        .route(web::post().to(user::theme::upload_file_to_theme_storage))
                        .route(web::delete().to(user::theme::delete_theme)),
                ),
        );

        route
    }
}

pub struct ConnectorOnboarding;

#[cfg(all(feature = "olap", feature = "v1"))]
impl ConnectorOnboarding {
    pub fn server(state: AppState) -> Scope {
        web::scope("/connector_onboarding")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/action_url")
                    .route(web::post().to(connector_onboarding::get_action_url)),
            )
            .service(
                web::resource("/sync")
                    .route(web::post().to(connector_onboarding::sync_onboarding_status)),
            )
            .service(
                web::resource("/reset_tracking_id")
                    .route(web::post().to(connector_onboarding::reset_tracking_id)),
            )
    }
}

#[cfg(feature = "olap")]
pub struct WebhookEvents;

#[cfg(all(feature = "olap", feature = "v1"))]
impl WebhookEvents {
    pub fn server(config: AppState) -> Scope {
        web::scope("/events/{merchant_id}")
            .app_data(web::Data::new(config))
            .service(
                web::resource("")
                    .route(web::get().to(webhook_events::list_initial_webhook_delivery_attempts)),
            )
            .service(
                web::scope("/{event_id}")
                    .service(
                        web::resource("attempts")
                            .route(web::get().to(webhook_events::list_webhook_delivery_attempts)),
                    )
                    .service(
                        web::resource("retry")
                            .route(web::post().to(webhook_events::retry_webhook_delivery_attempt)),
                    ),
            )
    }
}

#[cfg(feature = "olap")]
pub struct FeatureMatrix;

#[cfg(all(feature = "olap", feature = "v1"))]
impl FeatureMatrix {
    pub fn server(state: AppState) -> Scope {
        web::scope("/feature_matrix")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::get().to(feature_matrix::fetch_feature_matrix)))
    }
}
