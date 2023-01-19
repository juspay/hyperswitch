use std::{
    sync::{self, atomic},
    time as std_time,
};

use error_stack::{report, ResultExt};
use futures::StreamExt;
use redis_interface::{RedisConnectionPool, RedisEntryId};
use router_env::opentelemetry;
use tokio::sync::oneshot;
use uuid::Uuid;

use super::{consumer, metrics, process_data, workflows};
use crate::{
    configs::settings::SchedulerSettings,
    core::errors::{self, CustomResult},
    logger,
    routes::AppState,
    scheduler::{ProcessTrackerBatch, SchedulerFlow},
    types::storage::{self, enums::ProcessTrackerStatus},
    utils::{OptionExt, StringExt},
};

pub async fn divide_and_append_tasks(
    state: &AppState,
    flow: SchedulerFlow,
    tasks: Vec<storage::ProcessTracker>,
    settings: &SchedulerSettings,
) -> CustomResult<(), errors::ProcessTrackerError> {
    let batches = divide(tasks, settings);
    // Safety: Assuming we won't deal with more than `u64::MAX` batches at once
    #[allow(clippy::as_conversions)]
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
                state
                .store
                .process_tracker_update_process_status_by_ids(
                    process_ids,
                    storage::ProcessTrackerUpdate::StatusUpdate {
                        status: ProcessTrackerStatus::Processing,
                        business_status: None,
                    },
                )
                .await.map_or_else(|error| {
                    logger::error!(error=%error.current_context(),"Error while updating process status");
                    Err(error.change_context(errors::ProcessTrackerError::ProcessUpdateFailed))
                }, |count| {
                    logger::debug!("Updated status of {count} processes");
                    Ok(())
                })
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

    match state
        .store
        .stream_append_entry(
            &pt_batch.stream_name,
            &RedisEntryId::AutoGeneratedID,
            field_value_pairs,
        )
        .await
        .change_context(errors::ProcessTrackerError::BatchInsertionFailed)
    {
        Ok(x) => Ok(x),
        Err(mut err) => {
            match state
                .store
                .process_tracker_update_process_status_by_ids(
                    pt_batch.trackers.iter().map(|process| process.id.clone()).collect(),
                    storage::ProcessTrackerUpdate::StatusUpdate {
                        status: ProcessTrackerStatus::Processing,
                        business_status: None,
                    },
                )
                .await.map_or_else(|error| {
                    logger::error!(error=%error.current_context(),"Error while updating process status");
                    Err(error.change_context(errors::ProcessTrackerError::ProcessUpdateFailed))
                }, |count| {
                    logger::debug!("Updated status of {count} processes");
                    Ok(())
                }) {
                    Ok(_) => (),
                    Err(inner_err) => {
                        err.extend_one(inner_err);
                    }
                };

            Err(err)
        }
    }
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
    conn: &RedisConnectionPool,
    stream_name: &str,
    group_name: &str,
    consumer_name: &str,
) -> CustomResult<Vec<ProcessTrackerBatch>, errors::ProcessTrackerError> {
    let response = conn
        .stream_read_with_options(
            stream_name,
            RedisEntryId::UndeliveredEntryID,
            // Update logic for collecting to Vec and flattening, if count > 1 is provided
            Some(1),
            None,
            Some((group_name, consumer_name)),
        )
        .await
        .map_err(|error| {
            logger::warn!(%error, "Warning: finding batch in stream");
            error.change_context(errors::ProcessTrackerError::BatchNotFound)
        })?;
    metrics::BATCHES_CONSUMED.add(&metrics::CONTEXT, 1, &[]);

    let (batches, entry_ids): (Vec<Vec<ProcessTrackerBatch>>, Vec<Vec<String>>) = response.into_values().map(|entries| {
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
    consumer_operation_counter.fetch_add(1, atomic::Ordering::Release);
    let start_time = std_time::Instant::now();

    match consumer::consumer_operations(&state, &options, &settings).await {
        Ok(_) => (),
        Err(err) => error_handler_fun(err),
    }
    let end_time = std_time::Instant::now();
    let duration = end_time.saturating_duration_since(start_time).as_secs_f64();
    logger::debug!("Time taken to execute consumer_operation: {}s", duration);

    let current_count = consumer_operation_counter.fetch_sub(1, atomic::Ordering::Release);
    logger::info!("Current tasks being executed: {}", current_count);
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

pub fn get_schedule_time(
    mapping: process_data::ConnectorPTMapping,
    merchant_name: &str,
    retry_count: i32,
) -> Option<i32> {
    let mapping = match mapping.custom_merchant_mapping.get(merchant_name) {
        Some(map) => map.clone(),
        None => mapping.default_mapping,
    };

    if retry_count == 0 {
        Some(mapping.start_after)
    } else {
        get_delay(
            retry_count,
            mapping.count.iter().zip(mapping.frequency.iter()),
        )
    }
}

fn get_delay<'a>(
    retry_count: i32,
    mut array: impl Iterator<Item = (&'a i32, &'a i32)>,
) -> Option<i32> {
    match array.next() {
        Some(ele) => {
            let v = retry_count - ele.0;
            if v <= 0 {
                Some(*ele.1)
            } else {
                get_delay(v, array)
            }
        }
        None => None,
    }
}

pub(crate) async fn lock_acquire_release<F, Fut, E>(
    state: &AppState,
    settings: &SchedulerSettings,
    callback: F,
) -> Result<(), E>
where
    F: Fn() -> Fut,
    Fut: futures::Future<Output = Result<(), E>>,
{
    let tag = "PRODUCER_LOCK";
    let lock_key = &settings.producer.lock_key;
    let lock_val = "LOCKED";
    let ttl = settings.producer.lock_ttl;

    let result = if state
        .store
        .acquire_pt_lock(tag, lock_key, lock_val, ttl)
        .await
    {
        let result = callback().await;
        state.store.release_pt_lock(tag, lock_key).await;
        result
    } else {
        Ok(())
    };
    result
}

pub(crate) async fn signal_handler(
    mut sig: signal_hook_tokio::Signals,
    sender: oneshot::Sender<()>,
) {
    if let Some(signal) = sig.next().await {
        logger::debug!(
            "Received singal: {:?}",
            signal_hook::low_level::signal_name(signal)
        );
        match signal {
            signal_hook::consts::SIGTERM | signal_hook::consts::SIGINT => match sender.send(()) {
                Ok(_) => {
                    logger::debug!("Request for force shutdown received")
                }
                Err(_) => {
                    logger::error!(
                        "The receiver is closed, a termination call might already be sent"
                    )
                }
            },
            _ => {}
        }
    }
}
