mod connection;
pub mod errors;
mod handler;
pub mod logger;
pub(crate) mod metrics;
mod query;
pub mod services;
pub mod settings;
mod stream;
mod types;
mod utils;
use std::sync::Arc;

use common_utils::signals::get_allowed_signals;
use diesel_models::kv;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};
use tokio::sync::mpsc;

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
    let task_handle = tokio::spawn(common_utils::signals::signal_handler(signal, tx.clone()));

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
