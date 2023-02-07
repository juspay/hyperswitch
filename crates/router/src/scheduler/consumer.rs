// TODO: Figure out what to log

use std::{
    fmt,
    sync::{self, atomic},
};

use common_utils::signals::{get_allowed_signals, oneshot};
use error_stack::{IntoReport, ResultExt};
use futures::future;
use redis_interface::{RedisConnectionPool, RedisEntryId};
use router_env::{instrument, tracing};
use time::PrimitiveDateTime;
use uuid::Uuid;

use super::{
    metrics,
    workflows::{self, ProcessTrackerWorkflow},
};
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    db::StorageInterface,
    logger,
    routes::AppState,
    scheduler::utils as pt_utils,
    types::storage::{self, enums, ProcessTrackerExt},
};

// Valid consumer business statuses
pub fn valid_business_statuses() -> Vec<&'static str> {
    vec!["Pending"]
}

#[instrument(skip_all)]
pub async fn start_consumer(
    state: &AppState,
    options: sync::Arc<super::SchedulerOptions>,
    settings: sync::Arc<settings::SchedulerSettings>,
) -> CustomResult<(), errors::ProcessTrackerError> {
    use std::time::Duration;

    use rand::Rng;

    let timeout = rand::thread_rng().gen_range(0..=options.looper_interval.milliseconds);
    tokio::time::sleep(Duration::from_millis(timeout)).await;

    let mut interval =
        tokio::time::interval(Duration::from_millis(options.looper_interval.milliseconds));

    let mut shutdown_interval = tokio::time::interval(Duration::from_millis(
        options.readiness.graceful_termination_duration.milliseconds,
    ));

    let consumer_operation_counter = sync::Arc::new(atomic::AtomicU64::new(0));
    let signal = get_allowed_signals()
        .map_err(|error| {
            logger::error!("Signal Handler Error: {:?}", error);
            errors::ProcessTrackerError::ConfigurationError
        })
        .into_report()
        .attach_printable("Failed while creating a signals handler")?;
    let (sx, mut rx) = oneshot::channel();
    let handle = signal.handle();
    let task_handle = tokio::spawn(common_utils::signals::signal_handler(signal, sx));

    loop {
        match rx.try_recv() {
            Err(oneshot::error::TryRecvError::Empty) => {
                interval.tick().await;

                // A guard from env to disable the consumer
                if settings.consumer.disabled {
                    continue;
                }

                tokio::task::spawn(pt_utils::consumer_operation_handler(
                    state.clone(),
                    options.clone(),
                    settings.clone(),
                    |err| {
                        logger::error!(%err);
                    },
                    sync::Arc::clone(&consumer_operation_counter),
                ));
            }
            Ok(()) | Err(oneshot::error::TryRecvError::Closed) => {
                logger::debug!("Awaiting shutdown!");
                shutdown_interval.tick().await;
                let active_tasks = consumer_operation_counter.load(atomic::Ordering::Acquire);

                match active_tasks {
                    0 => {
                        logger::info!("Terminating consumer");
                        break;
                    }
                    _ => continue,
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
pub async fn consumer_operations(
    state: &AppState,
    _options: &super::SchedulerOptions,
    settings: &settings::SchedulerSettings,
) -> CustomResult<(), errors::ProcessTrackerError> {
    let stream_name = settings.stream.clone();
    let group_name = settings.consumer.consumer_group.clone();
    let consumer_name = format!("consumer_{}", Uuid::new_v4());

    let group_created = &mut state
        .store
        .consumer_group_create(&stream_name, &group_name, &RedisEntryId::AfterLastID)
        .await;
    if group_created.is_err() {
        logger::info!("Consumer group already exists");
    }

    let mut tasks = state
        .store
        .fetch_consumer_tasks(&stream_name, &group_name, &consumer_name)
        .await?;

    logger::info!("{} picked {} tasks", consumer_name, tasks.len());
    let mut handler = vec![];

    for task in tasks.iter_mut() {
        let pickup_time = common_utils::date_time::now();

        pt_utils::add_histogram_metrics(&pickup_time, task, &stream_name);

        metrics::TASK_CONSUMED.add(&metrics::CONTEXT, 1, &[]);
        let runner = pt_utils::runner_from_task(task)?;
        handler.push(tokio::task::spawn(start_workflow(
            state.clone(),
            task.clone(),
            pickup_time,
            runner,
        )))
    }
    future::join_all(handler).await;

    Ok(())
}

#[instrument(skip(db, redis_conn))]
pub async fn fetch_consumer_tasks(
    db: &dyn StorageInterface,
    redis_conn: &RedisConnectionPool,
    stream_name: &str,
    group_name: &str,
    consumer_name: &str,
) -> CustomResult<Vec<storage::ProcessTracker>, errors::ProcessTrackerError> {
    let batches = pt_utils::get_batches(redis_conn, stream_name, group_name, consumer_name).await?;

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
pub async fn start_workflow(
    state: AppState,
    process: storage::ProcessTracker,
    _pickup_time: PrimitiveDateTime,
    runner: workflows::PTRunner,
) {
    tracing::Span::current().record("workflow_id", Uuid::new_v4().to_string());
    workflows::perform_workflow_execution(&state, process, runner).await
}

pub async fn run_executor<'a>(
    state: &'a AppState,
    process: storage::ProcessTracker,
    operation: &(impl ProcessTrackerWorkflow + Send + Sync),
) {
    let output = operation.execute_workflow(state, process.clone()).await;
    match output {
        Ok(_) => operation.success_handler(state, process).await,
        Err(error) => match operation.error_handler(state, process.clone(), error).await {
            Ok(_) => (),
            Err(error) => {
                logger::error!(%error, "Failed while handling error");
                let status = process
                    .finish_with_status(&*state.store, "GLOBAL_FAILURE".to_string())
                    .await;
                if let Err(err) = status {
                    logger::error!(%err, "Failed while performing database operation: GLOBAL_FAILURE");
                }
            }
        },
    };
    metrics::TASK_PROCESSED.add(&metrics::CONTEXT, 1, &[]);
}

#[instrument(skip_all)]
pub async fn consumer_error_handler<E: fmt::Display + fmt::Debug>(
    state: &AppState,
    process: storage::ProcessTracker,
    error: E,
) -> CustomResult<(), errors::ProcessTrackerError> {
    logger::error!(pt.name = ?process.name, pt.id = %process.id, ?error, "ERROR: Failed while executing workflow");

    let db: &dyn StorageInterface = &*state.store;
    db.process_tracker_update_process_status_by_ids(
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
    db: &dyn StorageInterface,
    process_tracker_entry: storage::ProcessTrackerNew,
) -> CustomResult<(), errors::StorageError> {
    db.insert_process(process_tracker_entry).await?;
    Ok(())
}
