use std::{collections::HashMap, sync::Arc};

use fred::types as fred;
use redis_interface as redis;

use crate::errors::DrainerError;

pub type StreamEntries = Vec<(String, HashMap<String, String>)>;
pub type StreamReadResult = HashMap<String, StreamEntries>;

pub async fn is_stream_available(stream_index: u8, store: Arc<router::services::Store>) -> bool {
    let stream_key_flag = get_steam_key_flag(store.clone(), stream_index.to_string());
    let value = fred::RedisValue::Boolean(true);

    match store
        .redis_conn
        .set_key_if_not_exist(stream_key_flag.as_str(), value)
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
) -> Result<StreamReadResult, DrainerError> {
    let stream_key = fred::MultipleKeys::from(stream_name);
    // "0-0" id gives first entry
    let stream_id = fred::XID::Manual("0-0".into());
    let entries = redis
        .stream_read_entries(stream_key, stream_id, Some(max_read_count))
        .await
        .map_err(|_| DrainerError::StreamReadError(stream_name.to_owned()))?;
    Ok(entries)
}

pub async fn trim_from_stream(
    stream_name: &str,
    minimum_entry_id: &str,
    redis: &redis::RedisConnectionPool,
) -> Result<usize, DrainerError> {
    let trim_kind = fred::XCapKind::MinID;
    let trim_type = fred::XCapTrim::Exact;
    let trim_id = fred::StringOrNumber::String(minimum_entry_id.into());
    let xcap = fred::XCap::try_from((trim_kind, trim_type, trim_id))
        .map_err(|_| DrainerError::StreamTrimFailed(stream_name.to_owned()))?;

    let trim_result = redis
        .stream_trim_entries(stream_name, xcap)
        .await
        .map_err(|_| DrainerError::StreamTrimFailed(stream_name.to_owned()))?;

    // Since xtrim deletes entires below given id excluding the given id.
    // Hence, deleting the minimum entry id
    redis
        .stream_delete_entries(stream_name, minimum_entry_id)
        .await
        .map_err(|_| DrainerError::StreamTrimFailed(stream_name.to_owned()))?;

    Ok(trim_result + 1)
}

pub async fn make_stream_available(
    stream_name_flag: &str,
    redis: &redis::RedisConnectionPool,
) -> Result<(), DrainerError> {
    redis
        .delete_key(stream_name_flag)
        .await
        .map_err(|_| DrainerError::DeleteKeyFailed(stream_name_flag.to_owned()))
}

pub async fn get_stream_length(
    redis: &redis::RedisConnectionPool,
    stream_name: &str,
) -> Result<usize, DrainerError> {
    let length = redis
        .stream_get_length(stream_name)
        .await
        .map_err(|_| DrainerError::StreamGetLengthError(stream_name.to_owned()))?;
    Ok(length)
}

pub fn parse_stream_entries<'a>(
    read_result: &'a StreamReadResult,
    stream_name: &str,
) -> Result<(&'a StreamEntries, String), DrainerError> {
    if let Some(entries) = read_result.get(stream_name) {
        if let Some(last_entry) = entries.last() {
            Ok((entries, last_entry.0.clone()))
        } else {
            Err(DrainerError::NoStreamEntry(stream_name.to_owned()))
        }
    } else {
        Err(DrainerError::NoStreamEntry(stream_name.to_owned()))
    }
}

pub fn determine_read_count(stream_length: usize, max_read_count: usize) -> usize {
    if stream_length > max_read_count {
        max_read_count
    } else {
        stream_length
    }
}

pub fn increment_stream_index(index: u8, total_streams: u8) -> u8 {
    if index == total_streams - 1 {
        0
    } else {
        index + 1
    }
}

pub fn get_steam_key_flag(store: Arc<router::services::Store>, stream_index: String) -> String {
    format!(
        "{}_in_use",
        store.drainer_stream(stream_index.as_str()).as_str()
    )
}
