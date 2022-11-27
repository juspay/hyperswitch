use std::sync::Arc;

use error_stack::{report, ResultExt};
use router_env::{tracing, tracing::instrument};
use time::Duration;

use super::metrics;
use crate::{
    configs::settings::SchedulerSettings,
    core::errors::{self, CustomResult},
    db::Db,
    logger::{debug, error, info, warn},
    routes::AppState,
    scheduler::{utils::*, SchedulerFlow, SchedulerOptions},
    types::storage::{self, enums::ProcessTrackerStatus},
    utils,
};

#[instrument(skip_all)]
pub async fn start_producer(
    state: &AppState,
    options: Arc<SchedulerOptions>,
    scheduler_settings: Arc<SchedulerSettings>,
) -> CustomResult<(), errors::ProcessTrackerError> {
    use rand::Rng;

    let timeout = rand::thread_rng().gen_range(0..=options.looper_interval.milliseconds);
    tokio::time::sleep(std::time::Duration::from_millis(timeout)).await;

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(
        options.looper_interval.milliseconds,
    ));

    loop {
        interval.tick().await;

        let is_ready = options.readiness.is_ready;
        if is_ready {
            match run_producer_flow(state, &options, &scheduler_settings).await {
                Ok(_) => (),
                Err(error) => {
                    // Intentionally not propagating error to caller.
                    // Any errors that occur in the producer flow must be handled here only, as
                    // this is the topmost level function which is concerned with the producer flow.
                    error!(%error);
                }
            }
        } else {
            // TODO: Handle termination?
            info!("Terminating producer");
            break;
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn run_producer_flow(
    state: &AppState,
    op: &SchedulerOptions,
    settings: &SchedulerSettings,
) -> CustomResult<(), errors::ProcessTrackerError> {
    let tag = "PRODUCER_LOCK";
    let lock_key = &settings.producer.lock_key;
    let lock_val = "LOCKED";
    let ttl = settings.producer.lock_ttl;

    // TODO: Pass callback function to acquire_pt_lock() to run after acquiring lock
    if acquire_pt_lock(state, tag, lock_key, lock_val, ttl).await {
        let tasks = fetch_producer_tasks(&state.store, op, settings).await?;
        debug!("Producer count of tasks {}", tasks.len());
        //TODO based on pt.name decide which pt goes to which stream
        // (LIVE_TRAFFIC_STRM,SCHEDULER_STREAM); array of [(stream,Vec<ProcessTracker>)]
        divide_and_append_tasks(state, SchedulerFlow::Producer, tasks, settings).await?;
        release_pt_lock(&state.store.redis_conn, tag, lock_key).await;
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn fetch_producer_tasks(
    db: &dyn Db,
    _options: &SchedulerOptions,
    conf: &SchedulerSettings,
) -> CustomResult<Vec<storage::ProcessTracker>, errors::ProcessTrackerError> {
    let upper = conf.producer.upper_fetch_limit;
    let lower = conf.producer.lower_fetch_limit;
    let now = utils::date_time::now();
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
    metrics::TASKS_PICKED_COUNT.add(new_tasks.len() as u64, &[]);
    Ok(new_tasks)
}
