#![allow(dead_code)]

pub mod consumer;
pub mod metrics;
pub mod producer;
pub mod types;
pub mod utils;
pub mod workflows;

use std::sync::Arc;

pub use self::types::*;
use crate::{
    configs::settings::SchedulerSettings,
    core::errors::{self, CustomResult},
    logger::error,
    routes::AppState,
};

pub async fn start_process_tracker(
    state: &AppState,
    options: Arc<SchedulerOptions>,
    scheduler_flow: SchedulerFlow,
    scheduler_settings: Arc<SchedulerSettings>,
) -> CustomResult<(), errors::ProcessTrackerError> {
    match scheduler_flow {
        SchedulerFlow::Producer => {
            producer::start_producer(state, Arc::clone(&options), scheduler_settings).await?
        }
        SchedulerFlow::Consumer => {
            consumer::start_consumer(state, Arc::clone(&options), scheduler_settings).await?
        }
        SchedulerFlow::Cleaner => {
            error!("This flow has not been implemented yet!");
        }
    }
    Ok(())
}
