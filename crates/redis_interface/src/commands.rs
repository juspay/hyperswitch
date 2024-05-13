//! An interface to abstract the `fred` commands
//!
//! The folder provides generic functions for providing serialization
//! and deserialization while calling redis.
//! It also includes instruments to provide tracing.
//!
//!

use std::fmt::Debug;

use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, ByteSliceExt, Encode, StringExt},
    fp_utils,
};
use error_stack::{report, ResultExt};
use fred::{
    interfaces::{HashesInterface, KeysInterface, SetsInterface, StreamsInterface},
    prelude::RedisErrorKind,
    types::{
        Expiration, FromRedis, MultipleIDs, MultipleKeys, MultipleOrderedPairs, MultipleStrings,
        MultipleValues, RedisKey, RedisMap, RedisValue, Scanner, SetOptions, XCap, XReadResponse,
    },
};
use futures::StreamExt;
use router_env::{instrument, logger, tracing};

use crate::{
    errors,
    types::{DelReply, HsetnxReply, MsetnxReply, RedisEntryId, SaddReply, SetnxReply},
};

impl super::RedisConnectionPool {
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key<V>(&self, key: &str, value: V) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .set(
                key,
                value,
                Some(Expiration::EX(self.config.default_ttl.into())),
                None,
                false,
            )
            .await
            .change_context(errors::RedisError::SetFailed)
    }

    pub async fn set_key_without_modifying_ttl<V>(
        &self,
        key: &str,
        value: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .set(key, value, Some(Expiration::KEEPTTL), None, false)
            .await
            .change_context(errors::RedisError::SetFailed)
    }

    pub async fn set_multiple_keys_if_not_exist<V>(
        &self,
        value: V,
    ) -> CustomResult<MsetnxReply, errors::RedisError>
    where
        V: TryInto<RedisMap> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .msetnx(value)
            .await
            .change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn serialize_and_set_key_if_not_exist<V>(
        &self,
        key: &str,
        value: V,
        ttl: Option<i64>,
    ) -> CustomResult<SetnxReply, errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;
        self.set_key_if_not_exists_with_expiry(key, serialized.as_slice(), ttl)
            .await
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn serialize_and_set_key<V>(
        &self,
        key: &str,
        value: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        self.set_key(key, serialized.as_slice()).await
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn serialize_and_set_key_without_modifying_ttl<V>(
        &self,
        key: &str,
        value: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        self.set_key_without_modifying_ttl(key, serialized.as_slice())
            .await
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn serialize_and_set_key_with_expiry<V>(
        &self,
        key: &str,
        value: V,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        self.pool
            .set(
                key,
                serialized.as_slice(),
                Some(Expiration::EX(seconds)),
                None,
                false,
            )
            .await
            .change_context(errors::RedisError::SetExFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_key<V>(&self, key: &str) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        self.pool
            .get(key)
            .await
            .change_context(errors::RedisError::GetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn exists<V>(&self, key: &str) -> CustomResult<bool, errors::RedisError>
    where
        V: Into<MultipleKeys> + Unpin + Send + 'static,
    {
        self.pool
            .exists(key)
            .await
            .change_context(errors::RedisError::GetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_and_deserialize_key<T>(
        &self,
        key: &str,
        type_name: &'static str,
    ) -> CustomResult<T, errors::RedisError>
    where
        T: serde::de::DeserializeOwned,
    {
        let value_bytes = self.get_key::<Vec<u8>>(key).await?;

        fp_utils::when(value_bytes.is_empty(), || Err(errors::RedisError::NotFound))?;

        value_bytes
            .parse_struct(type_name)
            .change_context(errors::RedisError::JsonDeserializationFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn delete_key(&self, key: &str) -> CustomResult<DelReply, errors::RedisError> {
        self.pool
            .del(key)
            .await
            .change_context(errors::RedisError::DeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key_with_expiry<V>(
        &self,
        key: &str,
        value: V,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .set(key, value, Some(Expiration::EX(seconds)), None, false)
            .await
            .change_context(errors::RedisError::SetExFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key_if_not_exists_with_expiry<V>(
        &self,
        key: &str,
        value: V,
        seconds: Option<i64>,
    ) -> CustomResult<SetnxReply, errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .set(
                key,
                value,
                Some(Expiration::EX(
                    seconds.unwrap_or(self.config.default_ttl.into()),
                )),
                Some(SetOptions::NX),
                false,
            )
            .await
            .change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expiry(
        &self,
        key: &str,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError> {
        self.pool
            .expire(key, seconds)
            .await
            .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expire_at(
        &self,
        key: &str,
        timestamp: i64,
    ) -> CustomResult<(), errors::RedisError> {
        self.pool
            .expire_at(key, timestamp)
            .await
            .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_hash_fields<V>(
        &self,
        key: &str,
        values: V,
        ttl: Option<i64>,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisMap> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let output: Result<(), _> = self
            .pool
            .hset(key, values)
            .await
            .change_context(errors::RedisError::SetHashFailed);
        // setting expiry for the key
        output
            .async_and_then(|_| {
                self.set_expiry(key, ttl.unwrap_or(self.config.default_hash_ttl.into()))
            })
            .await
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_hash_field_if_not_exist<V>(
        &self,
        key: &str,
        field: &str,
        value: V,
        ttl: Option<u32>,
    ) -> CustomResult<HsetnxReply, errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let output: Result<HsetnxReply, _> = self
            .pool
            .hsetnx(key, field, value)
            .await
            .change_context(errors::RedisError::SetHashFieldFailed);

        output
            .async_and_then(|inner| async {
                self.set_expiry(key, ttl.unwrap_or(self.config.default_hash_ttl).into())
                    .await?;
                Ok(inner)
            })
            .await
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn serialize_and_set_hash_field_if_not_exist<V>(
        &self,
        key: &str,
        field: &str,
        value: V,
        ttl: Option<u32>,
    ) -> CustomResult<HsetnxReply, errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        self.set_hash_field_if_not_exist(key, field, serialized.as_slice(), ttl)
            .await
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn serialize_and_set_multiple_hash_field_if_not_exist<V>(
        &self,
        kv: &[(&str, V)],
        field: &str,
        ttl: Option<u32>,
    ) -> CustomResult<Vec<HsetnxReply>, errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let mut hsetnx: Vec<HsetnxReply> = Vec::with_capacity(kv.len());
        for (key, val) in kv {
            hsetnx.push(
                self.serialize_and_set_hash_field_if_not_exist(key, field, val, ttl)
                    .await?,
            );
        }
        Ok(hsetnx)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn hscan(
        &self,
        key: &str,
        pattern: &str,
        count: Option<u32>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        Ok(self
            .pool
            .next()
            .hscan::<&str, &str>(key, pattern, count)
            .filter_map(|value| async move {
                match value {
                    Ok(mut v) => {
                        let v = v.take_results()?;

                        let v: Vec<String> =
                            v.iter().filter_map(|(_, val)| val.as_string()).collect();
                        Some(futures::stream::iter(v))
                    }
                    Err(err) => {
                        logger::error!(?err);
                        None
                    }
                }
            })
            .flatten()
            .collect::<Vec<_>>()
            .await)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn hscan_and_deserialize<T>(
        &self,
        key: &str,
        pattern: &str,
        count: Option<u32>,
    ) -> CustomResult<Vec<T>, errors::RedisError>
    where
        T: serde::de::DeserializeOwned,
    {
        let redis_results = self.hscan(key, pattern, count).await?;
        Ok(redis_results
            .iter()
            .filter_map(|v| {
                let r: T = v.parse_struct(std::any::type_name::<T>()).ok()?;
                Some(r)
            })
            .collect())
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_hash_field<V>(
        &self,
        key: &str,
        field: &str,
    ) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        self.pool
            .hget(key, field)
            .await
            .change_context(errors::RedisError::GetHashFieldFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_hash_field_and_deserialize<V>(
        &self,
        key: &str,
        field: &str,
        type_name: &'static str,
    ) -> CustomResult<V, errors::RedisError>
    where
        V: serde::de::DeserializeOwned,
    {
        let value_bytes = self.get_hash_field::<Vec<u8>>(key, field).await?;

        if value_bytes.is_empty() {
            return Err(errors::RedisError::NotFound.into());
        }

        value_bytes
            .parse_struct(type_name)
            .change_context(errors::RedisError::JsonDeserializationFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn sadd<V>(
        &self,
        key: &str,
        members: V,
    ) -> CustomResult<SaddReply, errors::RedisError>
    where
        V: TryInto<MultipleValues> + Debug + Send,
        V::Error: Into<fred::error::RedisError> + Send,
    {
        self.pool
            .sadd(key, members)
            .await
            .change_context(errors::RedisError::SetAddMembersFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_append_entry<F>(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: F,
    ) -> CustomResult<(), errors::RedisError>
    where
        F: TryInto<MultipleOrderedPairs> + Debug + Send + Sync,
        F::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .xadd(stream, false, None, entry_id, fields)
            .await
            .change_context(errors::RedisError::StreamAppendFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_delete_entries<Ids>(
        &self,
        stream: &str,
        ids: Ids,
    ) -> CustomResult<usize, errors::RedisError>
    where
        Ids: Into<MultipleStrings> + Debug + Send + Sync,
    {
        self.pool
            .xdel(stream, ids)
            .await
            .change_context(errors::RedisError::StreamDeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_trim_entries<C>(
        &self,
        stream: &str,
        xcap: C,
    ) -> CustomResult<usize, errors::RedisError>
    where
        C: TryInto<XCap> + Debug + Send + Sync,
        C::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .xtrim(stream, xcap)
            .await
            .change_context(errors::RedisError::StreamTrimFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_acknowledge_entries<Ids>(
        &self,
        stream: &str,
        group: &str,
        ids: Ids,
    ) -> CustomResult<usize, errors::RedisError>
    where
        Ids: Into<MultipleIDs> + Debug + Send + Sync,
    {
        self.pool
            .xack(stream, group, ids)
            .await
            .change_context(errors::RedisError::StreamAcknowledgeFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_get_length<K>(&self, stream: K) -> CustomResult<usize, errors::RedisError>
    where
        K: Into<RedisKey> + Debug + Send + Sync,
    {
        self.pool
            .xlen(stream)
            .await
            .change_context(errors::RedisError::GetLengthFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_entries<K, Ids>(
        &self,
        streams: K,
        ids: Ids,
        read_count: Option<u64>,
    ) -> CustomResult<XReadResponse<String, String, String, String>, errors::RedisError>
    where
        K: Into<MultipleKeys> + Debug + Send + Sync,
        Ids: Into<MultipleIDs> + Debug + Send + Sync,
    {
        self.pool
            .xread_map(
                Some(read_count.unwrap_or(self.config.default_stream_read_count)),
                None,
                streams,
                ids,
            )
            .await
            .map_err(|err| match err.kind() {
                RedisErrorKind::NotFound | RedisErrorKind::Parse => {
                    report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
                }
                _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
            })
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_with_options<K, Ids>(
        &self,
        streams: K,
        ids: Ids,
        count: Option<u64>,
        block: Option<u64>,          // timeout in milliseconds
        group: Option<(&str, &str)>, // (group_name, consumer_name)
    ) -> CustomResult<XReadResponse<String, String, String, Option<String>>, errors::RedisError>
    where
        K: Into<MultipleKeys> + Debug + Send + Sync,
        Ids: Into<MultipleIDs> + Debug + Send + Sync,
    {
        match group {
            Some((group_name, consumer_name)) => {
                self.pool
                    .xreadgroup_map(group_name, consumer_name, count, block, false, streams, ids)
                    .await
            }
            None => self.pool.xread_map(count, block, streams, ids).await,
        }
        .map_err(|err| match err.kind() {
            RedisErrorKind::NotFound | RedisErrorKind::Parse => {
                report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
            }
            _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
        })
    }

    //                                              Consumer Group API

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_create(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), errors::RedisError> {
        if matches!(
            id,
            RedisEntryId::AutoGeneratedID | RedisEntryId::UndeliveredEntryID
        ) {
            // FIXME: Replace with utils::when
            Err(errors::RedisError::InvalidRedisEntryId)?;
        }

        self.pool
            .xgroup_create(stream, group, id, true)
            .await
            .change_context(errors::RedisError::ConsumerGroupCreateFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_destroy(
        &self,
        stream: &str,
        group: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        self.pool
            .xgroup_destroy(stream, group)
            .await
            .change_context(errors::RedisError::ConsumerGroupDestroyFailed)
    }

    // the number of pending messages that the consumer had before it was deleted
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_delete_consumer(
        &self,
        stream: &str,
        group: &str,
        consumer: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        self.pool
            .xgroup_delconsumer(stream, group, consumer)
            .await
            .change_context(errors::RedisError::ConsumerGroupRemoveConsumerFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_set_last_id(
        &self,
        stream: &str,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<String, errors::RedisError> {
        self.pool
            .xgroup_setid(stream, group, id)
            .await
            .change_context(errors::RedisError::ConsumerGroupSetIdFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_set_message_owner<Ids, R>(
        &self,
        stream: &str,
        group: &str,
        consumer: &str,
        min_idle_time: u64,
        ids: Ids,
    ) -> CustomResult<R, errors::RedisError>
    where
        Ids: Into<MultipleIDs> + Debug + Send + Sync,
        R: FromRedis + Unpin + Send + 'static,
    {
        self.pool
            .xclaim(
                stream,
                group,
                consumer,
                min_idle_time,
                ids,
                None,
                None,
                None,
                false,
                false,
            )
            .await
            .change_context(errors::RedisError::ConsumerGroupClaimFailed)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use crate::{errors::RedisError, RedisConnectionPool, RedisEntryId, RedisSettings};

    #[tokio::test]
    async fn test_consumer_group_create() {
        let is_invalid_redis_entry_error = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let redis_conn = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");

                // Act
                let result1 = redis_conn
                    .consumer_group_create("TEST1", "GTEST", &RedisEntryId::AutoGeneratedID)
                    .await;

                let result2 = redis_conn
                    .consumer_group_create("TEST3", "GTEST", &RedisEntryId::UndeliveredEntryID)
                    .await;

                // Assert Setup
                *result1.unwrap_err().current_context() == RedisError::InvalidRedisEntryId
                    && *result2.unwrap_err().current_context() == RedisError::InvalidRedisEntryId
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_invalid_redis_entry_error);
    }

    #[tokio::test]
    async fn test_delete_existing_key_success() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let _ = pool.set_key("key", "value".to_string()).await;

                // Act
                let result = pool.delete_key("key").await;

                // Assert setup
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_delete_non_existing_key_success() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");

                // Act
                let result = pool.delete_key("key not exists").await;

                // Assert Setup
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }
}
