use redis_interface::{errors::RedisError, RedisEntryId, SetNXReply};
use router_env::logger;

use super::{MockDb, Sqlx};
use crate::{
    core::errors::{CustomResult, ProcessTrackerError},
    scheduler::consumer::fetch_consumer_tasks,
    services::Store,
    types::storage::ProcessTracker,
};

#[async_trait::async_trait]
pub trait QueueInterface {
    async fn fetch_consumer_tasks(
        &self,
        stream_name: &str,
        group_name: &str,
        consumer_name: &str,
    ) -> CustomResult<Vec<ProcessTracker>, ProcessTrackerError>;

    async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), RedisError>;

    async fn acquire_pt_lock(&self, tag: &str, lock_key: &str, lock_val: &str, ttl: i64) -> bool;

    async fn release_pt_lock(&self, tag: &str, lock_key: &str) -> bool;

    async fn stream_append_entry(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError>;

    async fn get_key(&self, key: &str) -> CustomResult<Vec<u8>, RedisError>;
}

#[async_trait::async_trait]
impl QueueInterface for Store {
    async fn fetch_consumer_tasks(
        &self,
        stream_name: &str,
        group_name: &str,
        consumer_name: &str,
    ) -> CustomResult<Vec<ProcessTracker>, ProcessTrackerError> {
        fetch_consumer_tasks(
            self,
            &self.redis_conn.clone(),
            stream_name,
            group_name,
            consumer_name,
        )
        .await
    }

    async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), RedisError> {
        self.redis_conn
            .consumer_group_create(stream, group, id)
            .await
    }

    async fn acquire_pt_lock(&self, tag: &str, lock_key: &str, lock_val: &str, ttl: i64) -> bool {
        let conn = self.redis_conn.clone();
        let is_lock_acquired = conn.set_key_if_not_exist(lock_key, lock_val).await;
        match is_lock_acquired {
            Ok(SetNXReply::KeySet) => match conn.set_expiry(lock_key, ttl).await {
                Ok(()) => true,

                #[allow(unused_must_use)]
                Err(error) => {
                    logger::error!(error=?error.current_context());
                    conn.delete_key(lock_key).await;
                    false
                }
            },
            Ok(SetNXReply::KeyNotSet) => {
                logger::error!(%tag, "Lock not acquired, previous fetch still in progress");
                false
            }
            Err(error) => {
                logger::error!(error=%error.current_context(), %tag, "Error while locking");
                false
            }
        }
    }

    async fn release_pt_lock(&self, tag: &str, lock_key: &str) -> bool {
        let is_lock_released = self.redis_conn.delete_key(lock_key).await;
        match is_lock_released {
            Ok(()) => true,
            Err(error) => {
                logger::error!(error=%error.current_context(), %tag, "Error while releasing lock");
                false
            }
        }
    }

    async fn stream_append_entry(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError> {
        self.redis_conn
            .stream_append_entry(stream, entry_id, fields)
            .await
    }

    async fn get_key(&self, key: &str) -> CustomResult<Vec<u8>, RedisError> {
        self.redis_conn.get_key::<Vec<u8>>(key).await
    }
}

#[async_trait::async_trait]
impl QueueInterface for MockDb {
    async fn fetch_consumer_tasks(
        &self,
        stream_name: &str,
        group_name: &str,
        consumer_name: &str,
    ) -> CustomResult<Vec<ProcessTracker>, ProcessTrackerError> {
        todo!()
    }

    async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), RedisError> {
        todo!()
    }

    async fn acquire_pt_lock(&self, tag: &str, lock_key: &str, lock_val: &str, ttl: i64) -> bool {
        todo!()
    }

    async fn release_pt_lock(&self, tag: &str, lock_key: &str) -> bool {
        todo!()
    }

    async fn stream_append_entry(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError> {
        todo!()
    }

    async fn get_key(&self, key: &str) -> CustomResult<Vec<u8>, RedisError> {
        self.redis.get_key(key).await
    }
}

#[async_trait::async_trait]
impl QueueInterface for Sqlx {
    async fn fetch_consumer_tasks(
        &self,
        stream_name: &str,
        group_name: &str,
        consumer_name: &str,
    ) -> CustomResult<Vec<ProcessTracker>, ProcessTrackerError> {
        todo!()
    }

    async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), RedisError> {
        todo!()
    }

    async fn acquire_pt_lock(&self, tag: &str, lock_key: &str, lock_val: &str, ttl: i64) -> bool {
        todo!()
    }

    async fn release_pt_lock(&self, tag: &str, lock_key: &str) -> bool {
        todo!()
    }

    async fn stream_append_entry(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError> {
        todo!()
    }

    async fn get_key(&self, key: &str) -> CustomResult<Vec<u8>, RedisError> {
        todo!()
    }
}
