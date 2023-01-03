use actix_web::{web, Scope};

use super::{
    admin::*, customers::*, ephemeral_key::*, health::*, mandates::*, payment_methods::*,
    payments::*, payouts::*, refunds::*, webhooks::*,
};
use crate::{
    configs::settings::Settings,
    db::{MockDb, StorageImpl, StorageInterface},
    services::Store,
};

#[derive(Clone)]
pub struct AppState {
    pub flow_name: String,
    pub store: Box<dyn StorageInterface>,
    pub conf: Settings,
}

impl AppState {
    pub async fn with_storage(conf: Settings, storage_impl: StorageImpl) -> Self {
        let testable = storage_impl == StorageImpl::PostgresqlTest;
        let store: Box<dyn StorageInterface> = match storage_impl {
            StorageImpl::Postgresql | StorageImpl::PostgresqlTest => {
                Box::new(Store::new(&conf, testable).await)
            }
            StorageImpl::Mock => Box::new(MockDb::new(&conf).await),
        };

        Self {
            flow_name: String::from("default"),
            store,
            conf,
        }
    }

    #[allow(unused_variables)]
    pub async fn new(conf: Settings) -> Self {
        Self::with_storage(conf, StorageImpl::Postgresql).await
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

pub struct Payments;

impl Payments {
    pub fn server(state: AppState) -> Scope {
        // Routes are matched in the order they are declared.
        web::scope("/payments")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(payments_create)))
            .service(web::resource("/list").route(web::get().to(payments_list)))
            .service(
                web::resource("/session_tokens").route(web::post().to(payments_connector_session)),
            )
            .service(
                web::resource("/{payment_id}")
                    .route(web::get().to(payments_retrieve))
                    .route(web::post().to(payments_update)),
            )
            .service(web::resource("/{payment_id}/confirm").route(web::post().to(payments_confirm)))
            .service(web::resource("/{payment_id}/cancel").route(web::post().to(payments_cancel)))
            .service(web::resource("/{payment_id}/capture").route(web::post().to(payments_capture)))
            .service(
                web::resource("/start/{payment_id}/{merchant_id}/{attempt_id}")
                    .route(web::get().to(payments_start)),
            )
            .service(
                web::resource("/{payment_id}/{merchant_id}/response/{connector}")
                    .route(web::get().to(payments_response)),
            )
    }
}

pub struct Customers;

impl Customers {
    pub fn server(state: AppState) -> Scope {
        web::scope("/customers")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(customers_create)))
            .service(
                web::resource("/{customer_id}")
                    .route(web::get().to(customers_retrieve))
                    .route(web::post().to(customers_update))
                    .route(web::delete().to(customers_delete)),
            )
            .service(
                web::resource("/{customer_id}/mandates")
                    .route(web::get().to(get_customer_mandates)),
            )
            .service(
                web::resource("/{customer_id}/payment_methods")
                    .route(web::get().to(list_customer_payment_method_api)),
            )
    }
}

pub struct Refunds;

impl Refunds {
    pub fn server(state: AppState) -> Scope {
        // Routes are matches in the order they are declared.
        web::scope("/refunds")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(refunds_create)))
            .service(
                web::resource("/{id}")
                    .route(web::get().to(refunds_retrieve))
                    .route(web::post().to(refunds_update)),
            )
            .service(web::resource("/list").route(web::get().to(refunds_list)))
    }
}

pub struct Payouts;

impl Payouts {
    pub fn server(state: AppState) -> Scope {
        web::scope("/payouts")
            .app_data(web::Data::new(state))
            .service(web::resource("/create").route(web::post().to(payouts_create)))
            .service(web::resource("/retrieve").route(web::get().to(payouts_retrieve)))
            .service(web::resource("/update").route(web::post().to(payouts_update)))
            .service(web::resource("/reverse").route(web::post().to(payouts_reverse)))
            .service(web::resource("/cancel").route(web::post().to(payouts_cancel)))
            .service(web::resource("/accounts").route(web::get().to(payouts_accounts)))
    }
}

pub struct PaymentMethods;

impl PaymentMethods {
    pub fn server(state: AppState) -> Scope {
        web::scope("/payment_methods")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(create_payment_method_api)))
            .service(
                web::resource("/{payment_method_id}")
                    .route(web::get().to(payment_method_retrieve_api))
                    .route(web::post().to(payment_method_update_api))
                    .route(web::delete().to(payment_method_delete_api)),
            )
    }
}

pub struct MerchantAccount;

impl MerchantAccount {
    pub fn server(config: AppState) -> Scope {
        web::scope("/accounts")
            .app_data(web::Data::new(config))
            .service(web::resource("").route(web::post().to(merchant_account_create)))
            .service(
                web::resource("/{id}")
                    .route(web::get().to(retrieve_merchant_account))
                    .route(web::post().to(update_merchant_account))
                    .route(web::delete().to(delete_merchant_account)),
            )
    }
}

pub struct MerchantConnectorAccount;

impl MerchantConnectorAccount {
    pub fn server(config: AppState) -> Scope {
        web::scope("/account")
            .app_data(web::Data::new(config))
            .service(
                web::resource("/{merchant_id}/connectors")
                    .route(web::post().to(payment_connector_create))
                    .route(web::get().to(payment_connector_list)),
            )
            .service(
                web::resource("/{merchant_id}/payment_methods")
                    .route(web::get().to(list_payment_method_api)),
            )
            .service(
                web::resource("/{merchant_id}/connectors/{merchant_connector_id}")
                    .route(web::get().to(payment_connector_retrieve))
                    .route(web::post().to(payment_connector_update))
                    .route(web::delete().to(payment_connector_delete)),
            )
    }
}

pub struct EphemeralKey;

impl EphemeralKey {
    pub fn server(config: AppState) -> Scope {
        web::scope("/ephemeral_keys")
            .app_data(web::Data::new(config))
            .service(web::resource("").route(web::post().to(ephemeral_key_create)))
            .service(web::resource("/{id}").route(web::delete().to(ephemeral_key_delete)))
    }
}

pub struct Mandates;

impl Mandates {
    pub fn server(config: AppState) -> Scope {
        web::scope("/mandates")
            .app_data(web::Data::new(config))
            .service(web::resource("/{id}").route(web::get().to(get_mandate)))
            .service(web::resource("/revoke/{id}").route(web::post().to(revoke_mandate)))
    }
}

pub struct Webhooks;

impl Webhooks {
    pub fn server(config: AppState) -> Scope {
        web::scope("/webhooks")
            .app_data(web::Data::new(config))
            .service(
                web::resource("/{merchant_id}/{connector}")
                    .route(web::post().to(receive_incoming_webhook)),
            )
    }
}
