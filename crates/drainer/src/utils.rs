use std::{collections::HashMap, sync::Arc};

use error_stack::IntoReport;
use redis_interface as redis;

use crate::{
    errors::{self, DrainerError},
    logger, metrics, services,
};

pub type StreamEntries = Vec<(String, HashMap<String, String>)>;
pub type StreamReadResult = HashMap<String, StreamEntries>;

pub async fn is_stream_available(stream_index: u8, store: Arc<services::Store>) -> bool {
    let stream_key_flag = get_stream_key_flag(store.clone(), stream_index);

    match store
        .redis_conn
        .set_key_if_not_exists_with_expiry(stream_key_flag.as_str(), true, None)
        .await
    {
        Ok(resp) => resp == redis::types::SetnxReply::KeySet,
        Err(error) => {
            logger::error!(?error);
            // Add metrics or logs
            false
        }
    }
}

pub async fn read_from_stream(
    stream_name: &str,
    max_read_count: u64,
    redis: &redis::RedisConnectionPool,
) -> errors::DrainerResult<StreamReadResult> {
    // "0-0" id gives first entry
    let stream_id = "0-0";
    let (output, execution_time) = common_utils::date_time::time_it(|| async {
        let entries = redis
            .stream_read_entries(stream_name, stream_id, Some(max_read_count))
            .await
            .map_err(DrainerError::from)
            .into_report()?;
        Ok(entries)
    })
    .await;

    metrics::REDIS_STREAM_READ_TIME.record(
        &metrics::CONTEXT,
        execution_time,
        &[metrics::KeyValue::new("stream", stream_name.to_owned())],
    );

    output
}

pub async fn trim_from_stream(
    stream_name: &str,
    minimum_entry_id: &str,
    redis: &redis::RedisConnectionPool,
) -> errors::DrainerResult<usize> {
    let trim_kind = redis::StreamCapKind::MinID;
    let trim_type = redis::StreamCapTrim::Exact;
    let trim_id = minimum_entry_id;
    let (trim_result, execution_time) =
        common_utils::date_time::time_it::<errors::DrainerResult<_>, _, _>(|| async {
            let trim_result = redis
                .stream_trim_entries(stream_name, (trim_kind, trim_type, trim_id))
                .await
                .map_err(DrainerError::from)
                .into_report()?;

            // Since xtrim deletes entries below given id excluding the given id.
            // Hence, deleting the minimum entry id
            redis
                .stream_delete_entries(stream_name, minimum_entry_id)
                .await
                .map_err(DrainerError::from)
                .into_report()?;

            Ok(trim_result)
        })
        .await;

    metrics::REDIS_STREAM_TRIM_TIME.record(
        &metrics::CONTEXT,
        execution_time,
        &[metrics::KeyValue::new("stream", stream_name.to_owned())],
    );

    // adding 1 because we are deleting the given id too
    Ok(trim_result? + 1)
}

pub async fn make_stream_available(
    stream_name_flag: &str,
    redis: &redis::RedisConnectionPool,
) -> errors::DrainerResult<()> {
    match redis.delete_key(stream_name_flag).await {
        Ok(redis::DelReply::KeyDeleted) => Ok(()),
        Ok(redis::DelReply::KeyNotDeleted) => {
            logger::error!("Tried to unlock a stream which is already unlocked");
            Ok(())
        }
        Err(error) => Err(DrainerError::from(error).into()),
    }
}

pub fn parse_stream_entries<'a>(
    read_result: &'a StreamReadResult,
    stream_name: &str,
) -> errors::DrainerResult<(&'a StreamEntries, String)> {
    read_result
        .get(stream_name)
        .and_then(|entries| {
            entries
                .last()
                .map(|last_entry| (entries, last_entry.0.clone()))
        })
        .ok_or_else(|| {
            errors::DrainerError::RedisError(error_stack::report!(
                redis::errors::RedisError::NotFound
            ))
        })
        .into_report()
}

// Here the output is in the format (stream_index, jobs_picked),
// similar to the first argument of the function
pub async fn increment_stream_index(
    (index, jobs_picked): (u8, u8),
    total_streams: u8,
    interval: &mut tokio::time::Interval,
) -> (u8, u8) {
    if index == total_streams - 1 {
        interval.tick().await;
        match jobs_picked {
            0 => metrics::CYCLES_COMPLETED_UNSUCCESSFULLY.add(&metrics::CONTEXT, 1, &[]),
            _ => metrics::CYCLES_COMPLETED_SUCCESSFULLY.add(&metrics::CONTEXT, 1, &[]),
        }
        (0, 0)
    } else {
        (index + 1, jobs_picked)
    }
}

pub(crate) fn get_stream_key_flag(store: Arc<services::Store>, stream_index: u8) -> String {
    format!("{}_in_use", get_drainer_stream_name(store, stream_index))
}

pub(crate) fn get_drainer_stream_name(store: Arc<services::Store>, stream_index: u8) -> String {
    store.drainer_stream(format!("shard_{stream_index}").as_str())
}
