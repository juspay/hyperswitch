mod connection;
pub mod errors;
mod handler;
mod health_check;
pub mod logger;
pub(crate) mod metrics;
mod query;
pub mod services;
pub mod settings;
mod stream;
mod types;
mod utils;
use std::sync::Arc;
mod secrets_transformers;

use actix_web::dev::Server;
use common_utils::signals::get_allowed_signals;
use diesel_models::kv;
use error_stack::{IntoReport, ResultExt};
use hyperswitch_interfaces::secrets_interface::secret_state::RawSecret;
use router_env::{
    instrument,
    tracing::{self, Instrument},
};
use tokio::sync::mpsc;

pub(crate) type Settings = crate::settings::Settings<RawSecret>;

use crate::{
    connection::pg_connection, services::Store, settings::DrainerSettings, types::StreamData,
};

pub async fn start_drainer(store: Arc<Store>, conf: DrainerSettings) -> errors::DrainerResult<()> {
    let drainer_handler = handler::Handler::from_conf(conf, store);

    let (tx, rx) = mpsc::channel::<()>(1);

    let signal =
        get_allowed_signals()
            .into_report()
            .change_context(errors::DrainerError::SignalError(
                "Failed while getting allowed signals".to_string(),
            ))?;
    let handle = signal.handle();
    let task_handle =
        tokio::spawn(common_utils::signals::signal_handler(signal, tx.clone()).in_current_span());

    let handler_clone = drainer_handler.clone();

    tokio::task::spawn(async move { handler_clone.shutdown_listener(rx).await });

    drainer_handler.spawn_error_handlers(tx)?;
    drainer_handler.spawn().await?;

    handle.close();
    let _ = task_handle
        .await
        .map_err(|err| logger::error!("Failed while joining signal handler: {:?}", err));

    Ok(())
}

pub async fn start_web_server(
    conf: Settings,
    store: Arc<Store>,
) -> Result<Server, errors::DrainerError> {
    let server = conf.server.clone();
    let web_server = actix_web::HttpServer::new(move || {
        actix_web::App::new().service(health_check::Health::server(conf.clone(), store.clone()))
    })
    .bind((server.host.as_str(), server.port))?
    .run();
    let _ = web_server.handle();

    Ok(web_server)
}
