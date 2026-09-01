//! The alerting plane for Hyperswitch.
//!
//! `alerts` delivers alerts. Deciding what is alert-worthy happens elsewhere; alerts arrive here
//! already decided. Its first concern is the [`notifier`], and further alerting concerns are
//! expected to live alongside it.
//!
//! The crate ships two ways, on the `drainer` model: as its own binary, and as a library exposing
//! an actix [`Scope`](actix_web::Scope) the router can mount in-process. Only the standalone path
//! is wired up today — see [`start_server`] — but routes are defined as `Scope` factories
//! ([`routes::Alerts::server`], [`health_check::Health::server`]) precisely so both paths will
//! share one definition rather than drifting.

pub mod auth;
pub mod errors;
pub mod health_check;
pub mod logger;
pub mod notifier;
pub mod routes;
pub mod services;
pub mod settings;
pub mod state;

mod secrets_transformers;

use actix_web::dev::Server;
use error_stack::ResultExt;
use hyperswitch_interfaces::secrets_interface::secret_state::RawSecret;

use crate::state::AppState;

/// The configuration, after secrets have been resolved.
pub type Settings = settings::Settings<RawSecret>;

/// The header carrying the request id, threaded through every log line for a request.
const REQUEST_ID_HEADER: &str = "x-request-id";

/// Build the standalone HTTP server.
///
/// The request-id middleware and root-span logger are mounted here, in the standalone path only.
/// When the router mounts this crate it already has its own, and running two would produce two
/// competing ids for one request — which is why they belong on the server rather than on the
/// scope.
pub async fn start_server(state: AppState) -> errors::AlertsResult<Server> {
    let server = state.conf.server.clone();

    let web_server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .service(health_check::Health::server())
            .service(routes::Alerts::server(state.clone()))
            .wrap(router_env::RequestIdentifier::new(REQUEST_ID_HEADER))
            .wrap(router_env::tracing_actix_web::TracingLogger::<
                router_env::CustomRootSpanBuilder,
            >::new())
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
