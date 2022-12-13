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
    ext_traits::{ByteSliceExt, Encode},
};
use error_stack::{IntoReport, ResultExt};
use fred::{
    interfaces::{HashesInterface, KeysInterface, StreamsInterface},
    types::{
        Expiration, FromRedis, MultipleIDs, MultipleKeys, MultipleOrderedPairs, MultipleStrings,
        RedisKey, RedisMap, RedisValue, SetOptions, XCap, XReadResponse,
    },
};
use router_env::{tracing, tracing::instrument};

use crate::{
    errors,
    types::{HsetnxReply, RedisEntryId, SetnxReply},
};

impl super::RedisConnectionPool {
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key<V>(&self, key: &str, value: V) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug,
        V::Error: Into<fred::error::RedisError>,
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
            .into_report()
            .change_context(errors::RedisError::SetFailed)
    }

    pub async fn msetnx<V>(&self, value: V) -> CustomResult<u8, errors::RedisError>
    where
        V: TryInto<RedisMap> + Debug,
        V::Error: Into<fred::error::RedisError>,
    {
        self.pool
            .msetnx::<u8, V>(value)
            .await
            .into_report()
            .change_context(errors::RedisError::SetFailed)
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
        let serialized = Encode::<V>::encode_to_vec(&value)
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        self.set_key(key, &serialized as &[u8]).await
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_key<V>(&self, key: &str) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        self.pool
            .get(key)
            .await
            .into_report()
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
            .into_report()
            .change_context(errors::RedisError::GetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_and_deserialize_key<T>(
        &self,
        key: &str,
        type_name: &str,
    ) -> CustomResult<T, errors::RedisError>
    where
        T: serde::de::DeserializeOwned,
    {
        let value_bytes = self.get_key::<Vec<u8>>(key).await?;

        value_bytes
            .parse_struct(type_name)
            .change_context(errors::RedisError::JsonDeserializationFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn delete_key(&self, key: &str) -> CustomResult<(), errors::RedisError> {
        self.pool
            .del(key)
            .await
            .into_report()
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
        V: TryInto<RedisValue> + Debug,
        V::Error: Into<fred::error::RedisError>,
    {
        self.pool
            .set(key, value, Some(Expiration::EX(seconds)), None, false)
            .await
            .into_report()
            .change_context(errors::RedisError::SetExFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key_if_not_exist<V>(
        &self,
        key: &str,
        value: V,
    ) -> CustomResult<SetnxReply, errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug,
        V::Error: Into<fred::error::RedisError>,
    {
        self.pool
            .set(
                key,
                value,
                Some(Expiration::EX(self.config.default_ttl.into())),
                Some(SetOptions::NX),
                false,
            )
            .await
            .into_report()
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
            .into_report()
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
            .into_report()
            .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_hash_fields<V>(
        &self,
        key: &str,
        values: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisMap> + Debug,
        V::Error: Into<fred::error::RedisError>,
    {
        self.pool
            .hset(key, values)
            .await
            .into_report()
            .change_context(errors::RedisError::SetHashFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_hash_field_if_not_exist<V>(
        &self,
        key: &str,
        field: &str,
        value: V,
    ) -> CustomResult<HsetnxReply, errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug,
        V::Error: Into<fred::error::RedisError>,
    {
        self.pool
            .hsetnx(key, field, value)
            .await
            .into_report()
            .change_context(errors::RedisError::SetHashFieldFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn serialize_and_set_hash_field_if_not_exist<V>(
        &self,
        key: &str,
        field: &str,
        value: V,
    ) -> CustomResult<HsetnxReply, errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let serialized = Encode::<V>::encode_to_vec(&value)
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        self.set_hash_field_if_not_exist(key, field, &serialized as &[u8])
            .await
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
            .into_report()
            .change_context(errors::RedisError::GetHashFieldFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_hash_field_and_deserialize<V>(
        &self,
        key: &str,
        field: &str,
        type_name: &str,
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
    pub async fn stream_append_entry<F>(
        &self,
        stream: &str,
        entry_id: &RedisEntryId,
        fields: F,
    ) -> CustomResult<(), errors::RedisError>
    where
        F: TryInto<MultipleOrderedPairs> + Debug,
        F::Error: Into<fred::error::RedisError>,
    {
        self.pool
            .xadd(stream, false, None, entry_id, fields)
            .await
            .into_report()
            .change_context(errors::RedisError::StreamAppendFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_delete_entries<Ids>(
        &self,
        stream: &str,
        ids: Ids,
    ) -> CustomResult<usize, errors::RedisError>
    where
        Ids: Into<MultipleStrings> + Debug,
    {
        self.pool
            .xdel(stream, ids)
            .await
            .into_report()
            .change_context(errors::RedisError::StreamDeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_trim_entries<C>(
        &self,
        stream: &str,
        xcap: C,
    ) -> CustomResult<usize, errors::RedisError>
    where
        C: TryInto<XCap> + Debug,
        C::Error: Into<fred::error::RedisError>,
    {
        self.pool
            .xtrim(stream, xcap)
            .await
            .into_report()
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
        Ids: Into<MultipleIDs> + Debug,
    {
        self.pool
            .xack(stream, group, ids)
            .await
            .into_report()
            .change_context(errors::RedisError::StreamAcknowledgeFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_get_length<K>(&self, stream: K) -> CustomResult<usize, errors::RedisError>
    where
        K: Into<RedisKey> + Debug,
    {
        self.pool
            .xlen(stream)
            .await
            .into_report()
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
        K: Into<MultipleKeys> + Debug,
        Ids: Into<MultipleIDs> + Debug,
    {
        self.pool
            .xread_map(
                Some(read_count.unwrap_or(self.config.default_stream_read_count)),
                None,
                streams,
                ids,
            )
            .await
            .into_report()
            .change_context(errors::RedisError::StreamReadFailed)
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
        K: Into<MultipleKeys> + Debug,
        Ids: Into<MultipleIDs> + Debug,
    {
        match group {
            Some((group_name, consumer_name)) => {
                self.pool
                    .xreadgroup_map(group_name, consumer_name, count, block, false, streams, ids)
                    .await
            }
            None => self.pool.xread_map(count, block, streams, ids).await,
        }
        .into_report()
        .change_context(errors::RedisError::StreamReadFailed)
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
            .into_report()
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
            .into_report()
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
            .into_report()
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
            .into_report()
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
        Ids: Into<MultipleIDs> + Debug,
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
            .into_report()
            .change_context(errors::RedisError::ConsumerGroupClaimFailed)
    }
}

#[cfg(test)]
mod tests {

    use crate::{errors::RedisError, RedisConnectionPool, RedisEntryId, RedisSettings};

    #[tokio::test]
    async fn test_consumer_group_create() {
        let redis_conn = RedisConnectionPool::new(&RedisSettings::default()).await;

        let result1 = redis_conn
            .consumer_group_create("TEST1", "GTEST", &RedisEntryId::AutoGeneratedID)
            .await;
        let result2 = redis_conn
            .consumer_group_create("TEST3", "GTEST", &RedisEntryId::UndeliveredEntryID)
            .await;

        assert!(matches!(
            result1.unwrap_err().current_context(),
            RedisError::InvalidRedisEntryId
        ));
        assert!(matches!(
            result2.unwrap_err().current_context(),
            RedisError::InvalidRedisEntryId
        ));
    }
}
