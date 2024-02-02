use std::sync::Arc;

use common_utils::errors::CustomResult;
use diesel_models::enums::ProcessTrackerStatus;
use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};
use time::Duration;
use tokio::sync::mpsc;

use super::{
    env::logger::{self, debug, error, warn},
    metrics,
};
use crate::{
    configs::settings::SchedulerSettings, errors, flow::SchedulerFlow,
    scheduler::SchedulerInterface, utils::*, SchedulerAppState,
};

#[instrument(skip_all)]
/// This method starts a producer with the given state and scheduler settings. It uses a random interval to run the producer flow and handles any errors that occur within the flow. It also listens for shutdown signals and terminates the producer accordingly.
pub async fn start_producer<T>(
    state: &T,
    scheduler_settings: Arc<SchedulerSettings>,
    (tx, mut rx): (mpsc::Sender<()>, mpsc::Receiver<()>),
) -> CustomResult<(), errors::ProcessTrackerError>
where
    T: SchedulerAppState,
{
    use std::time::Duration;

    use rand::distributions::{Distribution, Uniform};

    let mut rng = rand::thread_rng();

    // TODO: this can be removed once rand-0.9 is released
    // reference - https://github.com/rust-random/rand/issues/1326#issuecomment-1635331942
    #[allow(unknown_lints)]
    #[allow(clippy::unnecessary_fallible_conversions)]
    let timeout = Uniform::try_from(0..=scheduler_settings.loop_interval)
        .into_report()
        .change_context(errors::ProcessTrackerError::ConfigurationError)?;

    tokio::time::sleep(Duration::from_millis(timeout.sample(&mut rng))).await;

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
/// Asynchronously runs the producer flow for the scheduler application, using the provided state and settings.
/// This method acquires a lock on the scheduler's database, fetches producer tasks, divides and appends the tasks
/// to the scheduler, and releases the lock. It returns a CustomResult indicating success or a ProcessTrackerError
/// in case of an error.
pub async fn run_producer_flow<T>(
    state: &T,
    settings: &SchedulerSettings,
) -> CustomResult<(), errors::ProcessTrackerError>
where
    T: SchedulerAppState,
{
    lock_acquire_release::<_, _, _>(state.get_db().as_scheduler(), settings, move || async {
        let tasks = fetch_producer_tasks(state.get_db().as_scheduler(), settings).await?;
        debug!("Producer count of tasks {}", tasks.len());

        // [#268]: Allow task based segregation of tasks

        divide_and_append_tasks(
            state.get_db().as_scheduler(),
            SchedulerFlow::Producer,
            tasks,
            settings,
        )
        .await?;

        Ok(())
    })
    .await?;

    Ok(())
}

#[instrument(skip_all)]
/// Asynchronously fetches producer tasks from the database based on the provided scheduler settings.
/// 
/// # Arguments
/// 
/// * `db` - A reference to a trait object implementing the `SchedulerInterface` trait.
/// * `conf` - A reference to the `SchedulerSettings` struct containing configuration settings.
/// 
/// # Returns
/// 
/// A `CustomResult` containing a vector of `ProcessTracker` instances, or a `ProcessTrackerError` in case of failure.
pub async fn fetch_producer_tasks(
    db: &dyn SchedulerInterface,
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
