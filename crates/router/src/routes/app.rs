use std::sync::Arc;

use actix_web::{web, Scope};
#[cfg(all(feature = "business_profile_routing", feature = "olap"))]
use api_models::routing::RoutingRetrieveQuery;
#[cfg(feature = "olap")]
use common_enums::TransactionType;
#[cfg(feature = "email")]
use external_services::email::{ses::AwsSes, EmailService};
use external_services::file_storage::FileStorageInterface;
use hyperswitch_interfaces::{
    encryption_interface::EncryptionManagementInterface,
    secrets_interface::secret_state::{RawSecret, SecuredSecret},
};
use router_env::tracing_actix_web::RequestId;
use scheduler::SchedulerInterface;
use storage_impl::MockDb;
use tokio::sync::oneshot;

#[cfg(feature = "olap")]
use super::blocklist;
#[cfg(feature = "dummy_connector")]
use super::dummy_connector::*;
#[cfg(feature = "payouts")]
use super::payouts::*;
#[cfg(feature = "oltp")]
use super::pm_auth;
#[cfg(feature = "olap")]
use super::routing as cloud_routing;
#[cfg(feature = "olap")]
use super::verification::{apple_pay_merchant_registration, retrieve_apple_pay_verified_domains};
#[cfg(feature = "olap")]
use super::{
    admin::*, api_keys::*, connector_onboarding::*, disputes::*, files::*, gsm::*, payment_link::*,
    user::*, user_role::*, webhook_events::*,
};
use super::{cache::*, health::*};
#[cfg(any(feature = "olap", feature = "oltp"))]
use super::{configs::*, customers::*, mandates::*, payments::*, refunds::*};
#[cfg(any(feature = "olap", feature = "oltp"))]
use super::{currency, payment_methods::*};
#[cfg(feature = "oltp")]
use super::{ephemeral_key::*, webhooks::*};
use crate::configs::secrets_transformers;
#[cfg(all(feature = "frm", feature = "oltp"))]
use crate::routes::fraud_check as frm_routes;
#[cfg(all(feature = "recon", feature = "olap"))]
use crate::routes::recon as recon_routes;
#[cfg(feature = "olap")]
use crate::routes::verify_connector::payment_connector_verify;
pub use crate::{
    analytics::opensearch::OpenSearchClient,
    configs::settings,
    core::routing,
    db::{StorageImpl, StorageInterface},
    events::EventsHandler,
    routes::cards_info::card_iin_info,
    services::get_store,
};

#[derive(Clone)]
pub struct AppState {
    pub flow_name: String,
    pub store: Box<dyn StorageInterface>,
    pub conf: Arc<settings::Settings<RawSecret>>,
    pub event_handler: EventsHandler,
    #[cfg(feature = "email")]
    pub email_client: Arc<dyn EmailService>,
    pub api_client: Box<dyn crate::services::ApiClient>,
    #[cfg(feature = "olap")]
    pub pool: crate::analytics::AnalyticsProvider,
    #[cfg(feature = "olap")]
    pub opensearch_client: OpenSearchClient,
    pub request_id: Option<RequestId>,
    pub file_storage_client: Box<dyn FileStorageInterface>,
    pub encryption_client: Box<dyn EncryptionManagementInterface>,
}

impl scheduler::SchedulerAppState for AppState {
    fn get_db(&self) -> Box<dyn SchedulerInterface> {
        self.store.get_scheduler_db()
    }
}

pub trait AppStateInfo {
    fn conf(&self) -> settings::Settings<RawSecret>;
    fn store(&self) -> Box<dyn StorageInterface>;
    fn event_handler(&self) -> EventsHandler;
    #[cfg(feature = "email")]
    fn email_client(&self) -> Arc<dyn EmailService>;
    fn add_request_id(&mut self, request_id: RequestId);
    fn add_merchant_id(&mut self, merchant_id: Option<String>);
    fn add_flow_name(&mut self, flow_name: String);
    fn get_request_id(&self) -> Option<String>;
}

impl AppStateInfo for AppState {
    fn conf(&self) -> settings::Settings<RawSecret> {
        self.conf.as_ref().to_owned()
    }
    fn store(&self) -> Box<dyn StorageInterface> {
        self.store.to_owned()
    }
    #[cfg(feature = "email")]
    fn email_client(&self) -> Arc<dyn EmailService> {
        self.email_client.to_owned()
    }
    fn event_handler(&self) -> EventsHandler {
        self.event_handler.clone()
    }
    fn add_request_id(&mut self, request_id: RequestId) {
        self.api_client.add_request_id(request_id);
        self.store.add_request_id(request_id.to_string());
        self.request_id.replace(request_id);
    }

    fn add_merchant_id(&mut self, merchant_id: Option<String>) {
        self.api_client.add_merchant_id(merchant_id);
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
pub async fn create_email_client(settings: &settings::Settings<RawSecret>) -> impl EmailService {
    match settings.email.active_email_client {
        external_services::email::AvailableEmailClients::SES => {
            AwsSes::create(&settings.email, settings.proxy.https_url.to_owned()).await
        }
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

        let conf = secrets_transformers::fetch_raw_secrets(conf, &*secret_management_client).await;

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
            let opensearch_client = conf
                .opensearch
                .get_opensearch_client()
                .await
                .expect("Failed to create opensearch client");

            let store: Box<dyn StorageInterface> = match storage_impl {
                StorageImpl::Postgresql | StorageImpl::PostgresqlTest => match &event_handler {
                    EventsHandler::Kafka(kafka_client) => Box::new(
                        crate::db::KafkaStore::new(
                            #[allow(clippy::expect_used)]
                            get_store(&conf.clone(), shut_down_signal, testable)
                                .await
                                .expect("Failed to create store"),
                            kafka_client.clone(),
                        )
                        .await,
                    ),
                    EventsHandler::Logs(_) => Box::new(
                        #[allow(clippy::expect_used)]
                        get_store(&conf, shut_down_signal, testable)
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
            };

            #[cfg(feature = "olap")]
            let pool =
                crate::analytics::AnalyticsProvider::from_conf(conf.analytics.get_inner()).await;

            #[cfg(feature = "email")]
            let email_client = Arc::new(create_email_client(&conf).await);

            let file_storage_client = conf.file_storage.get_file_storage_client().await;

            Self {
                flow_name: String::from("default"),
                store,
                conf: Arc::new(conf),
                #[cfg(feature = "email")]
                email_client,
                api_client,
                event_handler,
                #[cfg(feature = "olap")]
                pool,
                #[cfg(feature = "olap")]
                opensearch_client,
                request_id: None,
                file_storage_client,
                encryption_client,
            }
        })
        .await
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

#[cfg(feature = "dummy_connector")]
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

#[cfg(any(feature = "olap", feature = "oltp"))]
impl Payments {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/payments").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            route = route
                .service(
                    web::resource("/list")
                        .route(web::get().to(payments_list))
                        .route(web::post().to(payments_list_by_filter)),
                )
                .service(web::resource("/filter").route(web::post().to(get_filters_for_payments)))
        }
        #[cfg(feature = "oltp")]
        {
            route = route
                .service(web::resource("").route(web::post().to(payments_create)))
                .service(
                    web::resource("/session_tokens")
                        .route(web::post().to(payments_connector_session)),
                )
                .service(
                    web::resource("/sync")
                        .route(web::post().to(payments_retrieve_with_gateway_creds)),
                )
                .service(
                    web::resource("/{payment_id}")
                        .route(web::get().to(payments_retrieve))
                        .route(web::post().to(payments_update)),
                )
                .service(
                    web::resource("/{payment_id}/confirm").route(web::post().to(payments_confirm)),
                )
                .service(
                    web::resource("/{payment_id}/cancel").route(web::post().to(payments_cancel)),
                )
                .service(
                    web::resource("/{payment_id}/capture").route(web::post().to(payments_capture)),
                )
                .service(
                    web::resource("/{payment_id}/approve")
                        .route(web::post().to(payments_approve)),
                )
                .service(
                    web::resource("/{payment_id}/reject")
                        .route(web::post().to(payments_reject)),
                )
                .service(
                    web::resource("/redirect/{payment_id}/{merchant_id}/{attempt_id}")
                        .route(web::get().to(payments_start)),
                )
                .service(
                    web::resource(
                        "/{payment_id}/{merchant_id}/redirect/response/{connector}/{creds_identifier}",
                    )
                    .route(web::get().to(payments_redirect_response_with_creds_identifier)),
                )
                .service(
                    web::resource("/{payment_id}/{merchant_id}/redirect/response/{connector}")
                        .route(web::get().to(payments_redirect_response))
                        .route(web::post().to(payments_redirect_response))
                )
                .service(
                    web::resource("/{payment_id}/{merchant_id}/redirect/complete/{connector}")
                        .route(web::get().to(payments_complete_authorize))
                        .route(web::post().to(payments_complete_authorize)),
                )
                .service(
                    web::resource("/{payment_id}/incremental_authorization").route(web::post().to(payments_incremental_authorization)),
                )
                .service(
                    web::resource("/{payment_id}/{merchant_id}/authorize/{connector}").route(web::post().to(post_3ds_payments_authorize)),
                )
                .service(
                    web::resource("/{payment_id}/3ds/authentication").route(web::post().to(payments_external_authentication)),
                );
        }
        route
    }
}

#[cfg(any(feature = "olap", feature = "oltp"))]
pub struct Forex;

#[cfg(any(feature = "olap", feature = "oltp"))]
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

#[cfg(feature = "olap")]
impl Routing {
    pub fn server(state: AppState) -> Scope {
        #[allow(unused_mut)]
        let mut route = web::scope("/routing")
            .app_data(web::Data::new(state.clone()))
            .service(web::resource("/active").route(web::get().to(
                |state, req, #[cfg(feature = "business_profile_routing")] query_params| {
                    cloud_routing::routing_retrieve_linked_config(
                        state,
                        req,
                        #[cfg(feature = "business_profile_routing")]
                        query_params,
                        #[cfg(feature = "business_profile_routing")]
                        &TransactionType::Payment,
                    )
                },
            )))
            .service(
                web::resource("")
                    .route(
                        #[cfg(feature = "business_profile_routing")]
                        web::get().to(|state, req, path: web::Query<RoutingRetrieveQuery>| {
                            cloud_routing::list_routing_configs(
                                state,
                                req,
                                path,
                                &TransactionType::Payment,
                            )
                        }),
                        #[cfg(not(feature = "business_profile_routing"))]
                        web::get().to(cloud_routing::list_routing_configs),
                    )
                    .route(web::post().to(|state, req, payload| {
                        cloud_routing::routing_create_config(
                            state,
                            req,
                            payload,
                            &TransactionType::Payment,
                        )
                    })),
            )
            .service(
                web::resource("/default")
                    .route(web::get().to(|state, req| {
                        cloud_routing::routing_retrieve_default_config(
                            state,
                            req,
                            &TransactionType::Payment,
                        )
                    }))
                    .route(web::post().to(|state, req, payload| {
                        cloud_routing::routing_update_default_config(
                            state,
                            req,
                            payload,
                            &TransactionType::Payment,
                        )
                    })),
            )
            .service(web::resource("/deactivate").route(web::post().to(
                |state, req, #[cfg(feature = "business_profile_routing")] payload| {
                    cloud_routing::routing_unlink_config(
                        state,
                        req,
                        #[cfg(feature = "business_profile_routing")]
                        payload,
                        &TransactionType::Payment,
                    )
                },
            )))
            .service(
                web::resource("/decision")
                    .route(web::put().to(cloud_routing::upsert_decision_manager_config))
                    .route(web::get().to(cloud_routing::retrieve_decision_manager_config))
                    .route(web::delete().to(cloud_routing::delete_decision_manager_config)),
            )
            .service(
                web::resource("/decision/surcharge")
                    .route(web::put().to(cloud_routing::upsert_surcharge_decision_manager_config))
                    .route(web::get().to(cloud_routing::retrieve_surcharge_decision_manager_config))
                    .route(
                        web::delete().to(cloud_routing::delete_surcharge_decision_manager_config),
                    ),
            )
            .service(
                web::resource("/default/profile/{profile_id}").route(web::post().to(
                    |state, req, path, payload| {
                        cloud_routing::routing_update_default_config_for_profile(
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
                    cloud_routing::routing_retrieve_default_config_for_profiles(
                        state,
                        req,
                        &TransactionType::Payment,
                    )
                })),
            );

        #[cfg(feature = "payouts")]
        {
            route = route
                .service(
                    web::resource("/payouts")
                        .route(
                            #[cfg(feature = "business_profile_routing")]
                            web::get().to(|state, req, path: web::Query<RoutingRetrieveQuery>| {
                                cloud_routing::list_routing_configs(
                                    state,
                                    req,
                                    #[cfg(feature = "business_profile_routing")]
                                    path,
                                    #[cfg(feature = "business_profile_routing")]
                                    &TransactionType::Payout,
                                )
                            }),
                            #[cfg(not(feature = "business_profile_routing"))]
                            web::get().to(cloud_routing::list_routing_configs),
                        )
                        .route(web::post().to(|state, req, payload| {
                            cloud_routing::routing_create_config(
                                state,
                                req,
                                payload,
                                &TransactionType::Payout,
                            )
                        })),
                )
                .service(web::resource("/payouts/active").route(web::get().to(
                    |state, req, #[cfg(feature = "business_profile_routing")] query_params| {
                        cloud_routing::routing_retrieve_linked_config(
                            state,
                            req,
                            #[cfg(feature = "business_profile_routing")]
                            query_params,
                            #[cfg(feature = "business_profile_routing")]
                            &TransactionType::Payout,
                        )
                    },
                )))
                .service(
                    web::resource("/payouts/default")
                        .route(web::get().to(|state, req| {
                            cloud_routing::routing_retrieve_default_config(
                                state,
                                req,
                                &TransactionType::Payout,
                            )
                        }))
                        .route(web::post().to(|state, req, payload| {
                            cloud_routing::routing_update_default_config(
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
                            cloud_routing::routing_link_config(
                                state,
                                req,
                                path,
                                &TransactionType::Payout,
                            )
                        },
                    )),
                )
                .service(web::resource("/payouts/deactivate").route(web::post().to(
                    |state, req, #[cfg(feature = "business_profile_routing")] payload| {
                        cloud_routing::routing_unlink_config(
                            state,
                            req,
                            #[cfg(feature = "business_profile_routing")]
                            payload,
                            &TransactionType::Payout,
                        )
                    },
                )))
                .service(
                    web::resource("/payouts/default/profile/{profile_id}").route(web::post().to(
                        |state, req, path, payload| {
                            cloud_routing::routing_update_default_config_for_profile(
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
                        cloud_routing::routing_retrieve_default_config_for_profiles(
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
                    .route(web::get().to(cloud_routing::routing_retrieve_config)),
            )
            .service(
                web::resource("/{algorithm_id}/activate").route(web::post().to(
                    |state, req, path| {
                        cloud_routing::routing_link_config(
                            state,
                            req,
                            path,
                            &TransactionType::Payment,
                        )
                    },
                )),
            );
        route
    }
}

pub struct Customers;

#[cfg(any(feature = "olap", feature = "oltp"))]
impl Customers {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/customers").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            route = route
                .service(
                    web::resource("/{customer_id}/mandates")
                        .route(web::get().to(get_customer_mandates)),
                )
                .service(web::resource("/list").route(web::get().to(customers_list)))
        }

        #[cfg(feature = "oltp")]
        {
            route = route
                .service(web::resource("").route(web::post().to(customers_create)))
                .service(
                    web::resource("/payment_methods")
                        .route(web::get().to(list_customer_payment_method_api_client)),
                )
                .service(
                    web::resource("/{customer_id}/payment_methods")
                        .route(web::get().to(list_customer_payment_method_api)),
                )
                .service(
                    web::resource("/{customer_id}/payment_methods/{payment_method_id}/default")
                        .route(web::post().to(default_payment_method_set_api)),
                )
                .service(
                    web::resource("/{customer_id}")
                        .route(web::get().to(customers_retrieve))
                        .route(web::post().to(customers_update))
                        .route(web::delete().to(customers_delete)),
                );
        }

        route
    }
}

pub struct Refunds;

#[cfg(any(feature = "olap", feature = "oltp"))]
impl Refunds {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/refunds").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            route = route
                .service(web::resource("/list").route(web::post().to(refunds_list)))
                .service(web::resource("/filter").route(web::post().to(refunds_filter_list)));
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

#[cfg(feature = "payouts")]
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
                    web::resource("/filter").route(web::post().to(payouts_list_available_filters)),
                );
        }
        route = route
            .service(
                web::resource("/{payout_id}")
                    .route(web::get().to(payouts_retrieve))
                    .route(web::put().to(payouts_update)),
            )
            .service(web::resource("/{payout_id}/cancel").route(web::post().to(payouts_cancel)))
            .service(web::resource("/{payout_id}/fulfill").route(web::post().to(payouts_fulfill)));
        route
    }
}

pub struct PaymentMethods;

#[cfg(any(feature = "olap", feature = "oltp"))]
impl PaymentMethods {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/payment_methods").app_data(web::Data::new(state));
        #[cfg(feature = "olap")]
        {
            route = route.service(
                web::resource("/filter")
                    .route(web::get().to(list_countries_currencies_for_connector_payment_method)),
            );
        }
        #[cfg(feature = "oltp")]
        {
            route = route
                .service(
                    web::resource("")
                        .route(web::post().to(create_payment_method_api))
                        .route(web::get().to(list_payment_method_api)), // TODO : added for sdk compatibility for now, need to deprecate this later
                )
                .service(
                    web::resource("/{payment_method_id}")
                        .route(web::get().to(payment_method_retrieve_api))
                        .route(web::post().to(payment_method_update_api))
                        .route(web::delete().to(payment_method_delete_api)),
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

#[cfg(all(feature = "olap", feature = "recon"))]
pub struct Recon;

#[cfg(all(feature = "olap", feature = "recon"))]
impl Recon {
    pub fn server(state: AppState) -> Scope {
        web::scope("/recon")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/update_merchant")
                    .route(web::post().to(recon_routes::update_merchant)),
            )
            .service(web::resource("/token").route(web::get().to(recon_routes::get_recon_token)))
            .service(
                web::resource("/request").route(web::post().to(recon_routes::request_for_recon)),
            )
            .service(web::resource("/verify_token").route(web::get().to(verify_recon_token)))
    }
}

#[cfg(feature = "olap")]
pub struct Blocklist;

#[cfg(feature = "olap")]
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

pub struct MerchantAccount;

#[cfg(feature = "olap")]
impl MerchantAccount {
    pub fn server(state: AppState) -> Scope {
        web::scope("/accounts")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(merchant_account_create)))
            .service(web::resource("/list").route(web::get().to(merchant_account_list)))
            .service(
                web::resource("/{id}/kv")
                    .route(web::post().to(merchant_account_toggle_kv))
                    .route(web::get().to(merchant_account_kv_status)),
            )
            .service(
                web::resource("/{id}")
                    .route(web::get().to(retrieve_merchant_account))
                    .route(web::post().to(update_merchant_account))
                    .route(web::delete().to(delete_merchant_account)),
            )
    }
}

pub struct MerchantConnectorAccount;

#[cfg(any(feature = "olap", feature = "oltp"))]
impl MerchantConnectorAccount {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/account").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            use super::admin::*;

            route = route
                .service(
                    web::resource("/connectors/verify")
                        .route(web::post().to(payment_connector_verify)),
                )
                .service(
                    web::resource("/{merchant_id}/connectors")
                        .route(web::post().to(payment_connector_create))
                        .route(web::get().to(payment_connector_list)),
                )
                .service(
                    web::resource("/{merchant_id}/connectors/{merchant_connector_id}")
                        .route(web::get().to(payment_connector_retrieve))
                        .route(web::post().to(payment_connector_update))
                        .route(web::delete().to(payment_connector_delete)),
                );
        }
        #[cfg(feature = "oltp")]
        {
            route = route.service(
                web::resource("/payment_methods").route(web::get().to(list_payment_method_api)),
            );
        }
        route
    }
}

pub struct EphemeralKey;

#[cfg(feature = "oltp")]
impl EphemeralKey {
    pub fn server(config: AppState) -> Scope {
        web::scope("/ephemeral_keys")
            .app_data(web::Data::new(config))
            .service(web::resource("").route(web::post().to(ephemeral_key_create)))
            .service(web::resource("/{id}").route(web::delete().to(ephemeral_key_delete)))
    }
}

pub struct Mandates;

#[cfg(any(feature = "olap", feature = "oltp"))]
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

#[cfg(feature = "oltp")]
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

pub struct ApiKeys;

#[cfg(feature = "olap")]
impl ApiKeys {
    pub fn server(state: AppState) -> Scope {
        web::scope("/api_keys/{merchant_id}")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(api_key_create)))
            .service(web::resource("/list").route(web::get().to(api_key_list)))
            .service(
                web::resource("/{key_id}")
                    .route(web::get().to(api_key_retrieve))
                    .route(web::post().to(api_key_update))
                    .route(web::delete().to(api_key_revoke)),
            )
    }
}

pub struct Disputes;

#[cfg(feature = "olap")]
impl Disputes {
    pub fn server(state: AppState) -> Scope {
        web::scope("/disputes")
            .app_data(web::Data::new(state))
            .service(web::resource("/list").route(web::get().to(retrieve_disputes_list)))
            .service(web::resource("/accept/{dispute_id}").route(web::post().to(accept_dispute)))
            .service(
                web::resource("/evidence")
                    .route(web::post().to(submit_dispute_evidence))
                    .route(web::put().to(attach_dispute_evidence))
                    .route(web::delete().to(delete_dispute_evidence)),
            )
            .service(
                web::resource("/evidence/{dispute_id}")
                    .route(web::get().to(retrieve_dispute_evidence)),
            )
            .service(web::resource("/{dispute_id}").route(web::get().to(retrieve_dispute)))
    }
}

pub struct Cards;

impl Cards {
    pub fn server(state: AppState) -> Scope {
        web::scope("/cards")
            .app_data(web::Data::new(state))
            .service(web::resource("/{bin}").route(web::get().to(card_iin_info)))
    }
}

pub struct Files;

#[cfg(feature = "olap")]
impl Files {
    pub fn server(state: AppState) -> Scope {
        web::scope("/files")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(files_create)))
            .service(
                web::resource("/{file_id}")
                    .route(web::delete().to(files_delete))
                    .route(web::get().to(files_retrieve)),
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
#[cfg(feature = "olap")]
impl PaymentLink {
    pub fn server(state: AppState) -> Scope {
        web::scope("/payment_link")
            .app_data(web::Data::new(state))
            .service(web::resource("/list").route(web::post().to(payments_link_list)))
            .service(
                web::resource("/{payment_link_id}").route(web::get().to(payment_link_retrieve)),
            )
            .service(
                web::resource("{merchant_id}/{payment_id}")
                    .route(web::get().to(initiate_payment_link)),
            )
            .service(
                web::resource("status/{merchant_id}/{payment_id}")
                    .route(web::get().to(payment_link_status)),
            )
    }
}

pub struct BusinessProfile;

#[cfg(feature = "olap")]
impl BusinessProfile {
    pub fn server(state: AppState) -> Scope {
        web::scope("/account/{account_id}/business_profile")
            .app_data(web::Data::new(state))
            .service(
                web::resource("")
                    .route(web::post().to(business_profile_create))
                    .route(web::get().to(business_profiles_list)),
            )
            .service(
                web::resource("/{profile_id}")
                    .route(web::get().to(business_profile_retrieve))
                    .route(web::post().to(business_profile_update))
                    .route(web::delete().to(business_profile_delete)),
            )
    }
}

pub struct Gsm;

#[cfg(feature = "olap")]
impl Gsm {
    pub fn server(state: AppState) -> Scope {
        web::scope("/gsm")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(create_gsm_rule)))
            .service(web::resource("/get").route(web::post().to(get_gsm_rule)))
            .service(web::resource("/update").route(web::post().to(update_gsm_rule)))
            .service(web::resource("/delete").route(web::post().to(delete_gsm_rule)))
    }
}

#[cfg(feature = "olap")]
pub struct Verify;

#[cfg(feature = "olap")]
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

#[cfg(feature = "olap")]
impl User {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/user").app_data(web::Data::new(state));

        route = route
            .service(
                web::resource("/signin").route(web::post().to(user_signin_without_invite_checks)),
            )
            .service(web::resource("/v2/signin").route(web::post().to(user_signin)))
            .service(web::resource("/signout").route(web::post().to(signout)))
            .service(web::resource("/change_password").route(web::post().to(change_password)))
            .service(web::resource("/internal_signup").route(web::post().to(internal_user_signup)))
            .service(web::resource("/switch_merchant").route(web::post().to(switch_merchant_id)))
            .service(
                web::resource("/create_merchant")
                    .route(web::post().to(user_merchant_account_create)),
            )
            .service(web::resource("/switch/list").route(web::get().to(list_merchant_ids_for_user)))
            .service(web::resource("/permission_info").route(web::get().to(get_authorization_info)))
            .service(web::resource("/update").route(web::post().to(update_user_account_details)))
            .service(
                web::resource("/data")
                    .route(web::get().to(get_multiple_dashboard_metadata))
                    .route(web::post().to(set_dashboard_metadata)),
            );

        #[cfg(feature = "email")]
        {
            route = route
                .service(
                    web::resource("/connect_account").route(web::post().to(user_connect_account)),
                )
                .service(web::resource("/forgot_password").route(web::post().to(forgot_password)))
                .service(web::resource("/reset_password").route(web::post().to(reset_password)))
                .service(
                    web::resource("/signup_with_merchant_id")
                        .route(web::post().to(user_signup_with_merchant_id)),
                )
                .service(
                    web::resource("/verify_email")
                        .route(web::post().to(verify_email_without_invite_checks)),
                )
                .service(web::resource("/v2/verify_email").route(web::post().to(verify_email)))
                .service(
                    web::resource("/verify_email_request")
                        .route(web::post().to(verify_email_request)),
                )
                .service(web::resource("/user/resend_invite").route(web::post().to(resend_invite)))
                .service(
                    web::resource("/accept_invite_from_email")
                        .route(web::post().to(accept_invite_from_email)),
                );
        }
        #[cfg(not(feature = "email"))]
        {
            route = route.service(web::resource("/signup").route(web::post().to(user_signup)))
        }

        // User management
        route = route.service(
            web::scope("/user")
                .service(web::resource("").route(web::get().to(get_user_role_details)))
                .service(
                    web::resource("/list").route(web::get().to(list_users_for_merchant_account)),
                )
                .service(web::resource("/invite").route(web::post().to(invite_user)))
                .service(
                    web::resource("/invite_multiple").route(web::post().to(invite_multiple_user)),
                )
                .service(web::resource("/invite/accept").route(web::post().to(accept_invitation)))
                .service(web::resource("/update_role").route(web::post().to(update_user_role)))
                .service(
                    web::resource("/transfer_ownership")
                        .route(web::post().to(transfer_org_ownership)),
                )
                .service(web::resource("/delete").route(web::delete().to(delete_user_role))),
        );

        // Role information
        route = route.service(
            web::scope("/role")
                .service(
                    web::resource("")
                        .route(web::get().to(get_role_from_token))
                        .route(web::post().to(create_role)),
                )
                .service(web::resource("/list").route(web::get().to(list_all_roles)))
                .service(
                    web::resource("/{role_id}")
                        .route(web::get().to(get_role))
                        .route(web::put().to(update_role)),
                ),
        );

        #[cfg(feature = "dummy_connector")]
        {
            route = route.service(
                web::resource("/sample_data")
                    .route(web::post().to(generate_sample_data))
                    .route(web::delete().to(delete_sample_data)),
            )
        }
        route
    }
}

pub struct ConnectorOnboarding;

#[cfg(feature = "olap")]
impl ConnectorOnboarding {
    pub fn server(state: AppState) -> Scope {
        web::scope("/connector_onboarding")
            .app_data(web::Data::new(state))
            .service(web::resource("/action_url").route(web::post().to(get_action_url)))
            .service(web::resource("/sync").route(web::post().to(sync_onboarding_status)))
            .service(web::resource("/reset_tracking_id").route(web::post().to(reset_tracking_id)))
    }
}

#[cfg(feature = "olap")]
pub struct WebhookEvents;

#[cfg(feature = "olap")]
impl WebhookEvents {
    pub fn server(config: AppState) -> Scope {
        web::scope("/events/{merchant_id_or_profile_id}")
            .app_data(web::Data::new(config))
            .service(web::resource("").route(web::get().to(list_initial_webhook_delivery_attempts)))
            .service(
                web::resource("/{event_id}/attempts")
                    .route(web::get().to(list_webhook_delivery_attempts)),
            )
    }
}
