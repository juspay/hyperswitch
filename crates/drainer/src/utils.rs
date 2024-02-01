use std::sync::{atomic, Arc};

use error_stack::IntoReport;
use redis_interface as redis;
use serde::de::Deserialize;

use crate::{
    errors, kv, metrics,
    stream::{StreamEntries, StreamReadResult},
};

/// Parses the stream entries from the given `StreamReadResult` for the specified `stream_name`.
pub fn parse_stream_entries<'a>(
    read_result: &'a StreamReadResult,
    stream_name: &str,
) -> errors::DrainerResult<&'a StreamEntries> {
    read_result
        .get(stream_name)
        .ok_or_else(|| {
            errors::DrainerError::RedisError(error_stack::report!(
                redis::errors::RedisError::NotFound
            ))
        })
        .into_report()
}

/// Deserialize a JSON value into an i64. If the value is a string, it will attempt to parse the string into an i64. If the value is a number, it will attempt to extract the i64 from the number. If the value is of any other type, it will return an error.
pub(crate) fn deserialize_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = serde_json::Value::deserialize(deserializer)?;
    match s {
        serde_json::Value::String(str_val) => str_val.parse().map_err(serde::de::Error::custom),
        serde_json::Value::Number(num_val) => match num_val.as_i64() {
            Some(val) => Ok(val),
            None => Err(serde::de::Error::custom(format!(
                "could not convert {num_val:?} to i64"
            ))),
        },
        other => Err(serde::de::Error::custom(format!(
            "unexpected data format - expected string or number, got: {other:?}"
        ))),
    }
}

/// Deserialize a key-value database operation from the given deserializer.
///
/// # Arguments
///
/// * `deserializer` - A serde deserializer for the given data format
///
/// # Returns
///
/// A Result containing the deserialized key-value database operation, or a custom error if deserialization fails.
///
/// # Errors
///
/// An error is returned if the deserialization fails or if the data format is unexpected.
///
pub(crate) fn deserialize_db_op<'de, D>(deserializer: D) -> Result<kv::DBOperation, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = serde_json::Value::deserialize(deserializer)?;
    match s {
        serde_json::Value::String(str_val) => {
            serde_json::from_str(&str_val).map_err(serde::de::Error::custom)
        }
        other => Err(serde::de::Error::custom(format!(
            "unexpected data format - expected string got: {other:?}"
        ))),
    }
}

// Here the output is in the format (stream_index, jobs_picked),
// similar to the first argument of the function
#[inline(always)]
/// Increments the stream index and resets the jobs_picked counter if the index reaches the total number of streams.
///
/// # Arguments
///
/// * `index` - The current index of the stream
/// * `jobs_picked` - A reference to an atomic counter for the jobs picked
/// * `total_streams` - The total number of streams
///
/// # Returns
///
/// The new index after incrementing, or 0 if the index reaches the total number of streams
pub async fn increment_stream_index(
    (index, jobs_picked): (u8, Arc<atomic::AtomicU8>),
    total_streams: u8,
) -> u8 {
    if index == total_streams - 1 {
        match jobs_picked.load(atomic::Ordering::SeqCst) {
            0 => metrics::CYCLES_COMPLETED_UNSUCCESSFULLY.add(&metrics::CONTEXT, 1, &[]),
            _ => metrics::CYCLES_COMPLETED_SUCCESSFULLY.add(&metrics::CONTEXT, 1, &[]),
        }
        jobs_picked.store(0, atomic::Ordering::SeqCst);
        0
    } else {
        index + 1
    }
}
