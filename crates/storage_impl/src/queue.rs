// use time::PrimitiveDateTime;
use redis_interface::{errors::RedisError, RedisEntryId, SetnxReply};

use common_utils::errors::CustomResult;
// use diesel_models::{enums, queue as storage};s
// use error_stack::{report, ResultExt};
use router_env::logger;
use sample::queue::QueueInterface;
use sample::RedisConnInterface;

use crate::{errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> QueueInterface for RouterStore<T> {
    type Error = errors::StorageError;

    // async fn fetch_consumer_tasks(
    //     &self,
    //     stream_name: &str,
    //     group_name: &str,
    //     consumer_name: &str,
    // ) -> CustomResult<Vec<storage::ProcessTracker>, ProcessTrackerError> {
    //     crate::consumer::fetch_consumer_tasks(
    //         self,
    //         &self
    //             .get_redis_conn()
    //             .map_err(ProcessTrackerError::ERedisError)?
    //             .clone(),
    //         stream_name,
    //         group_name,
    //         consumer_name,
    //     )
    //     .await
    // }

    async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), RedisError> {
        self.get_redis_conn()?
            .consumer_group_create(&stream.into(), group, id)
            .await
    }

    async fn acquire_pt_lock(
        &self,
        tag: &str,
        lock_key: &str,
        lock_val: &str,
        ttl: i64,
    ) -> CustomResult<bool, RedisError> {
        let conn = self.get_redis_conn()?.clone();
        let is_lock_acquired = conn
            .set_key_if_not_exists_with_expiry(&lock_key.into(), lock_val, None)
            .await;
        Ok(match is_lock_acquired {
            Ok(SetnxReply::KeySet) => match conn.set_expiry(&lock_key.into(), ttl).await {
                Ok(()) => true,

                #[allow(unused_must_use)]
                Err(error) => {
                    logger::error!(?error);
                    conn.delete_key(&lock_key.into()).await;
                    false
                }
            },
            Ok(SetnxReply::KeyNotSet) => {
                logger::error!(%tag, "Lock not acquired, previous fetch still in progress");
                false
            }
            Err(error) => {
                logger::error!(?error, %tag, "Error while locking");
                false
            }
        })
    }

    async fn release_pt_lock(&self, tag: &str, lock_key: &str) -> CustomResult<bool, RedisError> {
        let is_lock_released = self.get_redis_conn()?.delete_key(&lock_key.into()).await;
        Ok(match is_lock_released {
            Ok(_del_reply) => true,
            Err(error) => {
                logger::error!(?error, %tag, "Error while releasing lock");
                false
            }
        })
    }

    async fn stream_append_entry(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError> {
        self.get_redis_conn()?
            .stream_append_entry(&stream.into(), entry_id, fields)
            .await
    }

    async fn get_key(&self, key: &str) -> CustomResult<Vec<u8>, RedisError> {
        self.get_redis_conn()?.get_key::<Vec<u8>>(&key.into()).await
    }
}