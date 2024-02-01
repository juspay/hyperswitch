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
use error_stack::{IntoReport, ResultExt};
use fred::{
    interfaces::{HashesInterface, KeysInterface, StreamsInterface},
    prelude::RedisErrorKind,
    types::{
        Expiration, FromRedis, MultipleIDs, MultipleKeys, MultipleOrderedPairs, MultipleStrings,
        RedisKey, RedisMap, RedisValue, Scanner, SetOptions, XCap, XReadResponse,
    },
};
use futures::StreamExt;
use router_env::{instrument, logger, tracing};

use crate::{
    errors,
    types::{DelReply, HsetnxReply, MsetnxReply, RedisEntryId, SetnxReply},
};

impl super::RedisConnectionPool {
    #[instrument(level = "DEBUG", skip(self))]
        /// Sets a key-value pair in the Redis database using the provided key and value. 
    /// Returns a CustomResult indicating success or an error of type errors::RedisError.
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
            .into_report()
            .change_context(errors::RedisError::SetFailed)
    }

        /// Asynchronously sets multiple keys in the Redis server if they do not already exist, along with their respective values. 
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be set, which must implement the `TryInto<RedisMap>`, `Debug`, `Send`, and `Sync` traits.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the result of the operation, along with any potential `RedisError`.
    ///
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
            .into_report()
            .change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Serializes the given value and sets it as the value for the specified key in Redis, if the key does not already exist.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set in Redis.
    /// * `value` - The value to be serialized and set in Redis.
    /// * `ttl` - An optional time-to-live (TTL) in seconds for the key-value pair.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `SetnxReply` if the operation was successful, otherwise a `RedisError`.
    ///
    pub async fn serialize_and_set_key_if_not_exist<V>(
        &self,
        key: &str,
        value: V,
        ttl: Option<i64>,
    ) -> CustomResult<SetnxReply, errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let serialized = Encode::<V>::encode_to_vec(&value)
            .change_context(errors::RedisError::JsonSerializationFailed)?;
        self.set_key_if_not_exists_with_expiry(key, serialized.as_slice(), ttl)
            .await
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously serializes the given value using serde and sets it as the value for the specified key in Redis.
    /// 
    /// # Arguments
    /// 
    /// * `key` - A string slice representing the key in Redis.
    /// * `value` - The value to be serialized and set as the value for the specified key in Redis.
    /// 
    /// # Returns
    /// 
    /// * A `CustomResult` containing a unit value `()` if successful, otherwise an `errors::RedisError`.
    /// 
    /// # Constraints
    /// 
    /// The generic type `V` must implement the `serde::Serialize` and `Debug` traits.
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

        self.set_key(key, serialized.as_slice()).await
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously serializes the given value and sets it in the Redis cache with the specified key and expiry time in seconds. 
    /// Returns a CustomResult indicating success or an errors::RedisError if the serialization or setting operation fails.
    pub async fn serialize_and_set_key_with_expiry<V>(
        &self,
        key: &str,
        value: V,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: serde::Serialize + Debug,
    {
        let serialized = Encode::<V>::encode_to_vec(&value)
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
            .into_report()
            .change_context(errors::RedisError::SetExFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously retrieves a value from Redis cache using the provided key. 
    /// If successful, returns a result containing the retrieved value. 
    /// If an error occurs during the retrieval process, returns a CustomResult with a RedisError containing the details of the failure.
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
        /// Asynchronously checks if a key exists in the Redis database using the provided key. Returns a `CustomResult` indicating whether the key exists or not, along with any potential errors that may occur during the operation.
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
        /// Asynchronously retrieves a value from a Redis key, deserializes it into the specified type using serde, and returns a CustomResult containing the deserialized value or a RedisError if the value is not found or deserialization fails.
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
        /// Asynchronously deletes a key from the Redis database using the provided key and returns a CustomResult containing the DelReply if successful, or an errors::RedisError if the deletion fails.
    pub async fn delete_key(&self, key: &str) -> CustomResult<DelReply, errors::RedisError> {
        self.pool
            .del(key)
            .await
            .into_report()
            .change_context(errors::RedisError::DeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Sets a key-value pair in Redis with an expiry time in seconds.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set in Redis
    /// * `value` - The value to associate with the key
    /// * `seconds` - The number of seconds until the key expires
    ///
    /// # Returns
    ///
    /// A `CustomResult` indicating success or an `errors::RedisError` if the operation fails
    ///
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
            .into_report()
            .change_context(errors::RedisError::SetExFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Sets the given key-value pair in the Redis server if the key does not already exist, with an optional expiry time in seconds.
    /// Returns a CustomResult containing a SetnxReply if successful, or a RedisError if the operation fails.
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
            .into_report()
            .change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Sets the expiry time for a key in the Redis database.
    ///
    /// # Arguments
    ///
    /// * `key` - A string slice representing the key for which the expiry time will be set.
    /// * `seconds` - An i64 representing the number of seconds after which the key will expire.
    ///
    /// # Returns
    ///
    /// * `CustomResult<(), errors::RedisError>` - A custom result indicating the success or failure of setting the expiry time for the key.
    ///
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
        /// Sets the expiration time for the specified key in Redis at the given timestamp.
    /// 
    /// # Arguments
    /// 
    /// * `key` - A reference to a string representing the key in Redis.
    /// * `timestamp` - An i64 representing the UNIX timestamp at which the key should expire.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing `()` if the expiration time was set successfully, otherwise an `errors::RedisError`.
    /// 
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
        /// Asynchronously sets the hash fields for a given key with the provided values and optional time-to-live (TTL).
    ///
    /// # Arguments
    ///
    /// * `key` - A string slice representing the key for the hash.
    /// * `values` - A type that can be converted into a RedisMap, implementing TryInto<RedisMap> + Debug + Send + Sync traits.
    /// * `ttl` - An optional i64 representing the time-to-live for the key in seconds.
    ///
    /// # Returns
    ///
    /// A CustomResult with a success value of () if the hash fields are set successfully, or a RedisError if an error occurs.
    ///
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
            .into_report()
            .change_context(errors::RedisError::SetHashFailed);
        // setting expiry for the key
        output
            .async_and_then(|_| {
                self.set_expiry(key, ttl.unwrap_or(self.config.default_hash_ttl.into()))
            })
            .await
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously sets a hash field in Redis if it does not already exist. If the field does not exist, it is set with the specified value and an optional time-to-live (TTL) value. If the TTL is not provided, the default hash TTL from the configuration is used. Returns a `CustomResult` containing the result of the operation or a `RedisError` if the operation fails.
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
            .into_report()
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
        /// Asynchronously serializes the given value and sets it as the value of the specified field in a hash if the field does not already exist.
    /// If the field exists, it does nothing. Returns a result indicating whether the operation was successful or an error occurred.
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
        let serialized = Encode::<V>::encode_to_vec(&value)
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        self.set_hash_field_if_not_exist(key, field, serialized.as_slice(), ttl)
            .await
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously fetches multiple values from a Redis database using the provided keys.
    ///
    /// # Arguments
    ///
    /// * `keys` - The keys for which to fetch the values
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `Vec` of `Option`s of the fetched values, or a `RedisError` if the operation fails.
    ///
    /// # Generic Types
    ///
    /// * `K` - The type of the keys
    /// * `V` - The type of the values
    ///
    /// # Constraints
    ///
    /// * `V` must implement `FromRedis`, be `Unpin`, `Send`, and have a static lifetime
    /// * `K` must implement `Into<MultipleKeys>`, be `Send`, and implement `Debug`
    ///
    pub async fn get_multiple_keys<K, V>(
        &self,
        keys: K,
    ) -> CustomResult<Vec<Option<V>>, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
        K: Into<MultipleKeys> + Send + Debug,
    {
        self.pool
            .mget(keys)
            .await
            .into_report()
            .change_context(errors::RedisError::GetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously retrieves and deserializes multiple keys from Redis. The method takes in a set of keys and a type name, and returns a `CustomResult` containing a vector of optional deserialized values. The keys are first retrieved from Redis as byte vectors, then each byte vector is deserialized into the specified type using the provided type name. Any deserialization errors are wrapped in a `RedisError` and returned as part of the `CustomResult`.
    pub async fn get_and_deserialize_multiple_keys<K, V>(
        &self,
        keys: K,
        type_name: &'static str,
    ) -> CustomResult<Vec<Option<V>>, errors::RedisError>
    where
        K: Into<MultipleKeys> + Send + Debug,
        V: serde::de::DeserializeOwned,
    {
        let data = self.get_multiple_keys::<K, Vec<u8>>(keys).await?;
        data.into_iter()
            .map(|value_bytes| {
                value_bytes
                    .map(|bytes| {
                        bytes
                            .parse_struct(type_name)
                            .change_context(errors::RedisError::JsonSerializationFailed)
                    })
                    .transpose()
            })
            .collect()
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously serializes and sets multiple hash fields if they do not already exist in Redis, using the provided key-value pairs. Each key-value pair is serialized and set as a hash field if it does not already exist, with an optional time-to-live (TTL) value. Returns a vector of HsetnxReply indicating the success of each operation.
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
        /// Asynchronously scans the hash set at the specified key in the Redis database using the given pattern,
    /// and returns a vector of strings containing the values that match the pattern.
    /// If a count is provided, only the specified number of elements will be returned.
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
        /// Asynchronously scans the specified hash key for fields matching the given pattern,
    /// deserializes the values into the specified type T, and returns a vector of the deserialized values.
    ///
    /// # Arguments
    ///
    /// * `key` - A reference to the key of the hash to be scanned.
    /// * `pattern` - A reference to the pattern to match the fields in the hash.
    /// * `count` - An optional parameter specifying the maximum number of elements to return.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of the deserialized values if successful,
    /// otherwise returns a `RedisError`.
    ///
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
        /// Asynchronously retrieves the value associated with the specified field in a hash stored at the specified key in Redis using the connection pool. 
    /// 
    /// # Arguments
    /// 
    /// * `key` - A reference to a string representing the key of the hash in Redis.
    /// * `field` - A reference to a string representing the field within the hash.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the value associated with the specified field, or an error of type `errors::RedisError` if the operation fails.
    /// 
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
        /// Asynchronously gets the value of a field from a hash stored in Redis, deserializes it into the specified type using serde, and returns the deserialized value.
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
        /// Asynchronously appends an entry to a Redis stream using the provided entry ID and fields. Returns a custom result indicating success or a Redis error.
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
            .into_report()
            .change_context(errors::RedisError::StreamAppendFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously deletes entries with the specified IDs from the given stream in the Redis database.
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
            .into_report()
            .change_context(errors::RedisError::StreamDeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously trims the entries of a Redis stream based on the provided xcap (approximate number of elements in the stream).
    /// Returns a CustomResult containing the number of entries trimmed or an error of type errors::RedisError.
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
            .into_report()
            .change_context(errors::RedisError::StreamTrimFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously acknowledges the specified entries within a given consumer group of a stream.
    /// 
    /// # Arguments
    /// 
    /// * `stream` - A reference to the name of the stream.
    /// * `group` - A reference to the name of the consumer group.
    /// * `ids` - The IDs of the entries to be acknowledged.
    /// 
    /// # Returns
    /// 
    /// A custom result containing the number of acknowledged entries or a `RedisError` if the acknowledgment failed.
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
            .into_report()
            .change_context(errors::RedisError::StreamAcknowledgeFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously retrieves the length of a Redis stream using the provided key.
    ///
    /// # Arguments
    ///
    /// * `stream` - The key of the Redis stream whose length needs to be retrieved.
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the length of the stream if successful, or a `RedisError` if the operation fails.
    pub async fn stream_get_length<K>(&self, stream: K) -> CustomResult<usize, errors::RedisError>
    where
        K: Into<RedisKey> + Debug + Send + Sync,
    {
        self.pool
            .xlen(stream)
            .await
            .into_report()
            .change_context(errors::RedisError::GetLengthFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously reads entries from one or more Redis streams identified by the given keys and IDs. The method allows specifying the number of entries to read from each stream. If the `read_count` parameter is not provided, it defaults to the value specified in the configuration. 
    /// 
    /// # Arguments
    /// * `streams` - A type that can be converted into multiple stream keys.
    /// * `ids` - A type that can be converted into multiple stream IDs.
    /// * `read_count` - An optional parameter specifying the number of entries to read from each stream.
    /// 
    /// # Returns
    /// The method returns a `CustomResult` containing an `XReadResponse` with the read entries if successful. Otherwise, it returns a `RedisError`.
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
            .into_report()
            .map_err(|err| match err.current_context().kind() {
                RedisErrorKind::NotFound | RedisErrorKind::Parse => {
                    err.change_context(errors::RedisError::StreamEmptyOrNotAvailable)
                }
                _ => err.change_context(errors::RedisError::StreamReadFailed),
            })
    }

    #[instrument(level = "DEBUG", skip(self))]
        /// Asynchronously reads from the specified Redis streams using the provided options.
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
        .into_report()
        .change_context(errors::RedisError::StreamReadFailed)
    }

    //                                              Consumer Group API

    #[instrument(level = "DEBUG", skip(self))]
        /// Creates a new consumer group on the specified stream in Redis. It takes the name of the stream, the name of the group to create, and an id for the consumer group. If the id is either AutoGeneratedID or UndeliveredEntryID, it will return an error. Otherwise, it will create the consumer group and return the result.
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
        /// Destroys a consumer group for a given stream in the Redis server.
    ///
    /// # Arguments
    ///
    /// * `stream` - A reference to a string representing the name of the stream.
    /// * `group` - A reference to a string representing the name of the consumer group to destroy.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the number of pending messages in the group if successful, otherwise a `RedisError`.
    ///
    /// # Errors
    ///
    /// If the consumer group destruction fails, a `RedisError` with context `ConsumerGroupDestroyFailed` is returned.
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
        /// Deletes a consumer from a specific consumer group within a Redis stream.
    ///
    /// # Arguments
    ///
    /// * `stream` - The name of the stream from which to remove the consumer.
    /// * `group` - The name of the consumer group from which to remove the consumer.
    /// * `consumer` - The name of the consumer to be removed.
    ///
    /// # Returns
    ///
    /// If successful, returns the number of pending messages that were removed along with the consumer. If an error occurs, returns a `RedisError`.
    ///
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
        /// Asynchronously sets the last message ID for a consumer group within a Redis stream.
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
        /// Asynchronously sets the message owner for a consumer group in Redis. This method claims ownership of pending messages in the specified consumer group within a given stream, and sets the minimum idle time for claimed messages before they can be re-claimed. It returns a CustomResult containing the result of the operation or a RedisError if the operation fails.
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
            .into_report()
            .change_context(errors::RedisError::ConsumerGroupClaimFailed)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use crate::{errors::RedisError, RedisConnectionPool, RedisEntryId, RedisSettings};

    #[tokio::test]
        /// Asynchronously creates consumer groups in Redis and checks for invalid Redis entry errors.
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
        /// Asynchronously tests the successful deletion of an existing key from a Redis connection pool.
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
        /// Asynchronously tests the successful deletion of a non-existing key from the Redis database.
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
