use std::sync::Arc;

use tokio::sync::mpsc;

use crate::{
    consumer::{
        consumer::{self},
        workflows,
    },
    producer, flow::SchedulerFlow, settings::SchedulerSettings,
};
use router::{
    core::errors::{self, CustomResult},
    db::StorageInterface,
    logger::error, routes::AppState
};

pub trait Storable {
    fn get_store(&self) -> Box<dyn StorageInterface>;
}

impl Storable for AppState {
    fn get_store(&self) -> Box<dyn StorageInterface> {
        self.clone().store
    }
}

pub async fn start_process_tracker<T: Storable + Send + Sync + Clone + 'static>(
    state: &T,
    scheduler_flow: SchedulerFlow,
    scheduler_settings: Arc<SchedulerSettings>,
    channel: (mpsc::Sender<()>, mpsc::Receiver<()>),
) -> CustomResult<(), errors::ProcessTrackerError> where workflows::PaymentsSyncWorkflow: workflows::ProcessTrackerWorkflow<T> {
    match scheduler_flow {
        SchedulerFlow::Producer => {
            producer::start_producer(state, scheduler_settings, channel).await?
        }
        SchedulerFlow::Consumer => {
            consumer::start_consumer(
                state,
                scheduler_settings,
                workflows::runner_from_task::<T>,
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
