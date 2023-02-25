#![forbid(unsafe_code)]
#![recursion_limit = "256"]

#[cfg(feature = "stripe")]
pub mod compatibility;
pub mod configs;
pub mod connection;
pub mod connector;
pub(crate) mod consts;
pub mod core;
pub mod cors;
pub mod db;
pub mod env;
pub(crate) mod macros;
pub mod routes;
pub mod scheduler;

pub mod middleware;
#[cfg(feature = "openapi")]
pub mod openapi;
pub mod services;
pub mod types;
pub mod utils;

use actix_web::{
    body::MessageBody,
    dev::{Server, ServiceFactory, ServiceRequest},
    middleware::ErrorHandlers,
};
use http::StatusCode;
use routes::AppState;

pub use self::env::logger;
use crate::{
    configs::settings,
    core::errors::{self, ApplicationResult},
};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Header Constants
pub mod headers {
    pub const X_API_KEY: &str = "X-API-KEY";
    pub const CONTENT_TYPE: &str = "Content-Type";
    pub const X_ROUTER: &str = "X-router";
    pub const AUTHORIZATION: &str = "Authorization";
    pub const ACCEPT: &str = "Accept";
    pub const X_API_VERSION: &str = "X-ApiVersion";
    pub const DATE: &str = "Date";
    pub const X_MERCHANT_ID: &str = "X-Merchant-Id";
    pub const X_LOGIN: &str = "X-Login";
    pub const X_TRANS_KEY: &str = "X-Trans-Key";
    pub const X_VERSION: &str = "X-Version";
    pub const X_DATE: &str = "X-Date";
}

pub mod pii {
    //! Personal Identifiable Information protection.

    pub(crate) use common_utils::pii::{CardNumber, Email};
    #[doc(inline)]
    pub use masking::*;
}

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
    let mut server_app = get_application_builder(request_body_limit);

    #[cfg(any(feature = "olap", feature = "oltp"))]
    {
        server_app = server_app
            .service(routes::Payments::server(state.clone()))
            .service(routes::Customers::server(state.clone()))
            .service(routes::Configs::server(state.clone()))
            .service(routes::Refunds::server(state.clone()))
            .service(routes::Payouts::server(state.clone()))
            .service(routes::MerchantConnectorAccount::server(state.clone()))
            .service(routes::Mandates::server(state.clone()));
    }

    #[cfg(feature = "oltp")]
    {
        server_app = server_app
            .service(routes::PaymentMethods::server(state.clone()))
            .service(routes::EphemeralKey::server(state.clone()))
            .service(routes::Webhooks::server(state.clone()));
    }

    #[cfg(feature = "olap")]
    {
        server_app = server_app
            .service(routes::MerchantAccount::server(state.clone()))
            .service(routes::ApiKeys::server(state.clone()));
    }

    #[cfg(feature = "stripe")]
    {
        server_app = server_app.service(routes::StripeApis::server(state.clone()));
    }
    server_app = server_app.service(routes::Health::server(state));
    server_app
}

/// Starts the server
///
/// # Panics
///
///  Unwrap used because without the value we can't start the server
#[allow(clippy::expect_used, clippy::unwrap_used)]
pub async fn start_server(conf: settings::Settings) -> ApplicationResult<(Server, AppState)> {
    logger::debug!(startup_config=?conf);
    let server = conf.server.clone();
    let state = routes::AppState::new(conf).await;
    // Cloning to close connections before shutdown
    let app_state = state.clone();
    let request_body_limit = server.request_body_limit;
    let server = actix_web::HttpServer::new(move || mk_app(state.clone(), request_body_limit))
        .bind((server.host.as_str(), server.port))?
        .workers(server.workers)
        .shutdown_timeout(server.shutdown_timeout)
        .run();

    Ok((server, app_state))
}

pub fn get_application_builder(
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

    actix_web::App::new()
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
}
