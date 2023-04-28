use actix_web::{web, Scope};

use super::{customers::*, payment_intents::*, refunds::*, setup_intents::*, webhooks::*};
use crate::routes::{self, webhooks};

pub struct PaymentIntents;

impl PaymentIntents {
    pub fn server(state: routes::AppState) -> Scope {
        web::scope("/payment_intents")
            .app_data(web::Data::new(state))
            .service(
                web::resource("/sync")
                    .route(web::post().to(payment_intents_retrieve_with_gateway_creds)),
            )
            .service(web::resource("/").route(web::post().to(payment_intents_create)))
            .service(web::resource("/{payment_id}").route(web::get().to(payment_intents_retrieve)))
            .service(web::resource("/{payment_id}").route(web::post().to(payment_intents_update)))
            .service(
                web::resource("/{payment_id}/confirm")
                    .route(web::post().to(payment_intents_confirm)),
            )
            .service(
                web::resource("/{payment_id}/capture")
                    .route(web::post().to(payment_intents_capture)),
            )
    }
}

pub struct SetupIntents;

impl SetupIntents {
    pub fn server(state: routes::AppState) -> Scope {
        web::scope("/setup_intents")
            .app_data(web::Data::new(state))
            .service(web::resource("/").route(web::post().to(setup_intents_create)))
            .service(web::resource("/{setup_id}").route(web::get().to(setup_intents_retrieve)))
            .service(web::resource("/{setup_id}").route(web::post().to(setup_intents_update)))
            .service(
                web::resource("/{setup_id}/confirm").route(web::post().to(setup_intents_confirm)),
            )
    }
}

pub struct Refunds;

impl Refunds {
    pub fn server(config: routes::AppState) -> Scope {
        web::scope("/refunds")
            .app_data(web::Data::new(config))
            .service(web::resource("/").route(web::post().to(refund_create)))
            .service(web::resource("/{refund_id}").route(web::get().to(refund_retrieve)))
            .service(web::resource("/{refund_id}").route(web::post().to(refund_update)))
            .service(
                web::resource("/sync").route(web::post().to(refund_retrieve_with_gateway_creds)),
            )
    }
}

pub struct Customers;

impl Customers {
    pub fn server(config: routes::AppState) -> Scope {
        web::scope("/customers")
            .app_data(web::Data::new(config))
            .service(web::resource("/").route(web::post().to(customer_create)))
            .service(web::resource("/{customer_id}").route(web::get().to(customer_retrieve)))
            .service(web::resource("/{customer_id}").route(web::post().to(customer_update)))
            .service(web::resource("/{customer_id}").route(web::delete().to(customer_delete)))
            .service(
                web::resource("/{customer_id}/payment_methods")
                    .route(web::get().to(list_customer_payment_method_api)),
            )
    }
}

pub struct Webhooks;

impl Webhooks {
    pub fn server(config: routes::AppState) -> Scope {
        web::scope("/webhooks")
            .app_data(web::Data::new(config))
            .service(
                web::resource("/{merchant_id}/{connector}")
                    .route(
                        web::post().to(webhooks::receive_incoming_webhook::<StripeOutgoingWebhook>),
                    )
                    .route(
                        web::get().to(webhooks::receive_incoming_webhook::<StripeOutgoingWebhook>),
                    ),
            )
    }
}
