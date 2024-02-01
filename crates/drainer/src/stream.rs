use std::collections::HashMap;

use error_stack::IntoReport;
use redis_interface as redis;
use router_env::{logger, tracing};

use crate::{errors, metrics, Store};

pub type StreamEntries = Vec<(String, HashMap<String, String>)>;
pub type StreamReadResult = HashMap<String, StreamEntries>;

impl Store {
    #[inline(always)]
        /// This method takes a shard key as input and returns a string representing the drainer stream name for that shard. It formats the drainer stream name using the provided shard key and the drainer stream name from the configuration.
    pub fn drainer_stream(&self, shard_key: &str) -> String {
        // Example: {shard_5}_drainer_stream
        format!("{{{}}}_{}", shard_key, self.config.drainer_stream_name,)
    }

    #[inline(always)]
        /// Retrieves the stream key flag for the given stream index.
    /// 
    /// Given a stream index, this method returns the corresponding stream key flag in the format "stream_name_in_use".
    pub(crate) fn get_stream_key_flag(&self, stream_index: u8) -> String {
        format!("{}_in_use", self.get_drainer_stream_name(stream_index))
    }

    #[inline(always)]
        /// This method takes in a stream index and returns the name of the drainer stream associated with that index.
    pub(crate) fn get_drainer_stream_name(&self, stream_index: u8) -> String {
        self.drainer_stream(format!("shard_{stream_index}").as_str())
    }

    #[router_env::instrument(skip_all)]
        /// Checks if a stream is available by attempting to set a key in the Redis database with a specified expiry time.
    ///
    /// # Arguments
    ///
    /// * `stream_index` - The index of the stream to check for availability.
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the stream is available or not.
    pub async fn is_stream_available(&self, stream_index: u8) -> bool {
        let stream_key_flag = self.get_stream_key_flag(stream_index);

        match self
            .redis_conn
            .set_key_if_not_exists_with_expiry(stream_key_flag.as_str(), true, None)
            .await
        {
            Ok(resp) => resp == redis::types::SetnxReply::KeySet,
            Err(error) => {
                logger::error!(operation="lock_stream",err=?error);
                false
            }
        }
    }

        /// Asynchronously makes a stream available by attempting to delete the specified key from the Redis database. 
    /// If the key is successfully deleted, returns Ok(()). If the key is not deleted because it is already unlocked, logs an error and returns Ok(()). 
    /// If an error occurs during the deletion process, returns an Err containing a DrainerError.
    pub async fn make_stream_available(&self, stream_name_flag: &str) -> errors::DrainerResult<()> {
        match self.redis_conn.delete_key(stream_name_flag).await {
            Ok(redis::DelReply::KeyDeleted) => Ok(()),
            Ok(redis::DelReply::KeyNotDeleted) => {
                logger::error!("Tried to unlock a stream which is already unlocked");
                Ok(())
            }
            Err(error) => Err(errors::DrainerError::from(error).into()),
        }
    }

        /// Asynchronously reads a specified number of entries from a Redis stream with the given stream name.
    /// Returns a `DrainerResult` containing the result of the stream read operation, along with any potential errors.
    pub async fn read_from_stream(
        &self,
        stream_name: &str,
        max_read_count: u64,
    ) -> errors::DrainerResult<StreamReadResult> {
        // "0-0" id gives first entry
        let stream_id = "0-0";
        let (output, execution_time) = common_utils::date_time::time_it(|| async {
            self.redis_conn
                .stream_read_entries(stream_name, stream_id, Some(max_read_count))
                .await
                .map_err(errors::DrainerError::from)
                .into_report()
        })
        .await;

        metrics::REDIS_STREAM_READ_TIME.record(
            &metrics::CONTEXT,
            execution_time,
            &[metrics::KeyValue::new("stream", stream_name.to_owned())],
        );

        output
    }
        /// Asynchronously trims the Redis stream with the provided stream name by removing entries with IDs less than the specified minimum entry ID. Returns a DrainerResult containing the number of entries trimmed from the stream. Also, records the execution time of the trim operation in the Redis stream trim time metrics. 
    pub async fn trim_from_stream(
        &self,
        stream_name: &str,
        minimum_entry_id: &str,
    ) -> errors::DrainerResult<usize> {
        let trim_kind = redis::StreamCapKind::MinID;
        let trim_type = redis::StreamCapTrim::Exact;
        let trim_id = minimum_entry_id;
        let (trim_result, execution_time) =
            common_utils::date_time::time_it::<errors::DrainerResult<_>, _, _>(|| async {
                let trim_result = self
                    .redis_conn
                    .stream_trim_entries(stream_name, (trim_kind, trim_type, trim_id))
                    .await
                    .map_err(errors::DrainerError::from)
                    .into_report()?;

                // Since xtrim deletes entries below given id excluding the given id.
                // Hence, deleting the minimum entry id
                self.redis_conn
                    .stream_delete_entries(stream_name, minimum_entry_id)
                    .await
                    .map_err(errors::DrainerError::from)
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
}
