//! An interface to abstract the `fred` commands
//!
//! The folder provides generic functions for providing serialization
//! and deserialization while calling redis.
//! It also includes instruments to provide tracing.

use std::fmt::Debug;

use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, ByteSliceExt, Encode, StringExt},
    fp_utils,
};
use error_stack::{report, ResultExt};
use fred::{
    interfaces::{HashesInterface, KeysInterface, ListInterface, SetsInterface, StreamsInterface},
    prelude::{LuaInterface, RedisErrorKind},
    types::{
        Expiration, FromRedis, MultipleIDs, MultipleKeys, MultipleOrderedPairs, MultipleStrings,
        MultipleValues, RedisMap, RedisValue, Scanner, SetOptions, XReadResponse,
    },
};
use tracing::instrument;

use crate::{
    errors,
    types::{
        DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisKey, SaddReply, SetGetReply,
        SetnxReply, StreamEntries, StreamReadResult, StreamTrimConfig,
    },
};

impl super::RedisConnectionPool {
    pub fn add_prefix(&self, key: &str) -> String {
        if self.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}:{}", self.key_prefix, key)
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key<V>(&self, key: &RedisKey, value: V) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .set(
                key.tenant_aware_key(self),
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
        key: &RedisKey,
        value: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .set(
                key.tenant_aware_key(self),
                value,
                Some(Expiration::KEEPTTL),
                None,
                false,
            )
            .await
            .change_context(errors::RedisError::SetFailed)
    }

    pub async fn set_multiple_keys_if_not_exist<K, V>(
        &self,
        key_value_pairs: &[(K, V)],
    ) -> CustomResult<MsetnxReply, errors::RedisError>
    where
        K: Clone + Into<String> + Debug + Send + Sync,
        V: Clone + Into<String> + Debug + Send + Sync,
    {
        let pairs: Vec<(String, String)> = key_value_pairs
            .iter()
            .map(|(k, v)| (k.clone().into(), v.clone().into()))
            .collect();

        let map = RedisMap::try_from(pairs)
            .change_context(errors::RedisError::SetFailed)
            .attach_printable("Failed to convert key-value pairs to fred::types::RedisMap")?;

        self.pool
            .msetnx(map)
            .await
            .change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn serialize_and_set_key_if_not_exist<V>(
        &self,
        key: &RedisKey,
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
        key: &RedisKey,
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
        key: &RedisKey,
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
        key: &RedisKey,
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
                key.tenant_aware_key(self),
                serialized.as_slice(),
                Some(Expiration::EX(seconds)),
                None,
                false,
            )
            .await
            .change_context(errors::RedisError::SetExFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_key<V>(&self, key: &RedisKey) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        match self
            .pool
            .get(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetFailed)
        {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }

                #[cfg(feature = "multitenancy_fallback")]
                {
                    self.pool
                        .get(key.tenant_unaware_key(self))
                        .await
                        .change_context(errors::RedisError::GetFailed)
                }
            }
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    async fn get_multiple_keys_with_mget<V>(
        &self,
        keys: &[RedisKey],
    ) -> CustomResult<Vec<Option<V>>, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let tenant_aware_keys: Vec<String> =
            keys.iter().map(|key| key.tenant_aware_key(self)).collect();
        self.pool
            .mget(tenant_aware_keys)
            .await
            .change_context(errors::RedisError::GetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    async fn get_multiple_keys_with_parallel_get<V>(
        &self,
        keys: &[RedisKey],
    ) -> CustomResult<Vec<Option<V>>, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let tenant_aware_keys: Vec<String> =
            keys.iter().map(|key| key.tenant_aware_key(self)).collect();

        let futures = tenant_aware_keys
            .iter()
            .map(|redis_key| self.pool.get::<Option<V>, _>(redis_key));

        let results = futures::future::try_join_all(futures)
            .await
            .change_context(errors::RedisError::GetFailed)
            .attach_printable("Failed to get keys in cluster mode")?;

        Ok(results)
    }

    /// Helper method to encapsulate the logic for choosing between cluster and non-cluster modes
    #[instrument(level = "DEBUG", skip(self))]
    async fn get_keys_by_mode<V>(
        &self,
        keys: &[RedisKey],
    ) -> CustomResult<Vec<Option<V>>, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        if self.config.cluster_enabled {
            // Use individual GET commands for cluster mode to avoid CROSSSLOT errors
            self.get_multiple_keys_with_parallel_get(keys).await
        } else {
            // Use MGET for non-cluster mode for better performance
            self.get_multiple_keys_with_mget(keys).await
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_multiple_keys<V>(
        &self,
        keys: &[RedisKey],
    ) -> CustomResult<Vec<Option<V>>, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        match self.get_keys_by_mode(keys).await {
            Ok(values) => Ok(values),
            Err(_err) => {
                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }

                #[cfg(feature = "multitenancy_fallback")]
                {
                    let tenant_unaware_keys: Vec<RedisKey> = keys
                        .iter()
                        .map(|key| key.tenant_unaware_key(self).into())
                        .collect();

                    self.get_keys_by_mode(&tenant_unaware_keys).await
                }
            }
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn exists<V>(&self, key: &RedisKey) -> CustomResult<bool, errors::RedisError>
    where
        V: Into<MultipleKeys> + Unpin + Send + 'static,
    {
        match self
            .pool
            .exists(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetFailed)
        {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }

                #[cfg(feature = "multitenancy_fallback")]
                {
                    self.pool
                        .exists(key.tenant_unaware_key(self))
                        .await
                        .change_context(errors::RedisError::GetFailed)
                }
            }
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_and_deserialize_key<T>(
        &self,
        key: &RedisKey,
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
    pub async fn get_and_deserialize_multiple_keys<T>(
        &self,
        keys: &[RedisKey],
        type_name: &'static str,
    ) -> CustomResult<Vec<Option<T>>, errors::RedisError>
    where
        T: serde::de::DeserializeOwned,
    {
        let value_bytes_vec = self.get_multiple_keys::<Vec<u8>>(keys).await?;

        let mut results = Vec::with_capacity(value_bytes_vec.len());
        for value_bytes_opt in value_bytes_vec {
            match value_bytes_opt {
                Some(value_bytes) => {
                    if value_bytes.is_empty() {
                        results.push(None);
                    } else {
                        let parsed = value_bytes
                            .parse_struct(type_name)
                            .change_context(errors::RedisError::JsonDeserializationFailed)?;
                        results.push(Some(parsed));
                    }
                }
                None => results.push(None),
            }
        }

        Ok(results)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn delete_key(&self, key: &RedisKey) -> CustomResult<DelReply, errors::RedisError> {
        match self
            .pool
            .del(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::DeleteFailed)
        {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }

                #[cfg(feature = "multitenancy_fallback")]
                {
                    self.pool
                        .del(key.tenant_unaware_key(self))
                        .await
                        .change_context(errors::RedisError::DeleteFailed)
                }
            }
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn delete_multiple_keys(
        &self,
        keys: &[RedisKey],
    ) -> CustomResult<Vec<DelReply>, errors::RedisError> {
        let futures = keys.iter().map(|key| self.delete_key(key));

        let del_result = futures::future::try_join_all(futures)
            .await
            .change_context(errors::RedisError::DeleteFailed)?;

        Ok(del_result)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key_with_expiry<V>(
        &self,
        key: &RedisKey,
        value: V,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .set(
                key.tenant_aware_key(self),
                value,
                Some(Expiration::EX(seconds)),
                None,
                false,
            )
            .await
            .change_context(errors::RedisError::SetExFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key_if_not_exists_with_expiry<V>(
        &self,
        key: &RedisKey,
        value: V,
        seconds: Option<i64>,
    ) -> CustomResult<SetnxReply, errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        self.pool
            .set(
                key.tenant_aware_key(self),
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
        key: &RedisKey,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError> {
        self.pool
            .expire(key.tenant_aware_key(self), seconds)
            .await
            .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expire_at(
        &self,
        key: &RedisKey,
        timestamp: i64,
    ) -> CustomResult<(), errors::RedisError> {
        self.pool
            .expire_at(key.tenant_aware_key(self), timestamp)
            .await
            .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_ttl(&self, key: &RedisKey) -> CustomResult<i64, errors::RedisError> {
        self.pool
            .ttl(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_hash_fields<F, V>(
        &self,
        key: &RedisKey,
        field_value_pairs: Vec<(F, V)>,
        ttl: Option<i64>,
    ) -> CustomResult<(), errors::RedisError>
    where
        F: Into<String> + Debug + Send + Sync,
        V: Into<String> + Debug + Send + Sync,
    {
        let pairs: Vec<(String, String)> = field_value_pairs
            .into_iter()
            .map(|(f, v)| (f.into(), v.into()))
            .collect();

        let map = RedisMap::try_from(pairs)
            .change_context(errors::RedisError::SetHashFailed)
            .attach_printable("Failed to convert field pairs to fred::types::RedisMap")?;

        let output: Result<(), _> = self
            .pool
            .hset(key.tenant_aware_key(self), map)
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
        key: &RedisKey,
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
            .hsetnx(key.tenant_aware_key(self), field, value)
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
        key: &RedisKey,
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
        kv: &[(&RedisKey, V)],
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
    pub async fn increment_fields_in_hash<T>(
        &self,
        key: &RedisKey,
        fields_to_increment: &[(T, i64)],
    ) -> CustomResult<Vec<usize>, errors::RedisError>
    where
        T: Debug + ToString,
    {
        let mut values_after_increment = Vec::with_capacity(fields_to_increment.len());
        for (field, increment) in fields_to_increment {
            values_after_increment.push(
                self.pool
                    .hincrby(key.tenant_aware_key(self), field.to_string(), *increment)
                    .await
                    .change_context(errors::RedisError::IncrementHashFieldFailed)?,
            )
        }

        Ok(values_after_increment)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn hscan(
        &self,
        key: &RedisKey,
        pattern: &str,
        count: Option<u32>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        use futures::StreamExt;

        Ok(self
            .pool
            .next()
            .hscan::<&str, &str>(&key.tenant_aware_key(self), pattern, count)
            .filter_map(|value| async move {
                match value {
                    Ok(mut v) => {
                        let v = v.take_results()?;

                        let v: Vec<String> =
                            v.values().filter_map(|val| val.as_string()).collect();
                        Some(futures::stream::iter(v))
                    }
                    Err(err) => {
                        tracing::error!(redis_err=?err, "Redis error while executing hscan command");
                        None
                    }
                }
            })
            .flatten()
            .collect::<Vec<_>>()
            .await)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn scan(
        &self,
        pattern: &RedisKey,
        count: Option<u32>,
        scan_type: Option<crate::types::RedisScanType>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        use futures::StreamExt;

        let fred_scan_type = scan_type.map(fred::types::ScanType::from);

        Ok(self
            .pool
            .next()
            .scan(pattern.tenant_aware_key(self), count, fred_scan_type)
            .filter_map(|value| async move {
                match value {
                    Ok(mut v) => {
                        let v = v.take_results()?;
                        let v: Vec<String> =
                            v.into_iter().filter_map(|val| val.into_string()).collect();
                        Some(futures::stream::iter(v))
                    }
                    Err(err) => {
                        tracing::error!(redis_err=?err, "Redis error while executing scan command");
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
        key: &RedisKey,
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
        key: &RedisKey,
        field: &str,
    ) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        match self
            .pool
            .hget(key.tenant_aware_key(self), field)
            .await
            .change_context(errors::RedisError::GetHashFieldFailed)
        {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(feature = "multitenancy_fallback")]
                {
                    self.pool
                        .hget(key.tenant_unaware_key(self), field)
                        .await
                        .change_context(errors::RedisError::GetHashFieldFailed)
                }

                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }
            }
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_hash_fields<V>(&self, key: &RedisKey) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        match self
            .pool
            .hgetall(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetHashFieldFailed)
        {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(feature = "multitenancy_fallback")]
                {
                    self.pool
                        .hgetall(key.tenant_unaware_key(self))
                        .await
                        .change_context(errors::RedisError::GetHashFieldFailed)
                }

                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }
            }
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_hash_field_and_deserialize<V>(
        &self,
        key: &RedisKey,
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
        key: &RedisKey,
        members: V,
    ) -> CustomResult<SaddReply, errors::RedisError>
    where
        V: TryInto<MultipleValues> + Debug + Send,
        V::Error: Into<fred::error::RedisError> + Send,
    {
        self.pool
            .sadd(key.tenant_aware_key(self), members)
            .await
            .change_context(errors::RedisError::SetAddMembersFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_append_entry<F, V>(
        &self,
        stream: &RedisKey,
        entry_id: &RedisEntryId,
        fields: Vec<(F, V)>,
    ) -> CustomResult<(), errors::RedisError>
    where
        F: Into<String> + Debug + Send + Sync,
        V: Into<String> + Debug + Send + Sync,
    {
        let pairs: Vec<(String, String)> = fields
            .into_iter()
            .map(|(f, v)| (f.into(), v.into()))
            .collect();

        let fred_fields = MultipleOrderedPairs::try_from(pairs)
            .change_context(errors::RedisError::StreamAppendFailed)
            .attach_printable(
                "Failed to convert field pairs to fred::types::MultipleOrderedPairs",
            )?;

        self.pool
            .xadd(
                stream.tenant_aware_key(self),
                false,
                None,
                entry_id,
                fred_fields,
            )
            .await
            .change_context(errors::RedisError::StreamAppendFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_delete_entries(
        &self,
        stream: &RedisKey,
        ids: Vec<String>,
    ) -> CustomResult<usize, errors::RedisError> {
        let fred_ids: MultipleStrings = ids.into();
        self.pool
            .xdel(stream.tenant_aware_key(self), fred_ids)
            .await
            .change_context(errors::RedisError::StreamDeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_trim_entries(
        &self,
        stream: &RedisKey,
        config: StreamTrimConfig,
    ) -> CustomResult<usize, errors::RedisError> {
        let xcap = fred::types::XCap::try_from(config)
            .change_context(errors::RedisError::StreamTrimFailed)
            .attach_printable("Failed to convert StreamTrimConfig to fred::types::XCap")?;
        self.pool
            .xtrim(stream.tenant_aware_key(self), xcap)
            .await
            .change_context(errors::RedisError::StreamTrimFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_acknowledge_entries(
        &self,
        stream: &RedisKey,
        group: &str,
        ids: Vec<String>,
    ) -> CustomResult<usize, errors::RedisError> {
        let fred_ids: MultipleIDs = ids.into();
        self.pool
            .xack(stream.tenant_aware_key(self), group, fred_ids)
            .await
            .change_context(errors::RedisError::StreamAcknowledgeFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_get_length(
        &self,
        stream: &RedisKey,
    ) -> CustomResult<usize, errors::RedisError> {
        self.pool
            .xlen(stream.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetLengthFailed)
    }

    fn get_keys_with_prefix(&self, streams: &[RedisKey]) -> MultipleKeys {
        let res: Vec<String> = streams.iter().map(|k| k.tenant_aware_key(self)).collect();
        MultipleKeys::from(res)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_entries(
        &self,
        streams: &[RedisKey],
        ids: Vec<String>,
        read_count: Option<u64>,
    ) -> CustomResult<StreamReadResult, errors::RedisError> {
        let strms = self.get_keys_with_prefix(streams);
        let ids: MultipleIDs = ids.into();
        let reply: XReadResponse<String, String, String, String> = self
            .pool
            .xread_map(
                Some(read_count.unwrap_or(self.config.default_stream_read_count)),
                None,
                strms,
                ids,
            )
            .await
            .map_err(|err| match err.kind() {
                RedisErrorKind::NotFound | RedisErrorKind::Parse => {
                    report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
                }
                _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
            })?;

        Ok(reply
            .into_iter()
            .map(|(stream_key, stream_entries)| {
                let parsed_entries: StreamEntries = stream_entries
                    .into_iter()
                    .map(|(entry_id, field_pairs)| {
                        // Convert raw fred field values into the common RedisValue wrapper type.
                        // This preserves all data (strings, nulls, binary, etc.) in a backend-neutral form.
                        let fields_by_redis_value: std::collections::HashMap<
                            String,
                            crate::RedisValue,
                        > = field_pairs
                            .into_iter()
                            .map(|(field_name, field_value)| {
                                (field_name, crate::RedisValue::new(field_value.into()))
                            })
                            .collect();
                        (entry_id, fields_by_redis_value)
                    })
                    .collect();
                (stream_key, parsed_entries)
            })
            .collect())
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_with_options(
        &self,
        streams: &[RedisKey],
        ids: Vec<String>,
        count: Option<u64>,
        block: Option<u64>,
        group: Option<(&str, &str)>,
    ) -> CustomResult<StreamReadResult, errors::RedisError> {
        let strms = self.get_keys_with_prefix(streams);
        let ids: MultipleIDs = ids.into();

        let reply: XReadResponse<String, String, String, Option<String>> = match group {
            Some((group_name, consumer_name)) => {
                self.pool
                    .xreadgroup_map(group_name, consumer_name, count, block, false, strms, ids)
                    .await
            }
            None => self.pool.xread_map(count, block, strms, ids).await,
        }
        .map_err(|err| match err.kind() {
            RedisErrorKind::NotFound | RedisErrorKind::Parse => {
                report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
            }
            _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
        })?;

        Ok(reply
            .into_iter()
            .map(|(stream_key, stream_entries)| {
                let parsed_entries: StreamEntries = stream_entries
                    .into_iter()
                    .map(|(entry_id, optional_field_pairs)| {
                        // Wrap fred's field values (Option<String>) into RedisValue.
                        // If the field has no value, we store Null to preserve the entry's presence.
                        let fields_by_redis_value: std::collections::HashMap<
                            String,
                            crate::RedisValue,
                        > = optional_field_pairs
                            .into_iter()
                            .map(|(field_name, maybe_field_value)| {
                                let redis_value_inner = match maybe_field_value {
                                    Some(string_value) => RedisValue::String(string_value.into()),
                                    None => RedisValue::Null,
                                };
                                (field_name, crate::RedisValue::new(redis_value_inner))
                            })
                            .collect();
                        (entry_id, fields_by_redis_value)
                    })
                    .collect();
                (stream_key, parsed_entries)
            })
            .collect())
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn append_elements_to_list<V>(
        &self,
        key: &RedisKey,
        elements: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<MultipleValues> + Debug + Send,
        V::Error: Into<fred::error::RedisError> + Send,
    {
        self.pool
            .rpush(key.tenant_aware_key(self), elements)
            .await
            .change_context(errors::RedisError::AppendElementsToListFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_list_elements(
        &self,
        key: &RedisKey,
        start: i64,
        stop: i64,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        self.pool
            .lrange(key.tenant_aware_key(self), start, stop)
            .await
            .change_context(errors::RedisError::GetListElementsFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_list_length(&self, key: &RedisKey) -> CustomResult<usize, errors::RedisError> {
        self.pool
            .llen(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetListLengthFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn lpop_list_elements(
        &self,
        key: &RedisKey,
        count: Option<usize>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        self.pool
            .lpop(key.tenant_aware_key(self), count)
            .await
            .change_context(errors::RedisError::PopListElementsFailed)
    }

    //                                              Consumer Group API

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_create(
        &self,
        stream: &RedisKey,
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
            .xgroup_create(stream.tenant_aware_key(self), group, id, true)
            .await
            .change_context(errors::RedisError::ConsumerGroupCreateFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_destroy(
        &self,
        stream: &RedisKey,
        group: &str,
    ) -> CustomResult<crate::types::ConsumerGroupDestroyReply, errors::RedisError> {
        let reply: crate::types::ConsumerGroupDestroyReply = self
            .pool
            .xgroup_destroy(stream.tenant_aware_key(self), group)
            .await
            .change_context(errors::RedisError::ConsumerGroupDestroyFailed)?;
        Ok(reply)
    }

    // the number of pending messages that the consumer had before it was deleted
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_delete_consumer(
        &self,
        stream: &RedisKey,
        group: &str,
        consumer: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        self.pool
            .xgroup_delconsumer(stream.tenant_aware_key(self), group, consumer)
            .await
            .change_context(errors::RedisError::ConsumerGroupRemoveConsumerFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_set_last_id(
        &self,
        stream: &RedisKey,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<String, errors::RedisError> {
        let id_str = id.to_stream_id();
        self.pool
            .xgroup_setid::<(), _, _, _>(stream.tenant_aware_key(self), group, &id_str)
            .await
            .change_context(errors::RedisError::ConsumerGroupSetIdFailed)?;
        Ok(id_str)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_set_message_owner<R>(
        &self,
        stream: &RedisKey,
        group: &str,
        consumer: &str,
        min_idle_time: u64,
        ids: Vec<String>,
    ) -> CustomResult<R, errors::RedisError>
    where
        R: FromRedis + Unpin + Send + 'static,
    {
        let fred_ids: MultipleIDs = ids.into();
        self.pool
            .xclaim(
                stream.tenant_aware_key(self),
                group,
                consumer,
                min_idle_time,
                fred_ids,
                None,
                None,
                None,
                false,
                false,
            )
            .await
            .change_context(errors::RedisError::ConsumerGroupClaimFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn evaluate_redis_script<V, T>(
        &self,
        lua_script: &'static str,
        key: Vec<String>,
        values: V,
    ) -> CustomResult<T, errors::RedisError>
    where
        V: TryInto<MultipleValues> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
        T: serde::de::DeserializeOwned + FromRedis,
    {
        let val: T = self
            .pool
            .eval(lua_script, key, values)
            .await
            .change_context(errors::RedisError::IncrementHashFieldFailed)?;
        Ok(val)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_multiple_keys_if_not_exists_and_get_values<V>(
        &self,
        keys: &[(RedisKey, V)],
        ttl: Option<i64>,
    ) -> CustomResult<Vec<SetGetReply<V>>, errors::RedisError>
    where
        V: TryInto<RedisValue>
            + Debug
            + FromRedis
            + ToOwned<Owned = V>
            + Send
            + Sync
            + serde::de::DeserializeOwned,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let futures = keys.iter().map(|(key, value)| {
            self.set_key_if_not_exists_and_get_value(key, (*value).to_owned(), ttl)
        });

        let del_result = futures::future::try_join_all(futures)
            .await
            .change_context(errors::RedisError::SetFailed)?;

        Ok(del_result)
    }

    /// Sets a value in Redis if not already present, and returns the value (either existing or newly set).
    /// This operation is atomic using Redis transactions.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key_if_not_exists_and_get_value<V>(
        &self,
        key: &RedisKey,
        value: V,
        ttl: Option<i64>,
    ) -> CustomResult<SetGetReply<V>, errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + FromRedis + Send + Sync + serde::de::DeserializeOwned,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let redis_key = key.tenant_aware_key(self);
        let ttl_seconds = ttl.unwrap_or(self.config.default_ttl.into());

        // Get a client from the pool and start transaction
        let trx = self.get_transaction();

        // Try to set if not exists with expiry - queue the command
        trx.set::<(), _, _>(
            &redis_key,
            value,
            Some(Expiration::EX(ttl_seconds)),
            Some(SetOptions::NX),
            false,
        )
        .await
        .change_context(errors::RedisError::SetFailed)
        .attach_printable("Failed to queue set command")?;

        // Always get the value after the SET attempt - queue the command
        trx.get::<V, _>(&redis_key)
            .await
            .change_context(errors::RedisError::GetFailed)
            .attach_printable("Failed to queue get command")?;

        // Execute transaction
        let mut results: Vec<RedisValue> = trx
            .exec(true)
            .await
            .change_context(errors::RedisError::SetFailed)
            .attach_printable("Failed to execute the redis transaction")?;

        let msg = "Got unexpected number of results from transaction";
        let get_result = results
            .pop()
            .ok_or(errors::RedisError::SetFailed)
            .attach_printable(msg)?;
        let set_result = results
            .pop()
            .ok_or(errors::RedisError::SetFailed)
            .attach_printable(msg)?;
        // Parse the GET result to get the actual value
        let actual_value: V = FromRedis::from_value(get_result)
            .change_context(errors::RedisError::SetFailed)
            .attach_printable("Failed to convert from redis value")?;

        // Check if SET NX succeeded or failed
        match set_result {
            // SET NX returns "OK" if key was set
            RedisValue::String(_) => Ok(SetGetReply::ValueSet(actual_value)),
            // SET NX returns null if key already exists
            RedisValue::Null => Ok(SetGetReply::ValueExists(actual_value)),
            _ => Err(report!(errors::RedisError::SetFailed))
                .attach_printable("Unexpected result from SET NX operation"),
        }
    }
}
