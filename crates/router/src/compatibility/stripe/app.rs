use actix_web::{web, Scope};

use super::{customers::*, payment_intents::*, refunds::*, setup_intents::*};
use crate::routes::AppState;

pub struct PaymentIntents;

impl PaymentIntents {
    pub fn server(state: AppState) -> Scope {
        web::scope("/payment_intents")
            .app_data(web::Data::new(state))
            .service(payment_intents_create)
            .service(payment_intents_retrieve)
            .service(payment_intents_update)
            .service(payment_intents_confirm)
            .service(payment_intents_capture)
    }
}

pub struct SetupIntents;

impl SetupIntents {
    pub fn server(state: AppState) -> Scope {
        web::scope("/setup_intents")
            .app_data(web::Data::new(state))
            .service(setup_intents_create)
            .service(setup_intents_retrieve)
            .service(setup_intents_update)
            .service(setup_intents_confirm)
    }
}

pub struct Refunds;

impl Refunds {
    pub fn server(config: AppState) -> Scope {
        web::scope("/refunds")
            .app_data(web::Data::new(config))
            .service(refund_create)
            .service(refund_retrieve)
            .service(refund_update)
    }
}

pub struct Customers;

impl Customers {
    pub fn server(config: AppState) -> Scope {
        web::scope("/customers")
            .app_data(web::Data::new(config))
            .service(customer_create)
            .service(customer_retrieve)
            .service(customer_update)
            .service(customer_delete)
    }
}
