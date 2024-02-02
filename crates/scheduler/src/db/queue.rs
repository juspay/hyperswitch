use common_utils::errors::CustomResult;
use diesel_models::process_tracker as storage;
use redis_interface::{errors::RedisError, RedisEntryId, SetnxReply};
use router_env::logger;
use storage_impl::{mock_db::MockDb, redis::kv_store::RedisConnInterface};

use crate::{errors::ProcessTrackerError, scheduler::Store};

#[async_trait::async_trait]
pub trait QueueInterface {
    async fn fetch_consumer_tasks(
        &self,
        stream_name: &str,
        group_name: &str,
        consumer_name: &str,
    ) -> CustomResult<Vec<storage::ProcessTracker>, ProcessTrackerError>;

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

#[async_trait::async_trait]
impl QueueInterface for Store {
        /// Asynchronously fetches consumer tasks from the specified stream for the given group and consumer.
    /// 
    /// # Arguments
    /// 
    /// * `stream_name` - A string reference representing the name of the stream to fetch tasks from.
    /// * `group_name` - A string reference representing the name of the consumer group.
    /// * `consumer_name` - A string reference representing the name of the consumer.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `ProcessTracker` objects if successful, else a `ProcessTrackerError`.
    /// 
    async fn fetch_consumer_tasks(
        &self,
        stream_name: &str,
        group_name: &str,
        consumer_name: &str,
    ) -> CustomResult<Vec<storage::ProcessTracker>, ProcessTrackerError> {
        crate::consumer::fetch_consumer_tasks(
            self,
            &self
                .get_redis_conn()
                .map_err(ProcessTrackerError::ERedisError)?
                .clone(),
            stream_name,
            group_name,
            consumer_name,
        )
        .await
    }

        /// Asynchronously creates a consumer group for a given stream in Redis using the provided group name and RedisEntryId.
    ///
    /// # Arguments
    ///
    /// * `stream` - A string reference representing the name of the stream.
    /// * `group` - A string reference representing the name of the consumer group to be created.
    /// * `id` - A reference to a RedisEntryId, which is used to uniquely identify the consumer group.
    ///
    /// # Returns
    ///
    /// This method returns a CustomResult indicating success or an error of type RedisError.
    ///
    async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), RedisError> {
        self.get_redis_conn()?
            .consumer_group_create(stream, group, id)
            .await
    }

        /// Asynchronously acquires a lock in Redis for a given resource using the specified tag, lock key, lock value, and time-to-live (TTL) in seconds. Returns a CustomResult indicating whether the lock was successfully acquired or if an error occurred during the process.
    
    async fn acquire_pt_lock(
        &self,
        tag: &str,
        lock_key: &str,
        lock_val: &str,
        ttl: i64,
    ) -> CustomResult<bool, RedisError> {
        let conn = self.get_redis_conn()?.clone();
        let is_lock_acquired = conn
            .set_key_if_not_exists_with_expiry(lock_key, lock_val, None)
            .await;
        Ok(match is_lock_acquired {
            Ok(SetnxReply::KeySet) => match conn.set_expiry(lock_key, ttl).await {
                Ok(()) => true,
    
                #[allow(unused_must_use)]
                Err(error) => {
                    logger::error!(error=?error.current_context());
                    conn.delete_key(lock_key).await;
                    false
                }
            },
            Ok(SetnxReply::KeyNotSet) => {
                logger::error!(%tag, "Lock not acquired, previous fetch still in progress");
                false
            }
            Err(error) => {
                logger::error!(error=%error.current_context(), %tag, "Error while locking");
                false
            }
        })
    }

        /// Asynchronously releases a point lock in Redis using the provided lock key. Returns a CustomResult indicating whether the lock was successfully released or not, along with any potential RedisError encountered during the operation. If the lock is successfully released, the method returns true; otherwise, it logs an error using the provided tag and returns false.
    async fn release_pt_lock(&self, tag: &str, lock_key: &str) -> CustomResult<bool, RedisError> {
        let is_lock_released = self.get_redis_conn()?.delete_key(lock_key).await;
        Ok(match is_lock_released {
            Ok(_del_reply) => true,
            Err(error) => {
                logger::error!(error=%error.current_context(), %tag, "Error while releasing lock");
                false
            }
        })
    }

        /// Asynchronously appends an entry to a Redis stream with the specified stream name, entry ID, and fields. Returns a CustomResult indicating success or a RedisError if an error occurs.
    async fn stream_append_entry(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError> {
        self.get_redis_conn()?
            .stream_append_entry(stream, entry_id, fields)
            .await
    }

        /// Asynchronously retrieves a value from Redis using the provided key.
    ///
    /// # Arguments
    ///
    /// * `key` - A reference to the key string used to retrieve the value from Redis.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing a `Vec<u8>` representing the value associated with the given key, or a `RedisError` if the operation fails.
    ///
    async fn get_key(&self, key: &str) -> CustomResult<Vec<u8>, RedisError> {
        self.get_redis_conn()?.get_key::<Vec<u8>>(key).await
    }
}

#[async_trait::async_trait]
impl QueueInterface for MockDb {
        /// Asynchronously fetches consumer tasks for a given stream, group, and consumer name from the database.
    async fn fetch_consumer_tasks(
        &self,
        _stream_name: &str,
        _group_name: &str,
        _consumer_name: &str,
    ) -> CustomResult<Vec<storage::ProcessTracker>, ProcessTrackerError> {
        // [#172]: Implement function for `MockDb`
        Err(ProcessTrackerError::ResourceFetchingFailed {
            resource_name: "consumer_tasks",
        })?
    }

        /// Asynchronously creates a consumer group in Redis for the given stream using the provided group name and entry id.
    ///
    /// # Arguments
    ///
    /// * `stream` - A string representing the name of the stream in Redis.
    /// * `group` - A string representing the name of the consumer group to be created.
    /// * `id` - A reference to a RedisEntryId struct representing the entry id.
    ///
    /// # Returns
    ///
    /// Returns a CustomResult indicating success or a RedisError if the consumer group creation failed.
    ///

    async fn consumer_group_create(
        &self,
        _stream: &str,
        _group: &str,
        _id: &RedisEntryId,
    ) -> CustomResult<(), RedisError> {
        // [#172]: Implement function for `MockDb`
        Err(RedisError::ConsumerGroupCreateFailed)?
    }


        /// Asynchronously acquires a lock on a certain resource using the specified tag, lock key, lock value, and time-to-live (TTL) duration.
    /// 
    /// # Arguments
    /// * `tag` - The tag used to identify the resource.
    /// * `lock_key` - The key used to lock the resource.
    /// * `lock_val` - The value used to lock the resource.
    /// * `ttl` - The time-to-live duration for the lock.
    /// 
    /// # Returns
    /// A `CustomResult` indicating whether the lock was successfully acquired or not, along with any potential `RedisError`.
    /// 
    /// [#172]: Implement function for `MockDb`
    async fn acquire_pt_lock(
        &self,
        _tag: &str,
        _lock_key: &str,
        _lock_val: &str,
        _ttl: i64,
    ) -> CustomResult<bool, RedisError> {
        // [#172]: Implement function for `MockDb`
        Ok(false)
    }

        /// Asynchronously releases a lock associated with the given tag and lock key.
    ///
    /// This method is used to release a lock previously acquired by calling `acquire_pt_lock`.
    ///
    /// # Arguments
    ///
    /// * `_tag` - A reference to the tag associated with the lock to be released.
    /// * `_lock_key` - A reference to the key of the lock to be released.
    ///
    /// # Returns
    ///
    /// A `CustomResult` indicating whether the lock was successfully released.
    ///
    /// If the lock is successfully released, the method returns `Ok(false)`. If an error occurs during the release process, a `RedisError` is returned.
    async fn release_pt_lock(&self, _tag: &str, _lock_key: &str) -> CustomResult<bool, RedisError> {
        // [#172]: Implement function for `MockDb`
        Ok(false)
    }


        /// Asynchronously appends an entry to the specified Redis stream with the given entry ID and fields.
    /// 
    /// # Arguments
    /// * `stream` - A reference to the name of the Redis stream
    /// * `entry_id` - A reference to the unique identifier of the entry
    /// * `fields` - A vector of tuples containing the field names and values for the entry
    /// 
    /// # Returns
    /// * `CustomResult<(), RedisError>` - A custom result indicating success or a Redis error
    /// 
    /// # Errors
    /// Returns a `RedisError::StreamAppendFailed` if the stream append operation fails.
    async fn stream_append_entry(
        &self,
        _stream: &str,
        _entry_id: &RedisEntryId,
        _fields: Vec<(&str, String)>,
    ) -> CustomResult<(), RedisError> {
        // [#172]: Implement function for `MockDb`
        Err(RedisError::StreamAppendFailed)?
    }

        /// Asynchronous method to retrieve a key from the Redis database.
    /// 
    /// # Arguments
    /// 
    /// * `key` - A reference to the key string that needs to be retrieved from the Redis database.
    /// 
    /// # Returns
    /// 
    /// * `Result` - A `CustomResult` containing a vector of bytes if the operation is successful, or a `RedisError` if there is a connection error.
    /// 
    async fn get_key(&self, _key: &str) -> CustomResult<Vec<u8>, RedisError> {
        Err(RedisError::RedisConnectionError.into())
    }
}
