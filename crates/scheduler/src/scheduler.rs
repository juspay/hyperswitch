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
    fn get_tenants(&self) -> Vec<String>;
}

pub async fn start_process_tracker<
    T: SchedulerAppState + 'static,
    U: SchedulerAppState + 'static,
    F,
>(
    state: &T,
    scheduler_flow: SchedulerFlow,
    scheduler_settings: Arc<SchedulerSettings>,
    channel: (mpsc::Sender<()>, mpsc::Receiver<()>),
    runner_from_task: impl workflows::ProcessTrackerWorkflows<U> + 'static + Copy + std::fmt::Debug,
    app_state_to_session_state: F,
) -> CustomResult<(), errors::ProcessTrackerError>
where
    F: Fn(&T, &str) -> U,
{
    match scheduler_flow {
        SchedulerFlow::Producer => {
            producer::start_producer(
                state,
                scheduler_settings,
                channel,
                app_state_to_session_state,
            )
            .await?
        }
        SchedulerFlow::Consumer => {
            consumer::start_consumer(
                state,
                scheduler_settings,
                runner_from_task,
                channel,
                app_state_to_session_state,
            )
            .await?
        }
        SchedulerFlow::Cleaner => {
            error!("This flow has not been implemented yet!");
        }
    }
    Ok(())
}
