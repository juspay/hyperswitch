use std::sync;

use common_utils::errors::CustomResult;
use diesel_models::enums::{self, ProcessTrackerStatus};
pub use diesel_models::process_tracker as storage;
use error_stack::{report, ResultExt};
use redis_interface::{RedisConnectionPool, RedisEntryId};
use router_env::{instrument, tracing};
use uuid::Uuid;

use super::{
    consumer::{self, types::process_data, workflows},
    env::logger,
};
use crate::{
    configs::settings::SchedulerSettings, consumer::types::ProcessTrackerBatch, errors,
    flow::SchedulerFlow, metrics, SchedulerInterface, SchedulerSessionState,
};

pub async fn divide_and_append_tasks<T>(
    state: &T,
    flow: SchedulerFlow,
    tasks: Vec<storage::ProcessTracker>,
    settings: &SchedulerSettings,
) -> CustomResult<(), errors::ProcessTrackerError>
where
    T: SchedulerInterface + Send + Sync + ?Sized,
{
    let batches = divide(tasks, settings);
    // Safety: Assuming we won't deal with more than `u64::MAX` batches at once
    #[allow(clippy::as_conversions)]
    metrics::BATCHES_CREATED.add(batches.len() as u64, &[]); // Metrics
    for batch in batches {
        let result = update_status_and_append(state, flow, batch).await;
        match result {
            Ok(_) => (),
            Err(error) => logger::error!(?error),
        }
    }
    Ok(())
}

pub async fn update_status_and_append<T>(
    state: &T,
    flow: SchedulerFlow,
    pt_batch: ProcessTrackerBatch,
) -> CustomResult<(), errors::ProcessTrackerError>
where
    T: SchedulerInterface + Send + Sync + ?Sized,
{
    let process_ids: Vec<String> = pt_batch
        .trackers
        .iter()
        .map(|process| process.id.to_owned())
        .collect();
    match flow {
        SchedulerFlow::Producer => state
            .process_tracker_update_process_status_by_ids(
                process_ids,
                storage::ProcessTrackerUpdate::StatusUpdate {
                    status: ProcessTrackerStatus::Processing,
                    business_status: None,
                },
            )
            .await
            .map_or_else(
                |error| {
                    logger::error!(?error, "Error while updating process status");
                    Err(error.change_context(errors::ProcessTrackerError::ProcessUpdateFailed))
                },
                |count| {
                    logger::debug!("Updated status of {count} processes");
                    Ok(())
                },
            ),
        SchedulerFlow::Cleaner => {
            let res = state
                .reinitialize_limbo_processes(process_ids, common_utils::date_time::now())
                .await;
            match res {
                Ok(count) => {
                    logger::debug!("Reinitialized {count} processes");
                    Ok(())
                }
                Err(error) => {
                    logger::error!(?error, "Error while reinitializing processes");
                    Err(error.change_context(errors::ProcessTrackerError::ProcessUpdateFailed))
                }
            }
        }
        _ => {
            let error = format!("Unexpected scheduler flow {flow:?}");
            logger::error!(%error);
            Err(report!(errors::ProcessTrackerError::UnexpectedFlow).attach_printable(error))
        }
    }?;

    let field_value_pairs = pt_batch.to_redis_field_value_pairs()?;

    match state
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
            let update_res = state
                .process_tracker_update_process_status_by_ids(
                    pt_batch
                        .trackers
                        .iter()
                        .map(|process| process.id.clone())
                        .collect(),
                    storage::ProcessTrackerUpdate::StatusUpdate {
                        status: ProcessTrackerStatus::Processing,
                        business_status: None,
                    },
                )
                .await
                .map_or_else(
                    |error| {
                        logger::error!(?error, "Error while updating process status");
                        Err(error.change_context(errors::ProcessTrackerError::ProcessUpdateFailed))
                    },
                    |count| {
                        logger::debug!("Updated status of {count} processes");
                        Ok(())
                    },
                );

            match update_res {
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
                group_name: conf.consumer.consumer_group.clone(),
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
    let response = match conn
        .stream_read_with_options(
            stream_name,
            RedisEntryId::UndeliveredEntryID,
            // Update logic for collecting to Vec and flattening, if count > 1 is provided
            Some(1),
            None,
            Some((group_name, consumer_name)),
        )
        .await
    {
        Ok(response) => response,
        Err(error) => {
            if let redis_interface::errors::RedisError::StreamEmptyOrNotAvailable =
                error.current_context()
            {
                logger::debug!("No batches processed as stream is empty");
                return Ok(Vec::new());
            } else {
                return Err(error.change_context(errors::ProcessTrackerError::BatchNotFound));
            }
        }
    };

    metrics::BATCHES_CONSUMED.add(1, &[]);

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

    conn.stream_acknowledge_entries(&stream_name.into(), group_name, entry_ids.clone())
        .await
        .map_err(|error| {
            logger::error!(?error, "Error acknowledging batch in stream");
            error.change_context(errors::ProcessTrackerError::BatchUpdateFailed)
        })?;
    conn.stream_delete_entries(&stream_name.into(), entry_ids.clone())
        .await
        .map_err(|error| {
            logger::error!(?error, "Error deleting batch from stream");
            error.change_context(errors::ProcessTrackerError::BatchDeleteFailed)
        })?;

    Ok(batches)
}

pub fn get_process_tracker_id<'a>(
    runner: storage::ProcessTrackerRunner,
    task_name: &'a str,
    txn_id: &'a str,
    merchant_id: &'a common_utils::id_type::MerchantId,
) -> String {
    format!(
        "{runner}_{task_name}_{txn_id}_{}",
        merchant_id.get_string_repr()
    )
}

pub fn get_time_from_delta(delta: Option<i32>) -> Option<time::PrimitiveDateTime> {
    delta.map(|t| common_utils::date_time::now().saturating_add(time::Duration::seconds(t.into())))
}

#[instrument(skip_all)]
pub async fn consumer_operation_handler<E, T>(
    state: T,
    settings: sync::Arc<SchedulerSettings>,
    error_handler_fun: E,
    workflow_selector: impl workflows::ProcessTrackerWorkflows<T> + 'static + Copy + std::fmt::Debug,
) where
    // Error handler function
    E: FnOnce(error_stack::Report<errors::ProcessTrackerError>),
    T: SchedulerSessionState + Send + Sync + 'static,
{
    match consumer::consumer_operations(&state, &settings, workflow_selector).await {
        Ok(_) => (),
        Err(err) => error_handler_fun(err),
    }
}

pub fn add_histogram_metrics(
    pickup_time: &time::PrimitiveDateTime,
    task: &mut storage::ProcessTracker,
    stream_name: &str,
) {
    #[warn(clippy::option_map_unit_fn)]
    if let Some((schedule_time, runner)) = task.schedule_time.as_ref().zip(task.runner.as_ref()) {
        let pickup_schedule_delta = (*pickup_time - *schedule_time).as_seconds_f64();
        logger::info!("Time delta for scheduled tasks: {pickup_schedule_delta} seconds");
        let runner_name = runner.clone();
        metrics::CONSUMER_OPS.record(
            pickup_schedule_delta,
            router_env::metric_attributes!((stream_name.to_owned(), runner_name)),
        );
    };
}

pub fn get_schedule_time(
    mapping: process_data::ConnectorPTMapping,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Option<i32> {
    let mapping = match mapping.custom_merchant_mapping.get(merchant_id) {
        Some(map) => map.clone(),
        None => mapping.default_mapping,
    };

    // For first try, get the `start_after` time
    if retry_count == 0 {
        Some(mapping.start_after)
    } else {
        get_delay(retry_count, &mapping.frequencies)
    }
}

pub fn get_pm_schedule_time(
    mapping: process_data::PaymentMethodsPTMapping,
    pm: enums::PaymentMethod,
    retry_count: i32,
) -> Option<i32> {
    let mapping = match mapping.custom_pm_mapping.get(&pm) {
        Some(map) => map.clone(),
        None => mapping.default_mapping,
    };

    if retry_count == 0 {
        Some(mapping.start_after)
    } else {
        get_delay(retry_count, &mapping.frequencies)
    }
}

pub fn get_outgoing_webhook_retry_schedule_time(
    mapping: process_data::OutgoingWebhookRetryProcessTrackerMapping,
    merchant_id: &common_utils::id_type::MerchantId,
    retry_count: i32,
) -> Option<i32> {
    let retry_mapping = match mapping.custom_merchant_mapping.get(merchant_id) {
        Some(map) => map.clone(),
        None => mapping.default_mapping,
    };

    // For first try, get the `start_after` time
    if retry_count == 0 {
        Some(retry_mapping.start_after)
    } else {
        get_delay(retry_count, &retry_mapping.frequencies)
    }
}

/// Get the delay based on the retry count
pub fn get_delay<'a>(
    retry_count: i32,
    frequencies: impl IntoIterator<Item = &'a (i32, i32)>,
) -> Option<i32> {
    // Preferably, fix this by using unsigned ints
    if retry_count <= 0 {
        return None;
    }

    let mut cumulative_count = 0;
    for &(frequency, count) in frequencies.into_iter() {
        cumulative_count += count;
        if cumulative_count >= retry_count {
            return Some(frequency);
        }
    }

    None
}

pub(crate) async fn lock_acquire_release<T, F, Fut>(
    state: &T,
    settings: &SchedulerSettings,
    callback: F,
) -> CustomResult<(), errors::ProcessTrackerError>
where
    F: Fn() -> Fut,
    T: SchedulerInterface + Send + Sync + ?Sized,
    Fut: futures::Future<Output = CustomResult<(), errors::ProcessTrackerError>>,
{
    let tag = "PRODUCER_LOCK";
    let lock_key = &settings.producer.lock_key;
    let lock_val = "LOCKED";
    let ttl = settings.producer.lock_ttl;

    if state
        .acquire_pt_lock(tag, lock_key, lock_val, ttl)
        .await
        .change_context(errors::ProcessTrackerError::ERedisError(
            errors::RedisError::RedisConnectionError.into(),
        ))?
    {
        let result = callback().await;
        state
            .release_pt_lock(tag, lock_key)
            .await
            .map_err(errors::ProcessTrackerError::ERedisError)?;
        result
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_delay() {
        let frequency_count = vec![(300, 10), (600, 5), (1800, 3), (3600, 2)];

        let retry_counts_and_expected_delays = [
            (-4, None),
            (-2, None),
            (0, None),
            (4, Some(300)),
            (7, Some(300)),
            (10, Some(300)),
            (12, Some(600)),
            (16, Some(1800)),
            (18, Some(1800)),
            (20, Some(3600)),
            (24, None),
            (30, None),
        ];

        for (retry_count, expected_delay) in retry_counts_and_expected_delays {
            let delay = get_delay(retry_count, &frequency_count);

            assert_eq!(
                delay, expected_delay,
                "Delay and expected delay differ for `retry_count` = {retry_count}"
            );
        }
    }
}
