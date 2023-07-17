use std::sync::Arc;

use common_utils::errors::CustomResult;
use diesel_models::services::{MockDb, Store};
use tokio::sync::mpsc;

use super::env::logger::error;
pub use crate::{
    consumer::{self, workflows},
    db::{process_tracker::ProcessTrackerInterface, queue::QueueInterface},
    errors,
    flow::SchedulerFlow,
    producer,
    settings::SchedulerSettings,
};

pub trait AsSchedulerInterface {
    fn as_scheduler(&self) -> &dyn SchedulerInterface;
}

impl<T: SchedulerInterface> AsSchedulerInterface for T {
    fn as_scheduler(&self) -> &dyn SchedulerInterface {
        self
    }
}

#[async_trait::async_trait]
pub trait SchedulerInterface:
    ProcessTrackerInterface + QueueInterface + AsSchedulerInterface
{
}

#[async_trait::async_trait]
impl SchedulerInterface for Store {}

#[async_trait::async_trait]
impl SchedulerInterface for MockDb {}

#[async_trait::async_trait]
pub trait SchedulerAppState: Send + Sync {
    fn get_db(&self) -> Box<dyn SchedulerInterface>;
}

pub async fn start_process_tracker<T: SchedulerAppState + Send + Sync + Clone + 'static>(
    state: &T,
    scheduler_flow: SchedulerFlow,
    scheduler_settings: Arc<SchedulerSettings>,
    channel: (mpsc::Sender<()>, mpsc::Receiver<()>),
    runner_from_task: impl workflows::ProcessTrackerWorkflows<T> + 'static + Copy + std::fmt::Debug,
) -> CustomResult<(), errors::ProcessTrackerError> {
    match scheduler_flow {
        SchedulerFlow::Producer => {
            producer::start_producer(state, scheduler_settings, channel).await?
        }
        SchedulerFlow::Consumer => {
            consumer::start_consumer(state, scheduler_settings, runner_from_task, channel).await?
        }
        SchedulerFlow::Cleaner => {
            error!("This flow has not been implemented yet!");
        }
    }
    Ok(())
}
