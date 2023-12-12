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

use diesel_models::kv;
use router_env::{instrument, tracing};

use crate::{
    connection::pg_connection, services::Store, settings::DrainerSettings, types::StreamData,
};

pub async fn start_drainer(store: Arc<Store>, conf: DrainerSettings) -> errors::DrainerResult<()> {
    let mut drainer_handler = handler::Handler::from_conf(conf, store);

    drainer_handler.spawn_error_handlers()?;
    drainer_handler.spawn().await?;

    Ok(())
}
