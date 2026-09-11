pub mod alert_manager;
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

pub type Settings = settings::Settings<RawSecret>;

pub async fn start_server(state: AppState) -> errors::ObservabilityResult<Server> {
    let server = state.conf.server.clone();

    let web_server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .service(routes::Health::server(state.clone()))
            .service(routes::Alerts::server(state.clone()))
            .wrap(router_env::tracing_actix_web::TracingLogger::<
                router_env::CustomRootSpanBuilder,
            >::new())
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
