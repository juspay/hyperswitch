use std::sync::Arc;

use common_utils::errors::CustomResult;
use storage_impl::mock_db::MockDb;
#[cfg(feature = "kv_store")]
use storage_impl::KVRouterStore;
#[cfg(not(feature = "kv_store"))]
use storage_impl::RouterStore;
use tokio::sync::mpsc;

use super::env::logger::error;
pub use crate::{
    configs::settings::SchedulerSettings,
    consumer::{self, workflows},
    db::{process_tracker::ProcessTrackerInterface, queue::QueueInterface},
    errors,
    flow::SchedulerFlow,
    producer,
};

#[cfg(not(feature = "olap"))]
type StoreType = storage_impl::database::store::Store;
#[cfg(feature = "olap")]
type StoreType = storage_impl::database::store::ReplicaStore;

#[cfg(not(feature = "kv_store"))]
pub type Store = RouterStore<StoreType>;
#[cfg(feature = "kv_store")]
pub type Store = KVRouterStore<StoreType>;

pub trait AsSchedulerInterface {
    fn as_scheduler(&self) -> &dyn SchedulerInterface;
}

impl<T: SchedulerInterface> AsSchedulerInterface for T {
        /// This method returns a reference to the trait object `SchedulerInterface` by borrowing `self`.
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
pub trait SchedulerAppState: Send + Sync + Clone {
    fn get_db(&self) -> Box<dyn SchedulerInterface>;
}

/// This method is used to start the process tracker for a given scheduler flow. It takes in the scheduler's state, the scheduler flow type, scheduler settings, a channel for communication, and a runner for task processing. Depending on the scheduler flow type, it either starts a producer, consumer, or logs an error message for an unimplemented flow. The method returns a result indicating success or an error related to the process tracker operation.
pub async fn start_process_tracker<T: SchedulerAppState + 'static>(
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
