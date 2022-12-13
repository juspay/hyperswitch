// TODO: Figure out what to log

use std::{
    fmt,
    sync::{self, atomic},
};

use error_stack::ResultExt;
use futures::future;
use redis_interface::{RedisConnectionPool, RedisEntryId};
use router_env::{tracing, tracing::instrument};
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
    logger::{error, info},
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

    let consumer_operation_counter = sync::Arc::new(atomic::AtomicU64::new(0));
    loop {
        interval.tick().await;

        let is_ready = options.readiness.is_ready;
        if is_ready {
            tokio::task::spawn(pt_utils::consumer_operation_handler(
                state.clone(),
                options.clone(),
                settings.clone(),
                |err| {
                    error!(%err);
                },
                sync::Arc::clone(&consumer_operation_counter),
            ));
        } else {
            tokio::time::interval(Duration::from_millis(
                options.readiness.graceful_termination_duration.milliseconds,
            ))
            .tick()
            .await;
            let _active_tasks = consumer_operation_counter.load(atomic::Ordering::Acquire);
            // TODO: Handle termination?
            info!("Terminating consumer");
            break;
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn consumer_operations(
    state: &AppState,
    _options: &super::SchedulerOptions,
    settings: &settings::SchedulerSettings,
) -> CustomResult<(), errors::ProcessTrackerError> {
    let stream_name = settings.stream.clone();
    let group_name = settings.consumer_group.clone();
    let consumer_name = format!("consumer_{}", Uuid::new_v4());

    let group_created = &mut state
        .store
        .consumer_group_create(&stream_name, &group_name, &RedisEntryId::AfterLastID)
        .await;
    if group_created.is_err() {
        info!("Consumer group already exists");
    }

    let mut tasks = state
        .store
        .fetch_consumer_tasks(&stream_name, &group_name, &consumer_name)
        .await?;

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

    let tasks = batches.into_iter().fold(Vec::new(), |mut acc, batch| {
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

    let updated_tasks = db
        .process_tracker_update_process_status_by_ids(
            task_ids,
            storage::ProcessTrackerUpdate::StatusUpdate {
                status: enums::ProcessTrackerStatus::ProcessStarted,
                business_status: None,
            },
        )
        .await
        .change_context(errors::ProcessTrackerError::ProcessFetchingFailed)?;

    Ok(updated_tasks)
}

// Accept flow_options if required
#[instrument(skip(state))]
pub async fn start_workflow(
    state: AppState,
    process: storage::ProcessTracker,
    _pickup_time: PrimitiveDateTime,
    runner: workflows::PTRunner,
) {
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
                error!("Failed while handling error");
                error!(%error);
                let status = process
                    .finish_with_status(&*state.store, "GLOBAL_FAILURE".to_string())
                    .await;
                if let Err(err) = status {
                    error!("Failed while performing database operation: GLOBAL_FAILURE");
                    error!(%err)
                }
            }
        },
    };
    metrics::TASK_PROCESSED.add(&metrics::CONTEXT, 1, &[]);
}

#[instrument(skip_all)]
pub async fn some_error_handler<E: fmt::Display>(
    state: &AppState,
    process: storage::ProcessTracker,
    error: E,
) -> CustomResult<(), errors::ProcessTrackerError> {
    error!(%process.id, "Failed while executing workflow");
    error!(%error);
    error!(
        pt.name = ?process.name,
        pt.id = %process.id,
        "Some error occurred"
    );

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
