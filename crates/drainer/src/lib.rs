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
use router_env::{instrument, tracing};
use tokio::sync::mpsc;

pub(crate) type Settings = crate::settings::Settings<RawSecret>;

use crate::{
    connection::pg_connection,
    services::Store,
    settings::DrainerSettings,
    stream::{DrainerErrorStream, DrainerStream},
    types::StreamData,
};

pub async fn start_drainer(store: Arc<Store>, conf: DrainerSettings) -> errors::DrainerResult<()> {
    let drainer_handler = handler::Handler::from_conf(conf.clone(), DrainerStream(store.clone()));

    let drainer_error_handler = handler::Handler::from_conf(conf, DrainerErrorStream(store));

    let (tx, rx) = mpsc::channel::<()>(1);

    let signal =
        get_allowed_signals()
            .into_report()
            .change_context(errors::DrainerError::SignalError(
                "Failed while getting allowed signals".to_string(),
            ))?;
    let handle = signal.handle();
    let task_handle = tokio::spawn(common_utils::signals::signal_handler(signal, tx.clone()));

    let handler_clone = drainer_handler.clone();
    let error_handler_clone = drainer_error_handler.clone();

    let (err_tx, err_rx) = mpsc::channel::<()>(1);

    let signal =
        get_allowed_signals()
            .into_report()
            .change_context(errors::DrainerError::SignalError(
                "Failed while getting allowed signals".to_string(),
            ))?;
    let err_handle = signal.handle();
    let err_task_handle = tokio::spawn(common_utils::signals::signal_handler(
        signal,
        err_tx.clone(),
    ));

    tokio::task::spawn(async move { handler_clone.shutdown_listener(rx).await });
    tokio::task::spawn(async move { error_handler_clone.shutdown_listener(err_rx).await });

    drainer_handler.spawn_error_handlers(tx)?;
    drainer_error_handler.spawn_error_handlers(err_tx)?;

    tokio::task::spawn(async move { drainer_error_handler.spawn().await });

    drainer_handler.spawn().await?;

    handle.close();
    err_handle.close();

    let _ = task_handle
        .await
        .map_err(|err| logger::error!("Failed while joining signal handler: {:?}", err));

    let _ = err_task_handle
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
