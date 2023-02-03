use error_stack::ResultExt;
use redis_interface::RedisEntryId;

use crate::{core::errors, services::Store, types::storage::kv, utils::storage_partitioning};

#[cfg(feature = "kv_store")]
/// Generates hscan field pattern. Suppose the field is pa_1234_ref_1211 it will generate
/// pa_1234_ref_*
pub fn generate_hscan_pattern_for_refund(sk: &str) -> String {
    sk.split('_')
        .take(3)
        .chain(["*"])
        .collect::<Vec<&str>>()
        .join("_")
}

#[cfg(feature = "kv_store")]
pub(crate) async fn push_to_drainer_stream<T>(
    store: &Store,
    redis_entry: kv::TypedSql,
    partition_key: storage_partitioning::PartitionKey<'_>,
) -> errors::CustomResult<(), errors::StorageError>
where
    T: storage_partitioning::KvStorePartition,
{
    let shard_key = T::shard_key(partition_key, store.config.drainer_num_partitions);
    let stream_name = store.get_drainer_stream_name(&shard_key);
    store
        .redis_conn
        .stream_append_entry(
            &stream_name,
            &RedisEntryId::AutoGeneratedID,
            redis_entry
                .to_field_value_pairs()
                .change_context(errors::StorageError::KVError)?,
        )
        .await
        .change_context(errors::StorageError::KVError)
}
