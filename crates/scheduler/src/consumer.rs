// TODO: Figure out what to log

use std::sync::{self, atomic};
pub mod types;
pub mod workflows;

use common_utils::{errors::CustomResult, signals::get_allowed_signals};
use diesel_models::enums;
pub use diesel_models::{self, process_tracker as storage};
use error_stack::{IntoReport, ResultExt};
use futures::future;
use redis_interface::{RedisConnectionPool, RedisEntryId};
use router_env::{
    instrument,
    tracing::{self, Instrument},
};
use time::PrimitiveDateTime;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::env::logger;
pub use super::workflows::ProcessTrackerWorkflow;
use crate::{
    configs::settings::SchedulerSettings, db::process_tracker::ProcessTrackerInterface, errors,
    metrics, utils as pt_utils, SchedulerAppState, SchedulerInterface,
};

// Valid consumer business statuses
pub fn valid_business_statuses() -> Vec<&'static str> {
    vec!["Pending"]
}

#[instrument(skip_all)]
pub async fn start_consumer<T: SchedulerAppState + 'static>(
    state: &T,
    settings: sync::Arc<SchedulerSettings>,
    workflow_selector: impl workflows::ProcessTrackerWorkflows<T> + 'static + Copy + std::fmt::Debug,
    (tx, mut rx): (mpsc::Sender<()>, mpsc::Receiver<()>),
) -> CustomResult<(), errors::ProcessTrackerError> {
    use std::time::Duration;

    use rand::distributions::{Distribution, Uniform};

    let mut rng = rand::thread_rng();

    // TODO: this can be removed once rand-0.9 is released
    // reference - https://github.com/rust-random/rand/issues/1326#issuecomment-1635331942
    #[allow(unknown_lints)]
    #[allow(clippy::unnecessary_fallible_conversions)]
    let timeout = Uniform::try_from(0..=settings.loop_interval)
        .into_report()
        .change_context(errors::ProcessTrackerError::ConfigurationError)?;

    tokio::time::sleep(Duration::from_millis(timeout.sample(&mut rng))).await;

    let mut interval = tokio::time::interval(Duration::from_millis(settings.loop_interval));

    let mut shutdown_interval =
        tokio::time::interval(Duration::from_millis(settings.graceful_shutdown_interval));

    let consumer_operation_counter = sync::Arc::new(atomic::AtomicU64::new(0));
    let signal = get_allowed_signals()
        .map_err(|error| {
            logger::error!(?error, "Signal Handler Error");
            errors::ProcessTrackerError::ConfigurationError
        })
        .into_report()
        .attach_printable("Failed while creating a signals handler")?;
    let handle = signal.handle();
    let task_handle =
        tokio::spawn(common_utils::signals::signal_handler(signal, tx).in_current_span());

    'consumer: loop {
        match rx.try_recv() {
            Err(mpsc::error::TryRecvError::Empty) => {
                interval.tick().await;

                // A guard from env to disable the consumer
                if settings.consumer.disabled {
                    continue;
                }

                pt_utils::consumer_operation_handler(
                    state.clone(),
                    settings.clone(),
                    |error| {
                        logger::error!(?error, "Failed to perform consumer operation");
                    },
                    sync::Arc::clone(&consumer_operation_counter),
                    workflow_selector,
                )
                .await;
            }
            Ok(()) | Err(mpsc::error::TryRecvError::Disconnected) => {
                logger::debug!("Awaiting shutdown!");
                rx.close();
                loop {
                    shutdown_interval.tick().await;
                    let active_tasks = consumer_operation_counter.load(atomic::Ordering::Acquire);
                    logger::info!("Active tasks: {active_tasks}");
                    match active_tasks {
                        0 => {
                            logger::info!("Terminating consumer");
                            break 'consumer;
                        }
                        _ => continue,
                    }
                }
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
pub async fn consumer_operations<T: SchedulerAppState + 'static>(
    state: &T,
    settings: &SchedulerSettings,
    workflow_selector: impl workflows::ProcessTrackerWorkflows<T> + 'static + Copy + std::fmt::Debug,
) -> CustomResult<(), errors::ProcessTrackerError> {
    let stream_name = settings.stream.clone();
    let group_name = settings.consumer.consumer_group.clone();
    let consumer_name = format!("consumer_{}", Uuid::new_v4());

    let group_created = &mut state
        .get_db()
        .consumer_group_create(&stream_name, &group_name, &RedisEntryId::AfterLastID)
        .await;
    if group_created.is_err() {
        logger::info!("Consumer group {group_name} already exists");
    }

    let mut tasks = state
        .get_db()
        .as_scheduler()
        .fetch_consumer_tasks(&stream_name, &group_name, &consumer_name)
        .await?;

    logger::info!("{} picked {} tasks", consumer_name, tasks.len());
    let mut handler = vec![];

    for task in tasks.iter_mut() {
        let pickup_time = common_utils::date_time::now();

        pt_utils::add_histogram_metrics(&pickup_time, task, &stream_name);

        metrics::TASK_CONSUMED.add(&metrics::CONTEXT, 1, &[]);

        handler.push(tokio::task::spawn(start_workflow(
            state.clone(),
            task.clone(),
            pickup_time,
            workflow_selector,
        )))
    }
    future::join_all(handler).await;

    Ok(())
}

#[instrument(skip(db, redis_conn))]
pub async fn fetch_consumer_tasks(
    db: &dyn ProcessTrackerInterface,
    redis_conn: &RedisConnectionPool,
    stream_name: &str,
    group_name: &str,
    consumer_name: &str,
) -> CustomResult<Vec<storage::ProcessTracker>, errors::ProcessTrackerError> {
    let batches = pt_utils::get_batches(redis_conn, stream_name, group_name, consumer_name).await?;

    // Returning early to avoid execution of database queries when `batches` is empty
    if batches.is_empty() {
        return Ok(Vec::new());
    }

    let mut tasks = batches.into_iter().fold(Vec::new(), |mut acc, batch| {
        acc.extend_from_slice(
            batch
                .trackers
                .into_iter()
                .filter(|task| task.is_valid_business_status(&valid_business_statuses()))
                .collect::<Vec<_>>()
                .as_slice(),
        );
        acc
    });
    let task_ids = tasks
        .iter()
        .map(|task| task.id.to_owned())
        .collect::<Vec<_>>();

    db.process_tracker_update_process_status_by_ids(
        task_ids,
        storage::ProcessTrackerUpdate::StatusUpdate {
            status: enums::ProcessTrackerStatus::ProcessStarted,
            business_status: None,
        },
    )
    .await
    .change_context(errors::ProcessTrackerError::ProcessFetchingFailed)?;
    tasks
        .iter_mut()
        .for_each(|x| x.status = enums::ProcessTrackerStatus::ProcessStarted);
    Ok(tasks)
}

// Accept flow_options if required
#[instrument(skip(state), fields(workflow_id))]
pub async fn start_workflow<T>(
    state: T,
    process: storage::ProcessTracker,
    _pickup_time: PrimitiveDateTime,
    workflow_selector: impl workflows::ProcessTrackerWorkflows<T> + 'static + std::fmt::Debug,
) -> CustomResult<(), errors::ProcessTrackerError>
where
    T: SchedulerAppState,
{
    tracing::Span::current().record("workflow_id", Uuid::new_v4().to_string());
    logger::info!(pt.name=?process.name, pt.id=%process.id);

    let res = workflow_selector
        .trigger_workflow(&state.clone(), process.clone())
        .await
        .map_err(|error| {
            logger::error!(?error, "Failed to trigger workflow");
            error
        });
    metrics::TASK_PROCESSED.add(&metrics::CONTEXT, 1, &[]);
    res
}

#[instrument(skip_all)]
pub async fn consumer_error_handler(
    state: &(dyn SchedulerInterface + 'static),
    process: storage::ProcessTracker,
    error: errors::ProcessTrackerError,
) -> CustomResult<(), errors::ProcessTrackerError> {
    logger::error!(pt.name=?process.name, pt.id=%process.id, ?error, "Failed to execute workflow");

    state
        .process_tracker_update_process_status_by_ids(
            vec![process.id],
            storage::ProcessTrackerUpdate::StatusUpdate {
                status: enums::ProcessTrackerStatus::Finish,
                business_status: Some("GLOBAL_ERROR".to_string()),
            },
        )
        .await
        .change_context(errors::ProcessTrackerError::ProcessUpdateFailed)?;
    Ok(())
}

pub async fn create_task(
    db: &dyn ProcessTrackerInterface,
    process_tracker_entry: storage::ProcessTrackerNew,
) -> CustomResult<(), storage_impl::errors::StorageError> {
    db.insert_process(process_tracker_entry).await?;
    Ok(())
}
