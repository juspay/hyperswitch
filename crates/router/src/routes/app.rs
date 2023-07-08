use actix_web::{web, Scope};
#[cfg(feature = "email")]
use external_services::email::{AwsSes, EmailClient};
use tokio::sync::oneshot;

#[cfg(feature = "dummy_connector")]
use super::dummy_connector::*;
#[cfg(feature = "olap")]
use super::{admin::*, api_keys::*, disputes::*, files::*};
use super::{cache::*, health::*};
#[cfg(any(feature = "olap", feature = "oltp"))]
use super::{configs::*, customers::*, mandates::*, payments::*, payouts::*, refunds::*};
#[cfg(feature = "oltp")]
use super::{ephemeral_key::*, payment_methods::*, webhooks::*};
#[cfg(feature = "kms")]
use crate::configs::kms;
use crate::{
    configs::settings,
    db::{MockDb, StorageImpl, StorageInterface},
    routes::cards_info::card_iin_info,
    services::Store,
};

#[derive(Clone)]
pub struct AppState {
    pub flow_name: String,
    pub store: Box<dyn StorageInterface>,
    pub conf: settings::Settings,
    #[cfg(feature = "email")]
    pub email_client: Box<dyn EmailClient>,
    #[cfg(feature = "kms")]
    pub kms_secrets: settings::ActiveKmsSecrets,
}

pub trait AppStateInfo {
    fn conf(&self) -> settings::Settings;
    fn flow_name(&self) -> String;
    fn store(&self) -> Box<dyn StorageInterface>;
    #[cfg(feature = "email")]
    fn email_client(&self) -> Box<dyn EmailClient>;
}

impl AppStateInfo for AppState {
    fn conf(&self) -> settings::Settings {
        self.conf.to_owned()
    }
    fn flow_name(&self) -> String {
        self.flow_name.to_owned()
    }
    fn store(&self) -> Box<dyn StorageInterface> {
        self.store.to_owned()
    }
    #[cfg(feature = "email")]
    fn email_client(&self) -> Box<dyn EmailClient> {
        self.email_client.to_owned()
    }
}

impl AppState {
    pub async fn with_storage(
        conf: settings::Settings,
        storage_impl: StorageImpl,
        shut_down_signal: oneshot::Sender<()>,
    ) -> Self {
        let testable = storage_impl == StorageImpl::PostgresqlTest;
        let store: Box<dyn StorageInterface> = match storage_impl {
            StorageImpl::Postgresql | StorageImpl::PostgresqlTest => {
                Box::new(Store::new(&conf, testable, shut_down_signal).await)
            }
            StorageImpl::Mock => Box::new(MockDb::new(&conf).await),
        };

        #[cfg(feature = "kms")]
        #[allow(clippy::expect_used)]
        let kms_secrets = kms::KmsDecrypt::decrypt_inner(
            settings::ActiveKmsSecrets {
                jwekey: conf.jwekey.clone().into(),
            },
            &conf.kms,
        )
        .await
        .expect("Failed while performing KMS decryption");

        #[cfg(feature = "email")]
        #[allow(clippy::expect_used)]
        let email_client = Box::new(AwsSes::new(&conf.email).await);
        Self {
            flow_name: String::from("default"),
            store,
            conf,
            #[cfg(feature = "email")]
            email_client,
            #[cfg(feature = "kms")]
            kms_secrets,
        }
    }

    pub async fn new(conf: settings::Settings, shut_down_signal: oneshot::Sender<()>) -> Self {
        Self::with_storage(conf, StorageImpl::Postgresql, shut_down_signal).await
    }
}

pub struct Health;

impl Health {
    pub fn server(state: AppState) -> Scope {
        web::scope("")
            .app_data(web::Data::new(state))
            .service(web::resource("/health").route(web::get().to(health)))
    }
}

#[cfg(feature = "dummy_connector")]
pub struct DummyConnector;

#[cfg(feature = "dummy_connector")]
impl DummyConnector {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/dummy-connector").app_data(web::Data::new(state));
        #[cfg(not(feature = "external_access_dc"))]
        {
            route = route.guard(actix_web::guard::Host("localhost"));
        }
        route = route
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
        route
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
                .service(web::resource("/list").route(web::get().to(payments_list)))
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
                );
        }
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
            route = route.service(
                web::resource("/{customer_id}/mandates")
                    .route(web::get().to(get_customer_mandates)),
            );
        }

        #[cfg(feature = "oltp")]
        {
            route = route
                .service(web::resource("").route(web::post().to(customers_create)))
                .service(
                    web::resource("/{customer_id}")
                        .route(web::get().to(customers_retrieve))
                        .route(web::post().to(customers_update))
                        .route(web::delete().to(customers_delete)),
                )
                .service(
                    web::resource("/{customer_id}/payment_methods")
                        .route(web::get().to(list_customer_payment_method_api)),
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

pub struct Payouts;

#[cfg(any(feature = "olap", feature = "oltp"))]
impl Payouts {
    pub fn server(state: AppState) -> Scope {
        let mut route = web::scope("/payouts").app_data(web::Data::new(state));

        #[cfg(feature = "olap")]
        {
            route =
                route.service(web::resource("/accounts").route(web::get().to(payouts_accounts)));
        }
        #[cfg(feature = "oltp")]
        {
            route = route
                .service(web::resource("/create").route(web::post().to(payouts_create)))
                .service(web::resource("/retrieve").route(web::get().to(payouts_retrieve)))
                .service(web::resource("/update").route(web::post().to(payouts_update)))
                .service(web::resource("/reverse").route(web::post().to(payouts_reverse)))
                .service(web::resource("/cancel").route(web::post().to(payouts_cancel)));
        }
        route
    }
}

pub struct PaymentMethods;

#[cfg(feature = "oltp")]
impl PaymentMethods {
    pub fn server(state: AppState) -> Scope {
        web::scope("/payment_methods")
            .app_data(web::Data::new(state))
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
    }
}

pub struct MerchantAccount;

#[cfg(feature = "olap")]
impl MerchantAccount {
    pub fn server(state: AppState) -> Scope {
        web::scope("/accounts")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(merchant_account_create)))
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

        web::scope("/webhooks")
            .app_data(web::Data::new(config))
            .service(
                web::resource("/{merchant_id}/{connector}")
                    .route(
                        web::post().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>),
                    )
                    .route(web::get().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>))
                    .route(
                        web::put().to(receive_incoming_webhook::<webhook_type::OutgoingWebhook>),
                    ),
            )
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
                    .route(web::post().to(config_key_update)),
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
                    .route(web::put().to(attach_dispute_evidence)),
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
