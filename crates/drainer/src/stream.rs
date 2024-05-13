use std::collections::HashMap;

use redis_interface as redis;
use router_env::{logger, tracing};

use crate::{errors, metrics, Store};

pub type StreamEntries = Vec<(String, HashMap<String, String>)>;
pub type StreamReadResult = HashMap<String, StreamEntries>;

impl Store {
    #[inline(always)]
    pub fn drainer_stream(&self, shard_key: &str) -> String {
        // Example: {shard_5}_drainer_stream
        format!("{{{}}}_{}", shard_key, self.config.drainer_stream_name,)
    }

    #[inline(always)]
    pub(crate) fn get_stream_key_flag(&self, stream_index: u8) -> String {
        format!("{}_in_use", self.get_drainer_stream_name(stream_index))
    }

    #[inline(always)]
    pub(crate) fn get_drainer_stream_name(&self, stream_index: u8) -> String {
        self.drainer_stream(format!("shard_{stream_index}").as_str())
    }

    #[router_env::instrument(skip_all)]
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
        })
        .await;

        metrics::REDIS_STREAM_READ_TIME.record(
            &metrics::CONTEXT,
            execution_time,
            &[metrics::KeyValue::new("stream", stream_name.to_owned())],
        );

        Ok(output?)
    }
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
                    .map_err(errors::DrainerError::from)?;

                // Since xtrim deletes entries below given id excluding the given id.
                // Hence, deleting the minimum entry id
                self.redis_conn
                    .stream_delete_entries(stream_name, minimum_entry_id)
                    .await
                    .map_err(errors::DrainerError::from)?;

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
