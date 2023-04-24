use std::sync::Arc;

use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};
use time::Duration;
use tokio::sync::mpsc;

use super::metrics;
use crate::{
    configs::settings::SchedulerSettings,
    core::errors::{self, CustomResult},
    db::StorageInterface,
    logger::{self, debug, error, warn},
    routes::AppState,
    scheduler::{utils::*, SchedulerFlow},
    types::storage::{self, enums::ProcessTrackerStatus},
};

#[instrument(skip_all)]
pub async fn start_producer(
    state: &AppState,
    scheduler_settings: Arc<SchedulerSettings>,
    (tx, mut rx): (mpsc::Sender<()>, mpsc::Receiver<()>),
) -> CustomResult<(), errors::ProcessTrackerError> {
    use rand::Rng;
    let timeout = rand::thread_rng().gen_range(0..=scheduler_settings.loop_interval);
    tokio::time::sleep(std::time::Duration::from_millis(timeout)).await;

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(
        scheduler_settings.loop_interval,
    ));

    let mut shutdown_interval = tokio::time::interval(std::time::Duration::from_millis(
        scheduler_settings.graceful_shutdown_interval,
    ));

    let signal = common_utils::signals::get_allowed_signals()
        .map_err(|error| {
            logger::error!("Signal Handler Error: {:?}", error);
            errors::ProcessTrackerError::ConfigurationError
        })
        .into_report()
        .attach_printable("Failed while creating a signals handler")?;
    let handle = signal.handle();
    let task_handle = tokio::spawn(common_utils::signals::signal_handler(signal, tx));

    loop {
        match rx.try_recv() {
            Err(mpsc::error::TryRecvError::Empty) => {
                interval.tick().await;
                match run_producer_flow(state, &scheduler_settings).await {
                    Ok(_) => (),
                    Err(error) => {
                        // Intentionally not propagating error to caller.
                        // Any errors that occur in the producer flow must be handled here only, as
                        // this is the topmost level function which is concerned with the producer flow.
                        error!(%error);
                    }
                }
            }
            Ok(()) | Err(mpsc::error::TryRecvError::Disconnected) => {
                logger::debug!("Awaiting shutdown!");
                rx.close();
                shutdown_interval.tick().await;
                logger::info!("Terminating consumer");
                break;
            }
        }
    }
    handle.close();
    task_handle
        .await
        .into_report()
        .change_context(errors::ProcessTrackerError::UnexpectedFlow)?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn run_producer_flow(
    state: &AppState,
    settings: &SchedulerSettings,
) -> CustomResult<(), errors::ProcessTrackerError> {
    lock_acquire_release::<_, _>(state, settings, move || async {
        let tasks = fetch_producer_tasks(&*state.store, settings).await?;
        debug!("Producer count of tasks {}", tasks.len());

        // [#268]: Allow task based segregation of tasks

        divide_and_append_tasks(state, SchedulerFlow::Producer, tasks, settings).await?;

        Ok(())
    })
    .await?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn fetch_producer_tasks(
    db: &dyn StorageInterface,
    conf: &SchedulerSettings,
) -> CustomResult<Vec<storage::ProcessTracker>, errors::ProcessTrackerError> {
    let upper = conf.producer.upper_fetch_limit;
    let lower = conf.producer.lower_fetch_limit;
    let now = common_utils::date_time::now();
    // Add these to validations
    let time_upper_limit = now.checked_add(Duration::seconds(upper)).ok_or_else(|| {
        report!(errors::ProcessTrackerError::ConfigurationError)
            .attach_printable("Error obtaining upper limit to fetch producer tasks")
    })?;
    let time_lower_limit = now.checked_sub(Duration::seconds(lower)).ok_or_else(|| {
        report!(errors::ProcessTrackerError::ConfigurationError)
            .attach_printable("Error obtaining lower limit to fetch producer tasks")
    })?;

    let mut new_tasks = db
        .find_processes_by_time_status(
            time_lower_limit,
            time_upper_limit,
            ProcessTrackerStatus::New,
            None,
        )
        .await
        .change_context(errors::ProcessTrackerError::ProcessFetchingFailed)?;
    let mut pending_tasks = db
        .find_processes_by_time_status(
            time_lower_limit,
            time_upper_limit,
            ProcessTrackerStatus::Pending,
            None,
        )
        .await
        .change_context(errors::ProcessTrackerError::ProcessFetchingFailed)?;

    if new_tasks.is_empty() {
        warn!("No new tasks found for producer to schedule");
    }
    if pending_tasks.is_empty() {
        warn!("No pending tasks found for producer to schedule");
    }

    new_tasks.append(&mut pending_tasks);

    // Safety: Assuming we won't deal with more than `u64::MAX` tasks at once
    #[allow(clippy::as_conversions)]
    metrics::TASKS_PICKED_COUNT.add(&metrics::CONTEXT, new_tasks.len() as u64, &[]);
    Ok(new_tasks)
}
