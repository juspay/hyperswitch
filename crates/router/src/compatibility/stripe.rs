pub mod app;
pub mod customers;
pub mod payment_intents;
pub mod payouts;
pub mod refunds;
pub mod setup_intents;
pub mod webhooks;
use actix_web::{web, Scope};
pub mod errors;

use crate::routes;
pub struct StripeApis;

impl StripeApis {
    pub fn server(state: routes::AppState) -> Scope {
        let max_depth = 10;
        let strict = false;
        web::scope("/vs/v1")
            .app_data(web::Data::new(serde_qs::Config::new(max_depth, strict)))
            .service(app::SetupIntents::server(state.clone()))
            .service(app::PaymentIntents::server(state.clone()))
            .service(app::Refunds::server(state.clone()))
            .service(app::Customers::server(state.clone()))
            .service(app::Webhooks::server(state.clone()))
            .service(app::Mandates::server(state))
    }
}
