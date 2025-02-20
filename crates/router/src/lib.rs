#[cfg(all(feature = "stripe", feature = "v1"))]
pub mod compatibility;
pub mod configs;
pub mod connection;
pub mod connector;
pub mod consts;
pub mod core;
pub mod cors;
pub mod db;
pub mod env;
pub mod locale;
pub(crate) mod macros;

pub mod routes;
pub mod workflows;

#[cfg(feature = "olap")]
pub mod analytics;
pub mod analytics_validator;
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
use hyperswitch_interfaces::secrets_interface::secret_state::SecuredSecret;
use router_env::tracing::Instrument;
use routes::{AppState, SessionState};
use storage_impl::errors::ApplicationResult;
use tokio::sync::{mpsc, oneshot};

pub use self::env::logger;
pub(crate) use self::macros::*;
use crate::{configs::settings, core::errors};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Import translate fn in root
use crate::locale::{_rust_i18n_t, _rust_i18n_try_translate};

/// Header Constants
pub mod headers {
    pub const ACCEPT: &str = "Accept";
    pub const ACCEPT_LANGUAGE: &str = "Accept-Language";
    pub const KEY: &str = "key";
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
    pub const USER_AGENT: &str = "User-Agent";
    pub const X_API_KEY: &str = "X-API-KEY";
    pub const X_API_VERSION: &str = "X-ApiVersion";
    pub const X_FORWARDED_FOR: &str = "X-Forwarded-For";
    pub const X_MERCHANT_ID: &str = "X-Merchant-Id";
    pub const X_ORGANIZATION_ID: &str = "X-Organization-Id";
    pub const X_LOGIN: &str = "X-Login";
    pub const X_TRANS_KEY: &str = "X-Trans-Key";
    pub const X_VERSION: &str = "X-Version";
    pub const X_CC_VERSION: &str = "X-CC-Version";
    pub const X_ACCEPT_VERSION: &str = "X-Accept-Version";
    pub const X_DATE: &str = "X-Date";
    pub const X_WEBHOOK_SIGNATURE: &str = "X-Webhook-Signature-512";
    pub const X_REQUEST_ID: &str = "X-Request-Id";
    pub const X_PROFILE_ID: &str = "X-Profile-Id";
    pub const STRIPE_COMPATIBLE_WEBHOOK_SIGNATURE: &str = "Stripe-Signature";
    pub const STRIPE_COMPATIBLE_CONNECT_ACCOUNT: &str = "Stripe-Account";
    pub const X_CLIENT_VERSION: &str = "X-Client-Version";
    pub const X_CLIENT_SOURCE: &str = "X-Client-Source";
    pub const X_PAYMENT_CONFIRM_SOURCE: &str = "X-Payment-Confirm-Source";
    pub const CONTENT_LENGTH: &str = "Content-Length";
    pub const BROWSER_NAME: &str = "x-browser-name";
    pub const X_CLIENT_PLATFORM: &str = "x-client-platform";
    pub const X_MERCHANT_DOMAIN: &str = "x-merchant-domain";
    pub const X_APP_ID: &str = "x-app-id";
    pub const X_REDIRECT_URI: &str = "x-redirect-uri";
    pub const X_TENANT_ID: &str = "x-tenant-id";
    pub const X_CLIENT_SECRET: &str = "X-Client-Secret";
    pub const X_CUSTOMER_ID: &str = "X-Customer-Id";
    pub const X_CONNECTED_MERCHANT_ID: &str = "x-connected-merchant-id";
}

pub mod pii {
    //! Personal Identifiable Information protection.

    pub(crate) use common_utils::pii::Email;
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
    let mut server_app = get_application_builder(request_body_limit, state.conf.cors.clone());

    #[cfg(all(feature = "dummy_connector", feature = "v1"))]
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
            #[cfg(feature = "v1")]
            {
                server_app = server_app
                    .service(routes::ProfileNew::server(state.clone()))
                    .service(routes::Forex::server(state.clone()));
            }

            server_app = server_app.service(routes::Profile::server(state.clone()));
        }
        server_app = server_app
            .service(routes::Payments::server(state.clone()))
            .service(routes::Customers::server(state.clone()))
            .service(routes::Configs::server(state.clone()))
            .service(routes::MerchantConnectorAccount::server(state.clone()))
            .service(routes::RelayWebhooks::server(state.clone()))
            .service(routes::Webhooks::server(state.clone()))
            .service(routes::Hypersense::server(state.clone()))
            .service(routes::Relay::server(state.clone()));

        #[cfg(feature = "oltp")]
        {
            server_app = server_app.service(routes::PaymentMethods::server(state.clone()));
        }

        #[cfg(all(feature = "v2", feature = "oltp"))]
        {
            server_app = server_app.service(routes::PaymentMethodsSession::server(state.clone()));
        }

        #[cfg(feature = "v1")]
        {
            server_app = server_app
                .service(routes::Refunds::server(state.clone()))
                .service(routes::Mandates::server(state.clone()));
        }
    }

    #[cfg(all(feature = "oltp", any(feature = "v1", feature = "v2"),))]
    {
        server_app = server_app.service(routes::EphemeralKey::server(state.clone()))
    }
    #[cfg(all(
        feature = "oltp",
        any(feature = "v1", feature = "v2"),
        not(feature = "customer_v2")
    ))]
    {
        server_app = server_app.service(routes::Poll::server(state.clone()))
    }

    #[cfg(feature = "olap")]
    {
        server_app = server_app
            .service(routes::Organization::server(state.clone()))
            .service(routes::MerchantAccount::server(state.clone()))
            .service(routes::ApiKeys::server(state.clone()))
            .service(routes::Routing::server(state.clone()));

        #[cfg(feature = "v1")]
        {
            server_app = server_app
                .service(routes::Files::server(state.clone()))
                .service(routes::Disputes::server(state.clone()))
                .service(routes::Blocklist::server(state.clone()))
                .service(routes::Gsm::server(state.clone()))
                .service(routes::ApplePayCertificatesMigration::server(state.clone()))
                .service(routes::PaymentLink::server(state.clone()))
                .service(routes::User::server(state.clone()))
                .service(routes::ConnectorOnboarding::server(state.clone()))
                .service(routes::Verify::server(state.clone()))
                .service(routes::Analytics::server(state.clone()))
                .service(routes::WebhookEvents::server(state.clone()))
                .service(routes::FeatureMatrix::server(state.clone()));
        }
    }

    #[cfg(all(feature = "payouts", feature = "v1"))]
    {
        server_app = server_app
            .service(routes::Payouts::server(state.clone()))
            .service(routes::PayoutLink::server(state.clone()));
    }

    #[cfg(all(
        feature = "stripe",
        any(feature = "v1", feature = "v2"),
        not(feature = "customer_v2")
    ))]
    {
        server_app = server_app
            .service(routes::StripeApis::server(state.clone()))
            .service(routes::Cards::server(state.clone()));
    }

    #[cfg(all(feature = "recon", feature = "v1"))]
    {
        server_app = server_app.service(routes::Recon::server(state.clone()));
    }

    server_app = server_app.service(routes::Cache::server(state.clone()));
    server_app = server_app.service(routes::Health::server(state.clone()));

    server_app
}

/// Starts the server
///
/// # Panics
///
///  Unwrap used because without the value we can't start the server
#[allow(clippy::expect_used, clippy::unwrap_used)]
pub async fn start_server(conf: settings::Settings<SecuredSecret>) -> ApplicationResult<Server> {
    logger::debug!(startup_config=?conf);
    let server = conf.server.clone();
    let (tx, rx) = oneshot::channel();
    let api_client = Box::new(services::ProxyClient::new(&conf.proxy).map_err(|error| {
        errors::ApplicationError::ApiClientError(error.current_context().clone())
    })?);
    let state = Box::pin(AppState::new(conf, tx, api_client)).await;
    let request_body_limit = server.request_body_limit;

    let server_builder =
        actix_web::HttpServer::new(move || mk_app(state.clone(), request_body_limit))
            .bind((server.host.as_str(), server.port))?
            .workers(server.workers)
            .shutdown_timeout(server.shutdown_timeout);

    #[cfg(feature = "tls")]
    let server = match server.tls {
        None => server_builder.run(),
        Some(tls_conf) => {
            let cert_file =
                &mut std::io::BufReader::new(std::fs::File::open(tls_conf.certificate).map_err(
                    |err| errors::ApplicationError::InvalidConfigurationValueError(err.to_string()),
                )?);
            let key_file =
                &mut std::io::BufReader::new(std::fs::File::open(tls_conf.private_key).map_err(
                    |err| errors::ApplicationError::InvalidConfigurationValueError(err.to_string()),
                )?);

            let cert_chain = rustls_pemfile::certs(cert_file)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| {
                    errors::ApplicationError::InvalidConfigurationValueError(err.to_string())
                })?;

            let mut keys = rustls_pemfile::pkcs8_private_keys(key_file)
                .map(|key| key.map(rustls::pki_types::PrivateKeyDer::Pkcs8))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| {
                    errors::ApplicationError::InvalidConfigurationValueError(err.to_string())
                })?;

            // exit if no keys could be parsed
            if keys.is_empty() {
                return Err(errors::ApplicationError::InvalidConfigurationValueError(
                    "Could not locate PKCS8 private keys.".into(),
                ));
            }

            let config_builder = rustls::ServerConfig::builder().with_no_client_auth();
            let config = config_builder
                .with_single_cert(cert_chain, keys.remove(0))
                .map_err(|err| {
                    errors::ApplicationError::InvalidConfigurationValueError(err.to_string())
                })?;

            server_builder
                .bind_rustls_0_22(
                    (tls_conf.host.unwrap_or(server.host).as_str(), tls_conf.port),
                    config,
                )?
                .run()
        }
    };

    #[cfg(not(feature = "tls"))]
    let server = server_builder.run();

    let _task_handle = tokio::spawn(receiver_for_error(rx, server.handle()).in_current_span());
    Ok(server)
}

pub async fn receiver_for_error(rx: oneshot::Receiver<()>, mut server: impl Stop) {
    match rx.await {
        Ok(_) => {
            logger::error!("The redis server failed ");
            server.stop_server().await;
        }
        Err(err) => {
            logger::error!("Channel receiver error: {err}");
        }
    }
}

#[async_trait::async_trait]
pub trait Stop {
    async fn stop_server(&mut self);
}

#[async_trait::async_trait]
impl Stop for ServerHandle {
    async fn stop_server(&mut self) {
        let _ = self.stop(true).await;
    }
}
#[async_trait::async_trait]
impl Stop for mpsc::Sender<()> {
    async fn stop_server(&mut self) {
        let _ = self.send(()).await.map_err(|err| logger::error!("{err}"));
    }
}

pub fn get_application_builder(
    request_body_limit: usize,
    cors: settings::CorsSettings,
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
        .wrap(cors::cors(cors))
        // this middleware works only for Http1.1 requests
        .wrap(middleware::Http400RequestDetailsLogger)
        .wrap(middleware::AddAcceptLanguageHeader)
        .wrap(middleware::LogSpanInitializer)
        .wrap(router_env::tracing_actix_web::TracingLogger::default())
}
