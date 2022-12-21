mod app;
mod customers;
mod payment_intents;
mod refunds;
mod setup_intents;
use actix_web::{web, Scope};
mod errors;

use crate::routes;
pub struct StripeApis;

impl StripeApis {
    pub(crate) fn server(state: routes::AppState) -> Scope {
        let max_depth = 10;
        let strict = false;
        web::scope("/vs/v1")
            .app_data(web::Data::new(serde_qs::Config::new(max_depth, strict)))
            .service(app::SetupIntents::server(state.clone()))
            .service(app::PaymentIntents::server(state.clone()))
            .service(app::Refunds::server(state.clone()))
            .service(app::Customers::server(state))
    }
}
