use std::{collections::HashMap, sync::Arc};

use error_stack::{IntoReport, ResultExt};
use redis_interface as redis;
use router::services::Store;

use crate::errors;

pub type StreamEntries = Vec<(String, HashMap<String, String>)>;
pub type StreamReadResult = HashMap<String, StreamEntries>;

pub async fn is_stream_available(stream_index: u8, store: Arc<router::services::Store>) -> bool {
    let stream_key_flag = get_stream_key_flag(store.clone(), stream_index);

    match store
        .redis_conn
        .set_key_if_not_exist(stream_key_flag.as_str(), true)
        .await
    {
        Ok(resp) => resp == redis::types::SetnxReply::KeySet,
        Err(_e) => {
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
    let entries = redis
        .stream_read_entries(stream_name, stream_id, Some(max_read_count))
        .await
        .change_context(errors::DrainerError::StreamReadError(
            stream_name.to_owned(),
        ))?;
    Ok(entries)
}

pub async fn trim_from_stream(
    stream_name: &str,
    minimum_entry_id: &str,
    redis: &redis::RedisConnectionPool,
) -> errors::DrainerResult<usize> {
    let trim_kind = redis::StreamCapKind::MinID;
    let trim_type = redis::StreamCapTrim::Exact;
    let trim_id = minimum_entry_id;
    let trim_result = redis
        .stream_trim_entries(stream_name, (trim_kind, trim_type, trim_id))
        .await
        .change_context(errors::DrainerError::StreamTrimFailed(
            stream_name.to_owned(),
        ))?;

    // Since xtrim deletes entires below given id excluding the given id.
    // Hence, deleting the minimum entry id
    redis
        .stream_delete_entries(stream_name, minimum_entry_id)
        .await
        .change_context(errors::DrainerError::StreamTrimFailed(
            stream_name.to_owned(),
        ))?;

    // adding 1 because we are deleting the given id too
    Ok(trim_result + 1)
}

pub async fn make_stream_available(
    stream_name_flag: &str,
    redis: &redis::RedisConnectionPool,
) -> errors::DrainerResult<()> {
    redis
        .delete_key(stream_name_flag)
        .await
        .change_context(errors::DrainerError::DeleteKeyFailed(
            stream_name_flag.to_owned(),
        ))
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
        .ok_or_else(|| errors::DrainerError::NoStreamEntry(stream_name.to_owned()))
        .into_report()
}

pub fn increment_stream_index(index: u8, total_streams: u8) -> u8 {
    if index == total_streams - 1 {
        0
    } else {
        index + 1
    }
}

pub(crate) fn get_stream_key_flag(store: Arc<router::services::Store>, stream_index: u8) -> String {
    format!("{}_in_use", get_drainer_stream_name(store, stream_index))
}

pub(crate) fn get_drainer_stream_name(store: Arc<Store>, stream_index: u8) -> String {
    store.get_drainer_stream_name(format!("shard_{}", stream_index).as_str())
}
