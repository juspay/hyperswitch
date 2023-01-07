#![forbid(unsafe_code)]
// FIXME: I strongly advise to add this warning.
// #![warn(missing_docs)]

// FIXME: I recommend to add these warnings too, although there is no harm if these wanrings will stay disabled.
// #![warn(rust_2018_idioms)]
// #![warn(missing_debug_implementations)]
// #![warn(
//     // clippy::as_conversions,
//     clippy::expect_used,
//     // clippy::integer_arithmetic,
//     clippy::missing_panics_doc,
//     clippy::panic,
//     clippy::panic_in_result_fn,
//     clippy::panicking_unwrap,
//     clippy::todo,
//     clippy::unreachable,
//     clippy::unwrap_in_result,
//     clippy::unwrap_used,
//     clippy::use_self
// )]

use actix_web::{
    body::MessageBody,
    dev::{Server, ServiceFactory, ServiceRequest},
    middleware::ErrorHandlers,
};
use http::StatusCode;
use routes::AppState;

use crate::{
    configs::settings,
    core::errors::{self, BachResult},
    cors, logger, middleware, routes, utils,
};

// pub use self::env::logger;
// use crate::{
//     configs::settings::Settings,
//     core::errors::{self, BachResult},
// };

// #[cfg(feature = "mimalloc")]
// #[global_allocator]
// static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

// /// Header Constants
// pub mod headers {
//     pub const X_API_KEY: &str = "X-API-KEY";
//     pub const CONTENT_TYPE: &str = "Content-Type";
//     pub const X_ROUTER: &str = "X-router";
//     pub const AUTHORIZATION: &str = "Authorization";
//     pub const ACCEPT: &str = "Accept";
//     pub const X_API_VERSION: &str = "X-ApiVersion";
// }

// pub mod pii {
//     //! Personal Identifiable Information protection.

//     pub(crate) use common_utils::pii::{CardNumber, Email};
//     #[doc(inline)]
//     pub use masking::*;
// }

pub fn mk_app(
    state: AppState,
    request_body_limit: usize,
) -> actix_web::App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let json_cfg = actix_web::web::JsonConfig::default()
        .limit(request_body_limit)
        .content_type_required(true)
        .content_type(|mime| mime == mime::APPLICATION_JSON) // FIXME: This doesn't seem to be enforced.
        .error_handler(utils::error_parser::custom_json_error_handler);

    let mut server_app = actix_web::App::new()
        .app_data(json_cfg)
        .wrap(middleware::RequestId)
        .wrap(router_env::tracing_actix_web::TracingLogger::default())
        .wrap(ErrorHandlers::new().handler(
            StatusCode::NOT_FOUND,
            errors::error_handlers::custom_error_handlers,
        ))
        .wrap(ErrorHandlers::new().handler(
            StatusCode::METHOD_NOT_ALLOWED,
            errors::error_handlers::custom_error_handlers,
        ))
        .wrap(cors::cors())
        .service(routes::Payments::olap_server(state.clone()))
        .service(routes::Customers::olap_server(state.clone()))
        .service(routes::Refunds::olap_server(state.clone()))
        .service(routes::Payouts::olap_server(state.clone()))
        //.service(routes::PaymentMethods::olap_server(state.clone()))
        .service(routes::MerchantAccount::olap_server(state.clone()))
        .service(routes::MerchantConnectorAccount::olap_server(state.clone()));
    //.service(routes::EphemeralKey::olap_server(state.clone()))
    //.service(routes::Webhooks::olap_server(state.clone()));

    #[cfg(feature = "stripe")]
    {
        server_app = server_app.service(routes::StripeApis::server(state.clone()));
    }
    server_app = server_app.service(routes::Health::olap_server(state));
    server_app
}

#[allow(clippy::expect_used, clippy::unwrap_used)]
/// # Panics
///
///  Unwrap used because without the value we can't start the server
pub async fn start_olap_server(conf: settings::Settings) -> BachResult<(Server, AppState)> {
    logger::debug!(startup_config=?conf);
    let server = conf.server.clone();
    let state = routes::AppState::new(conf).await;
    // Cloning to close connections before shutdown
    let app_state = state.clone();
    let request_body_limit = server.request_body_limit;
    let server = actix_web::HttpServer::new(move || mk_app(state.clone(), request_body_limit))
        .bind((server.host.as_str(), server.port))?
        .workers(server.workers.unwrap_or_else(num_cpus::get_physical))
        .run();

    Ok((server, app_state))
}
