// FIXME: I strongly advise to add this worning.
// #![warn(missing_docs)]

// FIXME: I recommend to add these wornings too, although there is no harm if these wanrings will stay disabled.
// #![warn(rust_2018_idioms)]
// #![warn(missing_debug_implementations)]
#![warn(
    // clippy::as_conversions,
    clippy::expect_used,
    // clippy::integer_arithmetic,
    clippy::missing_panics_doc,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::panicking_unwrap,
    clippy::unreachable,
    clippy::unwrap_in_result,
)]

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
pub mod pii;
pub mod routes;
pub mod scheduler;
#[allow(unused_imports)] // Allow unused imports only for schema module
pub mod schema;
pub mod services;
pub mod types;
pub mod utils;

use std::sync::Arc;

use actix_web::{dev::Server, middleware::ErrorHandlers};
use http::StatusCode;
use routes::AppState;

pub use self::env::logger;
use crate::{
    configs::settings::Settings,
    core::errors::{self, BachResult},
    db::SqlDb,
    services::Store,
};

/// Header Constants
pub mod headers {
    pub const X_API_KEY: &str = "X-API-KEY";
    pub const CONTENT_TYPE: &str = "Content-Type";
    pub const X_ROUTER: &str = "X-router";
    pub const AUTHORIZATION: &str = "Authorization";
}
#[allow(clippy::expect_used, clippy::unwrap_used)]
/// # Panics
///
///  Unwrap used because without the value we can't start the server
pub async fn start_server(conf: Settings) -> BachResult<(Server, AppState)> {
    logger::debug!(startup_config=?conf);

    let server = conf.server.clone();
    let state = routes::AppState {
        flow_name: String::from("default"),
        store: Store {
            pg_pool: SqlDb::new(&conf.database).await,
            // FIXME: from my understanding, this creates a single connection
            // for the entire lifetime of the server. This doesn't survive disconnects
            // from redis. Consider using connection pool.
            redis_conn: Arc::new(connection::redis_connection(&conf).await),
        },
        conf,
    };
    // Cloning to close connections before shutdown
    let app_state = state.clone();

    let request_body_limit = server.request_body_limit;

    let server = actix_web::HttpServer::new(move || {
        let json_cfg = actix_web::web::JsonConfig::default()
            .limit(request_body_limit)
            .content_type_required(true)
            .content_type(|mime| mime == mime::APPLICATION_JSON) // FIXME: This doesn't seem to be enforced.
            .error_handler(utils::error_parser::custom_json_error_handler);

        let mut server_app = actix_web::App::new()
            .app_data(json_cfg)
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
            .service(routes::Payments::server(state.clone()))
            .service(routes::Customers::server(state.clone()))
            .service(routes::Refunds::server(state.clone()))
            .service(routes::Payouts::server(state.clone()))
            .service(routes::PaymentMethods::server(state.clone()))
            .service(routes::MerchantAccount::server(state.clone()))
            .service(routes::MerchantConnectorAccount::server(state.clone()))
            .service(routes::Webhooks::server(state.clone()));

        #[cfg(feature = "stripe")]
        {
            server_app = server_app.service(routes::StripeApis::server(state.clone()));
        }
        server_app = server_app.service(routes::Health::server(state.clone()));
        server_app
    })
    .bind((server.host.as_str(), server.port))?
    .run();

    Ok((server, app_state))
}
