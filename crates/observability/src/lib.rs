//! The observability plane for Hyperswitch.
//!
//! `observability` delivers alerts. Deciding what is alert-worthy happens elsewhere; alerts
//! arrive here already decided. Its first concern is [`core::notifier`], and further alerting
//! concerns are expected to live alongside it.
//!
//! Laid out on the router's lines: [`core`] decides, [`routes`] exposes, and the whole route tree
//! is visible in [`routes::app`].
//!
//! The crate ships two ways, on the `drainer` model: as its own binary, and as a library exposing
//! an actix [`Scope`](actix_web::Scope) the router can mount in-process. Only the standalone path
//! is wired up today — see [`start_server`] — but routes are defined as `Scope` factories
//! ([`routes::Alerts::server`], [`routes::Health::server`]) precisely so both paths will share one
//! definition rather than drifting.

pub mod auth;
pub mod core;
pub mod domain;
pub mod errors;
pub mod logger;
pub mod routes;
pub mod services;
pub mod settings;
pub mod state;
pub mod types;

mod secrets_transformers;

use actix_web::dev::Server;
use error_stack::ResultExt;
use hyperswitch_interfaces::secrets_interface::secret_state::RawSecret;

use crate::state::AppState;

/// The configuration, after secrets have been resolved.
pub type Settings = settings::Settings<RawSecret>;

/// Build the standalone HTTP server.
///
/// The request-id middleware and root-span logger are mounted here, in the standalone path only.
/// When the router mounts this crate it already has its own, and running two would produce two
/// competing ids for one request — which is why they belong on the server rather than on the
/// scope.
pub async fn start_server(state: AppState) -> errors::ObservabilityResult<Server> {
    let server = state.conf.server.clone();

    let web_server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .service(routes::Health::server())
            .service(routes::Alerts::server(state.clone()))
            // Order matters and is the reverse of what it reads like: actix runs the *last*
            // registered wrap first, so `RequestIdentifier` must be registered last to run first.
            // `CustomRootSpanBuilder` reads the request id out of request extensions, so if the
            // tracing logger ran first it would find nothing and every root span would carry an
            // empty `request_id`. This matches the router's ordering at `router/src/lib.rs:574`.
            .wrap(router_env::tracing_actix_web::TracingLogger::<
                router_env::CustomRootSpanBuilder,
            >::new())
            // `common_utils::consts::X_REQUEST_ID` rather than our own literal: the router
            // defaults its trace header to this same constant, so a request id set by an upstream
            // hop is the one we log rather than a second id for the same request.
            .wrap(router_env::RequestIdentifier::new(
                common_utils::consts::X_REQUEST_ID,
            ))
    })
    .bind((server.host.as_str(), server.port))
    .change_context(errors::ConfigurationError::ConfigParsingError(format!(
        "Failed to bind to {}:{}",
        server.host, server.port
    )))?
    .workers(server.workers)
    .run();

    Ok(web_server)
}
