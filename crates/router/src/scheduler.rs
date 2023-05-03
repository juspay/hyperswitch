#![allow(dead_code)]

pub mod consumer;
pub mod metrics;
pub mod producer;
pub mod types;
pub mod utils;
pub mod workflows;

use std::sync::Arc;

use tokio::sync::mpsc;

pub use self::types::*;
use crate::{
    configs::settings::SchedulerSettings,
    core::errors::{self, CustomResult},
    logger::error,
    routes::AppState,
};

pub async fn start_process_tracker(
    state: &AppState,
    scheduler_flow: SchedulerFlow,
    scheduler_settings: Arc<SchedulerSettings>,
    channel: (mpsc::Sender<()>, mpsc::Receiver<()>),
) -> CustomResult<(), errors::ProcessTrackerError> {
    match scheduler_flow {
        SchedulerFlow::Producer => {
            producer::start_producer(state, scheduler_settings, channel).await?
        }
        SchedulerFlow::Consumer => {
            consumer::start_consumer(
                state,
                scheduler_settings,
                workflows::runner_from_task,
                channel,
            )
            .await?
        }
        SchedulerFlow::Cleaner => {
            error!("This flow has not been implemented yet!");
        }
    }
    Ok(())
}
