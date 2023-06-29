use std::sync::Arc;

use common_utils::errors::CustomResult;
use storage_models::services::{Store, MockDb};
use tokio::sync::mpsc;

use crate::{
    consumer::{
        consumer::{self},
        workflows::{self},
    },
    producer, flow::SchedulerFlow, settings::SchedulerSettings, errors, db::{process_tracker::ProcessTrackerInterface, queue::QueueInterface},
};
use super::env::logger::error;


pub trait AsSchedulerInterface {
    fn as_scheduler(&self) -> &dyn SchedulerInterface;
}

impl<T: SchedulerInterface> AsSchedulerInterface for T {
    fn as_scheduler(&self) -> &dyn SchedulerInterface {
        self
    }
}

#[async_trait::async_trait]
pub trait SchedulerInterface: ProcessTrackerInterface + QueueInterface + AsSchedulerInterface {}

#[async_trait::async_trait]
impl SchedulerInterface for Store {}

#[async_trait::async_trait]
impl SchedulerInterface for MockDb {}

#[async_trait::async_trait]
pub trait SchedulerAppState<T> : Send + Sync   {
    fn get_db(&self) -> T where T : SchedulerInterface + Send + Sync;
}

pub async fn start_process_tracker<F, T: SchedulerAppState<F> + Send + Sync + Clone + 'static>(
    state: &T,
    scheduler_flow: SchedulerFlow,
    scheduler_settings: Arc<SchedulerSettings>,
    channel: (mpsc::Sender<()>, mpsc::Receiver<()>),
    runner_from_task: workflows::WorkflowSelectorFn<T>
) -> CustomResult<(), errors::ProcessTrackerError> where F: SchedulerInterface {
    match scheduler_flow {
        SchedulerFlow::Producer => {
            producer::start_producer(state, scheduler_settings, channel).await?
        }
        SchedulerFlow::Consumer => {
            consumer::start_consumer(
                state,
                scheduler_settings,
                runner_from_task,
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
