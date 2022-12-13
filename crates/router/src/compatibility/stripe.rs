mod app;
mod customers;
mod payment_intents;
mod refunds;
mod setup_intents;
use actix_web::{web, Scope};
mod errors;
pub(crate) use errors::ErrorCode;

pub(crate) use self::app::{Customers, PaymentIntents, Refunds, SetupIntents};
use crate::routes::AppState;
pub struct StripeApis;

impl StripeApis {
    pub(crate) fn server(state: AppState) -> Scope {
        let max_depth = 10;
        let strict = false;
        web::scope("/vs/v1")
            .app_data(web::Data::new(serde_qs::Config::new(max_depth, strict)))
            .service(SetupIntents::server(state.clone()))
            .service(PaymentIntents::server(state.clone()))
            .service(Refunds::server(state.clone()))
            .service(Customers::server(state))
    }
}
