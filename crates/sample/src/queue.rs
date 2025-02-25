use common_utils::errors::CustomResult;
// use diesel_models::process_tracker as storage;
use redis_interface::{errors::RedisError, RedisEntryId};
// use router_env::logger;
// use storage_impl::{mock_db::MockDb, redis::kv_store::RedisConnInterface};

// use crate::{errors::ProcessTrackerError, scheduler::Store};

#[async_trait::async_trait]
pub trait QueueInterface {
    type Error;
    // async fn fetch_consumer_tasks(
    //     &self,
    //     stream_name: &str,
    //     group_name: &str,
    //     consumer_name: &str,
    // ) -> CustomResult<Vec<storage::ProcessTracker>, Self::Error>;

    async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), RedisError>;

    async fn acquire_pt_lock(
        &self,
        tag: &str,
        lock_key: &str,
        lock_val: &str,
        ttl: i64,
    ) -> CustomResult<bool, RedisError>;

    async fn release_pt_lock(&self, tag: &str, lock_key: &str) -> CustomResult<bool, RedisError>;

    async fn stream_append_entry(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError>;

    async fn get_key(&self, key: &str) -> CustomResult<Vec<u8>, RedisError>;
}