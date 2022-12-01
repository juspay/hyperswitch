use std::{
    sync::{self, atomic},
    time as std_time,
};

use error_stack::{report, ResultExt};
use router_env::opentelemetry;
use uuid::Uuid;

use super::{consumer, metrics, workflows};
use crate::{
    configs::settings::SchedulerSettings,
    core::errors::{self, CustomResult},
    db::process_tracker::IProcessTracker,
    logger,
    routes::AppState,
    scheduler::{ProcessTrackerBatch, SchedulerFlow},
    types::storage::{self, enums::ProcessTrackerStatus},
    utils::{OptionExt, StringExt},
};

pub async fn acquire_pt_lock(
    state: &AppState,
    tag: &str,
    lock_key: &str,
    lock_val: &str,
    ttl: i64,
) -> bool {
    let conn = state.store.redis_conn.clone();
    let is_lock_acquired = conn.set_key_if_not_exist(lock_key, lock_val).await;
    match is_lock_acquired {
        Ok(redis_interface::SetNXReply::KeySet) => match conn.set_expiry(lock_key, ttl).await {
            Ok(()) => true,

            #[allow(unused_must_use)]
            Err(error) => {
                logger::error!(error=?error.current_context());
                conn.delete_key(lock_key).await;
                false
            }
        },
        Ok(redis_interface::SetNXReply::KeyNotSet) => {
            logger::error!(%tag, "Lock not acquired, previous fetch still in progress");
            false
        }
        Err(error) => {
            logger::error!(error=%error.current_context(), %tag, "Error while locking");
            false
        }
    }
}

pub async fn release_pt_lock(
    redis_conn: &redis_interface::RedisConnectionPool,
    tag: &str,
    lock_key: &str,
) -> bool {
    let is_lock_released = redis_conn.delete_key(lock_key).await;
    match is_lock_released {
        Ok(()) => true,
        Err(error) => {
            logger::error!(error=%error.current_context(), %tag, "Error while releasing lock");
            false
        }
    }
}

pub async fn divide_and_append_tasks(
    state: &AppState,
    flow: SchedulerFlow,
    tasks: Vec<storage::ProcessTracker>,
    settings: &SchedulerSettings,
) -> CustomResult<(), errors::ProcessTrackerError> {
    let batches = divide(tasks, settings);
    metrics::BATCHES_CREATED.add(&metrics::CONTEXT, batches.len() as u64, &[]); // Metrics
    for batch in batches {
        let result = update_status_and_append(state, flow, batch).await;
        match result {
            Ok(_) => (),
            Err(error) => logger::error!(error=%error.current_context()),
        }
    }
    Ok(())
}

pub async fn update_status_and_append(
    state: &AppState,
    flow: SchedulerFlow,
    pt_batch: ProcessTrackerBatch,
) -> CustomResult<(), errors::ProcessTrackerError> {
    let process_ids: Vec<String> = pt_batch
        .trackers
        .iter()
        .map(|process| process.id.to_owned())
        .collect();
    match flow {
        SchedulerFlow::Producer => {
            let res = state
                .store
                .process_tracker_update_process_status_by_ids(
                    process_ids,
                    storage::ProcessTrackerUpdate::StatusUpdate {
                        status: ProcessTrackerStatus::Processing,
                        business_status: None,
                    },
                )
                .await;
            match res {
                Ok(trackers) => {
                    let count = trackers.len();
                    logger::debug!("Updated status of {count} processes");
                    Ok(())
                }
                Err(error) => {
                    logger::error!(error=%error.current_context(),"Error while updating process status");
                    Err(error.change_context(errors::ProcessTrackerError::ProcessUpdateFailed))
                }
            }
        }
        SchedulerFlow::Cleaner => {
            let res = state
                .store
                .reinitialize_limbo_processes(process_ids, common_utils::date_time::now())
                .await;
            match res {
                Ok(count) => {
                    logger::debug!("Reinitialized {count} processes");
                    Ok(())
                }
                Err(error) => {
                    logger::error!(error=%error.current_context(),"Error while reinitializing processes");
                    Err(error.change_context(errors::ProcessTrackerError::ProcessUpdateFailed))
                }
            }
        }
        _ => {
            let error_msg = format!("Unexpected scheduler flow {flow:?}");
            logger::error!(error = %error_msg);
            Err(report!(errors::ProcessTrackerError::UnexpectedFlow).attach_printable(error_msg))
        }
    }?;

    let field_value_pairs = pt_batch.to_redis_field_value_pairs()?;
    let redis_conn = state.store.redis_conn.clone();

    redis_conn
        .stream_append_entry(
            &pt_batch.stream_name,
            &redis_interface::RedisEntryId::AutoGeneratedID,
            field_value_pairs,
        )
        .await
        .change_context(errors::ProcessTrackerError::BatchInsertionFailed) // TODO: Handle error? (Update status of processes back to PENDING?)
}

pub fn divide(
    tasks: Vec<storage::ProcessTracker>,
    conf: &SchedulerSettings,
) -> Vec<ProcessTrackerBatch> {
    let now = common_utils::date_time::now();
    let batch_size = conf.producer.batch_size;
    divide_into_batches(batch_size, tasks, now, conf)
}

pub fn divide_into_batches(
    batch_size: usize,
    tasks: Vec<storage::ProcessTracker>,
    batch_creation_time: time::PrimitiveDateTime,
    conf: &SchedulerSettings,
) -> Vec<ProcessTrackerBatch> {
    let batch_id = Uuid::new_v4().to_string();

    tasks
        .chunks(batch_size)
        .fold(Vec::new(), |mut batches, item| {
            let batch = ProcessTrackerBatch {
                id: batch_id.clone(),
                group_name: conf.consumer_group.clone(),
                stream_name: conf.stream.clone(),
                connection_name: String::new(),
                created_time: batch_creation_time,
                rule: String::new(), // is it required?
                trackers: item.to_vec(),
            };
            batches.push(batch);

            batches
        })
}

pub async fn get_batches(
    conn: &redis_interface::RedisConnectionPool,
    stream_name: &str,
    group_name: &str,
    consumer_name: &str,
) -> CustomResult<Vec<ProcessTrackerBatch>, errors::ProcessTrackerError> {
    let response = conn
        .stream_read_with_options(
            stream_name,
            redis_interface::RedisEntryId::UndeliveredEntryID,
            // Update logic for collecting to Vec and flattening, if count > 1 is provided
            Some(1),
            None,
            Some((group_name, consumer_name)),
        )
        .await
        .map_err(|error| {
            // FIXME: Not a failure when the PT is not overloaded this will throw an error
            logger::error!(%error, "Error finding batch in stream");
            error.change_context(errors::ProcessTrackerError::BatchNotFound)
        })?;
    metrics::BATCHES_CONSUMED.add(&metrics::CONTEXT, 1, &[]);

    let (batches, entry_ids): (Vec<Vec<ProcessTrackerBatch>>, Vec<Vec<String>>) = response.into_iter().map(|(_key, entries)| {
        entries.into_iter().try_fold(
            (Vec::new(), Vec::new()),
            |(mut batches, mut entry_ids), entry| {
                // Redis entry ID
                entry_ids.push(entry.0);
                // Value HashMap
                batches.push(ProcessTrackerBatch::from_redis_stream_entry(entry.1)?);

                Ok((batches, entry_ids))
            },
        )
    }).collect::<CustomResult<Vec<(Vec<ProcessTrackerBatch>, Vec<String>)>, errors::ProcessTrackerError>>()?
    .into_iter()
    .unzip();
    // Flattening the Vec's since the count provided above is 1. This needs to be updated if a
    // count greater than 1 is provided.
    let batches = batches.into_iter().flatten().collect::<Vec<_>>();
    let entry_ids = entry_ids.into_iter().flatten().collect::<Vec<_>>();

    conn.stream_acknowledge_entries(stream_name, group_name, entry_ids.clone())
        .await
        .map_err(|error| {
            logger::error!(%error, "Error acknowledging batch in stream");
            error.change_context(errors::ProcessTrackerError::BatchUpdateFailed)
        })?;
    conn.stream_delete_entries(stream_name, entry_ids.clone())
        .await
        .map_err(|error| {
            logger::error!(%error, "Error deleting batch from stream");
            error.change_context(errors::ProcessTrackerError::BatchDeleteFailed)
        })?;

    Ok(batches)
}

pub fn get_process_tracker_id<'a>(
    runner: &'a str,
    task_name: &'a str,
    txn_id: &'a str,
    merchant_id: &'a str,
) -> String {
    format!("{runner}_{task_name}_{txn_id}_{merchant_id}")
}

pub fn get_time_from_delta(delta: Option<i32>) -> Option<time::PrimitiveDateTime> {
    delta.map(|t| common_utils::date_time::now().saturating_add(time::Duration::seconds(t.into())))
}

pub async fn consumer_operation_handler<E>(
    state: AppState,
    options: sync::Arc<super::SchedulerOptions>,
    settings: sync::Arc<SchedulerSettings>,
    error_handler_fun: E,
    consumer_operation_counter: sync::Arc<atomic::AtomicU64>,
) where
    // Error handler function
    E: FnOnce(error_stack::Report<errors::ProcessTrackerError>),
{
    consumer_operation_counter.fetch_add(1, atomic::Ordering::Relaxed);
    let start_time = std_time::Instant::now();

    match consumer::consumer_operations(&state, &options, &settings).await {
        Ok(_) => (),
        Err(err) => error_handler_fun(err),
    }
    let end_time = std_time::Instant::now();
    let duration = end_time.saturating_duration_since(start_time).as_secs_f64();
    logger::debug!("Time taken to execute consumer_operation: {}s", duration);

    consumer_operation_counter.fetch_sub(1, atomic::Ordering::Relaxed);
}

pub fn runner_from_task(
    task: &storage::ProcessTracker,
) -> Result<workflows::PTRunner, errors::ProcessTrackerError> {
    let runner = task.runner.clone().get_required_value("runner")?;

    Ok(runner.parse_enum("PTRunner")?)
}

pub fn add_histogram_metrics(
    pickup_time: &time::PrimitiveDateTime,
    task: &mut storage::ProcessTracker,
    stream_name: &str,
) {
    #[warn(clippy::option_map_unit_fn)]
    if let Some((schedule_time, runner)) = task.schedule_time.as_ref().zip(task.runner.as_ref()) {
        let pickup_schedule_delta = (*pickup_time - *schedule_time).as_seconds_f64();
        logger::error!(%pickup_schedule_delta, "<- Time delta for scheduled tasks");
        let runner_name = runner.clone();
        metrics::CONSUMER_STATS.record(
            &metrics::CONTEXT,
            pickup_schedule_delta,
            &[opentelemetry::KeyValue::new(
                stream_name.to_owned(),
                runner_name,
            )],
        );
    };
}
