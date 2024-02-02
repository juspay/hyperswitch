#![forbid(unsafe_code)]
#![recursion_limit = "256"]

#[cfg(feature = "stripe")]
pub mod compatibility;
pub mod configs;
pub mod connection;
pub mod connector;
pub mod consts;
pub mod core;
pub mod cors;
pub mod db;
pub mod env;
pub(crate) mod macros;

pub mod routes;
pub mod workflows;

#[cfg(feature = "olap")]
pub mod analytics;
pub mod events;
pub mod middleware;
pub mod services;
pub mod types;
pub mod utils;

use actix_web::{
    body::MessageBody,
    dev::{Server, ServerHandle, ServiceFactory, ServiceRequest},
    middleware::ErrorHandlers,
};
use http::StatusCode;
use routes::AppState;
use storage_impl::errors::ApplicationResult;
use tokio::sync::{mpsc, oneshot};

pub use self::env::logger;
pub(crate) use self::macros::*;
use crate::{configs::settings, core::errors};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Header Constants
pub mod headers {
    pub const ACCEPT: &str = "Accept";
    pub const API_KEY: &str = "API-KEY";
    pub const APIKEY: &str = "apikey";
    pub const X_CC_API_KEY: &str = "X-CC-Api-Key";
    pub const API_TOKEN: &str = "Api-Token";
    pub const AUTHORIZATION: &str = "Authorization";
    pub const CONTENT_TYPE: &str = "Content-Type";
    pub const DATE: &str = "Date";
    pub const IDEMPOTENCY_KEY: &str = "Idempotency-Key";
    pub const NONCE: &str = "nonce";
    pub const TIMESTAMP: &str = "Timestamp";
    pub const TOKEN: &str = "token";
    pub const X_API_KEY: &str = "X-API-KEY";
    pub const X_API_VERSION: &str = "X-ApiVersion";
    pub const X_FORWARDED_FOR: &str = "X-Forwarded-For";
    pub const X_MERCHANT_ID: &str = "X-Merchant-Id";
    pub const X_LOGIN: &str = "X-Login";
    pub const X_TRANS_KEY: &str = "X-Trans-Key";
    pub const X_VERSION: &str = "X-Version";
    pub const X_CC_VERSION: &str = "X-CC-Version";
    pub const X_ACCEPT_VERSION: &str = "X-Accept-Version";
    pub const X_DATE: &str = "X-Date";
    pub const X_WEBHOOK_SIGNATURE: &str = "X-Webhook-Signature-512";
    pub const X_REQUEST_ID: &str = "X-Request-Id";
    pub const STRIPE_COMPATIBLE_WEBHOOK_SIGNATURE: &str = "Stripe-Signature";
}

pub mod pii {
    //! Personal Identifiable Information protection.

    pub(crate) use common_utils::pii::Email;
    #[doc(inline)]
    pub use masking::*;
}

/// Creates and configures an Actix web application with various routes based on the given `state` and `request_body_limit`.
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

    #[cfg(feature = "dummy_connector")]
    {
        use routes::DummyConnector;
        server_app = server_app.service(DummyConnector::server(state.clone()));
    }

    #[cfg(any(feature = "olap", feature = "oltp"))]
    {
        #[cfg(feature = "olap")]
        {
            // This is a more specific route as compared to `MerchantConnectorAccount`
            // so it is registered before `MerchantConnectorAccount`.
            server_app = server_app.service(routes::BusinessProfile::server(state.clone()))
        }
        server_app = server_app
            .service(routes::Payments::server(state.clone()))
            .service(routes::Customers::server(state.clone()))
            .service(routes::Configs::server(state.clone()))
            .service(routes::Forex::server(state.clone()))
            .service(routes::Refunds::server(state.clone()))
            .service(routes::MerchantConnectorAccount::server(state.clone()))
            .service(routes::Mandates::server(state.clone()))
    }

    #[cfg(feature = "oltp")]
    {
        server_app = server_app
            .service(routes::EphemeralKey::server(state.clone()))
            .service(routes::Webhooks::server(state.clone()))
            .service(routes::PaymentMethods::server(state.clone()))
    }

    #[cfg(feature = "olap")]
    {
        server_app = server_app
            .service(routes::MerchantAccount::server(state.clone()))
            .service(routes::ApiKeys::server(state.clone()))
            .service(routes::Files::server(state.clone()))
            .service(routes::Disputes::server(state.clone()))
            .service(routes::Analytics::server(state.clone()))
            .service(routes::Routing::server(state.clone()))
            .service(routes::Blocklist::server(state.clone()))
            .service(routes::LockerMigrate::server(state.clone()))
            .service(routes::Gsm::server(state.clone()))
            .service(routes::PaymentLink::server(state.clone()))
            .service(routes::User::server(state.clone()))
            .service(routes::ConnectorOnboarding::server(state.clone()))
    }

    #[cfg(all(feature = "olap", feature = "kms"))]
    {
        server_app = server_app.service(routes::Verify::server(state.clone()));
    }

    #[cfg(feature = "payouts")]
    {
        server_app = server_app.service(routes::Payouts::server(state.clone()));
    }

    #[cfg(feature = "stripe")]
    {
        server_app = server_app.service(routes::StripeApis::server(state.clone()));
    }

    #[cfg(feature = "recon")]
    {
        server_app = server_app.service(routes::Recon::server(state.clone()));
    }

    server_app = server_app.service(routes::Cards::server(state.clone()));
    server_app = server_app.service(routes::Cache::server(state.clone()));
    server_app = server_app.service(routes::Health::server(state));

    server_app
}

/// Starts the server
///
/// # Panics
///
///  Unwrap used because without the value we can't start the server
#[allow(clippy::expect_used, clippy::unwrap_used)]
/// Starts an async server using the provided settings configuration.
pub async fn start_server(conf: settings::Settings) -> ApplicationResult<Server> {
    logger::debug!(startup_config=?conf);
    let server = conf.server.clone();
    let (tx, rx) = oneshot::channel();
    let api_client = Box::new(
        services::ProxyClient::new(
            conf.proxy.clone(),
            services::proxy_bypass_urls(&conf.locker),
        )
        .map_err(|error| {
            errors::ApplicationError::ApiClientError(error.current_context().clone())
        })?,
    );
    let state = Box::pin(routes::AppState::new(conf, tx, api_client)).await;
    let request_body_limit = server.request_body_limit;
    let server = actix_web::HttpServer::new(move || mk_app(state.clone(), request_body_limit))
        .bind((server.host.as_str(), server.port))?
        .workers(server.workers)
        .shutdown_timeout(server.shutdown_timeout)
        .run();
    tokio::spawn(receiver_for_error(rx, server.handle()));
    Ok(server)
}

/// Asynchronously waits for a signal from the given oneshot receiver. If the signal is received successfully, it logs an error message indicating that the redis server has failed and then stops the server. If an error occurs while receiving the signal, it logs an error message indicating the channel receiver error.
pub async fn receiver_for_error(rx: oneshot::Receiver<()>, mut server: impl Stop) {
    match rx.await {
        Ok(_) => {
            logger::error!("The redis server failed ");
            server.stop_server().await;
        }
        Err(err) => {
            logger::error!("Channel receiver error{err}");
        }
    }
}

#[async_trait::async_trait]
pub trait Stop {
    async fn stop_server(&mut self);
}

#[async_trait::async_trait]
impl Stop for ServerHandle {
        /// Asynchronously stops the server by calling the `stop` method with the `true` parameter and awaiting the result.
    async fn stop_server(&mut self) {
        let _ = self.stop(true).await;
    }
}
#[async_trait::async_trait]
impl Stop for mpsc::Sender<()> {
        /// Asynchronously stops the server by sending an empty message and awaiting the response.
    async fn stop_server(&mut self) {
        let _ = self.send(()).await.map_err(|err| logger::error!("{err}"));
    }
}

/// Returns a new actix_web::App with configurations for handling HTTP requests. 
/// The method sets the limit for request body size, adds error handlers for NOT_FOUND and METHOD_NOT_ALLOWED status codes, 
/// sets default response headers, adds request ID middleware, CORS middleware, logging middleware, and request details logging middleware.
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
        .error_handler(utils::error_parser::custom_json_error_handler);

    actix_web::App::new()
        .app_data(json_cfg)
        .wrap(ErrorHandlers::new().handler(
            StatusCode::NOT_FOUND,
            errors::error_handlers::custom_error_handlers,
        ))
        .wrap(ErrorHandlers::new().handler(
            StatusCode::METHOD_NOT_ALLOWED,
            errors::error_handlers::custom_error_handlers,
        ))
        .wrap(middleware::default_response_headers())
        .wrap(middleware::RequestId)
        .wrap(cors::cors())
        .wrap(middleware::LogSpanInitializer)
        // this middleware works only for Http1.1 requests
        .wrap(middleware::Http400RequestDetailsLogger)
        .wrap(router_env::tracing_actix_web::TracingLogger::default())
}
