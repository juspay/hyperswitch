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
        MultipleValues, RedisMap, RedisValue, ScanType, Scanner, SetOptions, XCap, XReadResponse,
    },
};
use futures::StreamExt;
use router_env::logger;
use tracing::instrument;

use crate::{
    errors,
    types::{
        DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisKey, SaddReply, SetGetReply,
        SetnxReply,
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            "Redis SET command Started"
        );

        let result = self
            .pool
            .set(
                redis_key.clone(),
                value,
                Some(Expiration::EX(self.config.default_ttl.into())),
                None,
                false,
            )
            .await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SET command completed"
        );

        result.change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key_without_modifying_ttl<V>(
        &self,
        key: &RedisKey,
        value: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            "Redis SET (KEEPTTL) command Started"
        );

        let result = self
            .pool
            .set(
                redis_key.clone(),
                value,
                Some(Expiration::KEEPTTL),
                None,
                false,
            )
            .await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SET (KEEPTTL) command completed"
        );

        result.change_context(errors::RedisError::SetFailed)
    }
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_multiple_keys_if_not_exist<V>(
        &self,
        value: V,
    ) -> CustomResult<MsetnxReply, errors::RedisError>
    where
        V: TryInto<RedisMap> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let start = std::time::Instant::now();
        let keys_display = format!("{:?}", &value);
        logger::debug!(
            keys = %keys_display,
            "Redis MSETNX command Started"
        );

        let result = self.pool.msetnx(value).await;

        logger::debug!(
            keys = %keys_display,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis MSETNX command completed"
        );

        result.change_context(errors::RedisError::SetFailed)
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        logger::debug!(
            key = %redis_key,
            "Redis SETNX (serialized) command Started"
        );

        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        let result = self
            .set_key_if_not_exists_with_expiry(key, serialized.as_slice(), ttl)
            .await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SETNX (serialized) command completed"
        );

        result
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        logger::debug!(
            key = %redis_key,
            "Redis SET (serialized) command Started"
        );

        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        let result = self.set_key(key, serialized.as_slice()).await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SET (serialized) command completed"
        );

        result
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        logger::debug!(
            key = %redis_key,
            "Redis SET (serialized, KEEPTTL) command Started"
        );

        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        let result = self
            .set_key_without_modifying_ttl(key, serialized.as_slice())
            .await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SET (serialized, KEEPTTL) command completed"
        );

        result
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        logger::debug!(
            key = %redis_key,
            "Redis SETEX (serialized) command Started"
        );

        let result = self
            .pool
            .set(
                redis_key.clone(),
                serialized.as_slice(),
                Some(Expiration::EX(seconds)),
                None,
                false,
            )
            .await;

        logger::debug!(
            key = %redis_key,
            ttl_seconds = %seconds,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SETEX (serialized) command completed"
        );

        result.change_context(errors::RedisError::SetExFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_key<V>(&self, key: &RedisKey) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            "Redis GET command Started"
        );

        let result = match self.pool.get(redis_key.clone()).await {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }

                #[cfg(feature = "multitenancy_fallback")]
                {
                    let fallback_key = key.tenant_unaware_key(self);
                    self.pool.get(fallback_key).await
                }
            }
        };

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis GET command completed"
        );

        result.change_context(errors::RedisError::GetFailed)
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
        logger::debug!(
            "Redis MGET command Started"
        );

        let start = std::time::Instant::now();
        let tenant_aware_keys: Vec<String> =
            keys.iter().map(|key| key.tenant_aware_key(self)).collect();

        let result = self.pool.mget(tenant_aware_keys.clone()).await;

        logger::debug!(
            keys = ?tenant_aware_keys,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis MGET command completed"
        );

        result.change_context(errors::RedisError::GetFailed)
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
        logger::debug!(
            "Redis GET (parallel) command Started"
        );

        let start = std::time::Instant::now();
        let tenant_aware_keys: Vec<String> =
            keys.iter().map(|key| key.tenant_aware_key(self)).collect();

        let futures = tenant_aware_keys
            .iter()
            .map(|redis_key| self.pool.get::<Option<V>, _>(redis_key));

        let results = futures::future::try_join_all(futures)
            .await
            .change_context(errors::RedisError::GetFailed)
            .attach_printable("Failed to get keys in cluster mode")?;

        logger::debug!(
            keys = ?tenant_aware_keys,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis parallel GET commands completed"
        );

        Ok(results)
    }

    #[instrument(level = "DEBUG", skip(self))]
    async fn get_keys_by_mode<V>(
        &self,
        keys: &[RedisKey],
    ) -> CustomResult<Vec<Option<V>>, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        let start = std::time::Instant::now();
        let keys_display: Vec<String> = keys.iter().map(|k| k.tenant_aware_key(self)).collect();
        logger::debug!(
            "Redis get_keys_by_mode completed Started"
        );

        let result = if self.config.cluster_enabled {
            // Use individual GET commands for cluster mode to avoid CROSSSLOT errors
            self.get_multiple_keys_with_parallel_get(keys).await
        } else {
            // Use MGET for non-cluster mode for better performance
            self.get_multiple_keys_with_mget(keys).await
        };

        logger::debug!(
            keys = ?keys_display,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis get_keys_by_mode completed"
        );

        result
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

        let start = std::time::Instant::now();
        let keys_display: Vec<String> = keys.iter().map(|k| k.tenant_aware_key(self)).collect();
        logger::debug!(
            keys = ?keys_display.clone(),
            "Redis GET multiple keys Started"
        );

        let result = match self.get_keys_by_mode(keys).await {
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
        };

        logger::debug!(
            keys = ?keys_display,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis GET multiple keys completed"
        );

        result
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        logger::debug!(
            key = %redis_key,
            type_name = %type_name,
            "Redis GET and deserialize Started"
        );

        let value_bytes = self.get_key::<Vec<u8>>(key).await?;

        fp_utils::when(value_bytes.is_empty(), || Err(errors::RedisError::NotFound))?;

        let result = value_bytes
            .parse_struct(type_name)
            .change_context(errors::RedisError::JsonDeserializationFailed);

        logger::debug!(
            key = %redis_key,
            type_name = %type_name,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis GET and deserialize completed"
        );

        result
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
        let start = std::time::Instant::now();
        let keys_display: Vec<String> = keys.iter().map(|k| k.tenant_aware_key(self)).collect();

        logger::debug!(
            keys = ?keys_display,
            type_name = %type_name,
            "Redis GET multiple and deserialize Started"
        );

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

        logger::debug!(
            keys = ?keys_display,
            type_name = %type_name,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis GET multiple and deserialize completed"
        );

        Ok(results)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn delete_key(&self, key: &RedisKey) -> CustomResult<DelReply, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            "Redis DEL command Started"
        );

        let result = match self.pool.del(redis_key.clone()).await {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }

                #[cfg(feature = "multitenancy_fallback")]
                {
                    self.pool.del(key.tenant_unaware_key(self)).await
                }
            }
        };

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis DEL command completed"
        );

        result.change_context(errors::RedisError::DeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn delete_multiple_keys(
        &self,
        keys: &[RedisKey],
    ) -> CustomResult<Vec<DelReply>, errors::RedisError> {
        let start = std::time::Instant::now();
        let keys_display: Vec<String> = keys.iter().map(|k| k.tenant_aware_key(self)).collect();
        logger::debug!(
            keys = ?keys_display,
            "Redis DEL multiple keys Started"
        );

        let futures = keys.iter().map(|key| self.delete_key(key));

        let del_result = futures::future::try_join_all(futures)
            .await
            .change_context(errors::RedisError::DeleteFailed)?;

        logger::debug!(
            keys = ?keys_display,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis DEL multiple keys completed"
        );

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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        logger::debug!(
            key = %redis_key,
            "Redis SETEX command Started"
        );


        let result = self
            .pool
            .set(
                redis_key.clone(),
                value,
                Some(Expiration::EX(seconds)),
                None,
                false,
            )
            .await;

        logger::debug!(
            key = %redis_key,
            ttl_seconds = %seconds,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SETEX command completed"
        );

        result.change_context(errors::RedisError::SetExFailed)
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        let ttl = seconds.unwrap_or(self.config.default_ttl.into());

        logger::debug!(
            key = %redis_key,
            "Redis SETNX command Started"
        );

        let result = self
            .pool
            .set(
                redis_key.clone(),
                value,
                Some(Expiration::EX(ttl)),
                Some(SetOptions::NX),
                false,
            )
            .await;

        logger::debug!(
            key = %redis_key,
            ttl_seconds = %ttl,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SETNX command completed"
        );

        result.change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expiry(
        &self,
        key: &RedisKey,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        logger::debug!(
            key = %redis_key,
            "Redis EXPIRE command Started"
        );

        let result = self.pool.expire(redis_key.clone(), seconds).await;

        logger::debug!(
            key = %redis_key,
            ttl_seconds = %seconds,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis EXPIRE command completed"
        );

        result.change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expire_at(
        &self,
        key: &RedisKey,
        timestamp: i64,
    ) -> CustomResult<(), errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            timestamp = %timestamp,
            "Redis EXPIREAT command Started"
        );

        let result = self.pool.expire_at(redis_key.clone(), timestamp).await;

        logger::debug!(
            key = %redis_key,
            timestamp = %timestamp,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis EXPIREAT command completed"
        );

        result.change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_ttl(&self, key: &RedisKey) -> CustomResult<i64, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            "Redis TTL command Started"
        );

        let result = self.pool.ttl(redis_key.clone()).await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis TTL command completed"
        );

        result.change_context(errors::RedisError::GetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_hash_fields<V>(
        &self,
        key: &RedisKey,
        values: V,
        ttl: Option<i64>,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisMap> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            "Redis HSET command Started"
        );

        let output: Result<(), _> = self
            .pool
            .hset(redis_key.clone(), values)
            .await
            .change_context(errors::RedisError::SetHashFailed);
        // setting expiry for the key
        let result = output
            .async_and_then(|_| {
                self.set_expiry(key, ttl.unwrap_or(self.config.default_hash_ttl.into()))
            })
            .await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HSET command completed"
        );

        result
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            field = %field,
            "Redis HSETNX command Started"
        );

        let output: Result<HsetnxReply, _> = self
            .pool
            .hsetnx(redis_key.clone(), field, value)
            .await
            .change_context(errors::RedisError::SetHashFieldFailed);

        let result = output
            .async_and_then(|inner| async {
                self.set_expiry(key, ttl.unwrap_or(self.config.default_hash_ttl).into())
                    .await?;
                Ok(inner)
            })
            .await;

        logger::debug!(
            key = %redis_key,
            field = %field,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HSETNX command completed"
        );

        result
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        
        logger::debug!(
            key = %redis_key,
            field = %field,
            "Redis HSETNX (serialized) command Started"
        );
        let serialized = value
            .encode_to_vec()
            .change_context(errors::RedisError::JsonSerializationFailed)?;

        let result = self
            .set_hash_field_if_not_exist(key, field, serialized.as_slice(), ttl)
            .await;

        logger::debug!(
            key = %redis_key,
            field = %field,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HSETNX (serialized) command completed"
        );

        result
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
        let start = std::time::Instant::now();
        let keys_display: Vec<String> = kv.iter().map(|(k, _)| k.tenant_aware_key(self)).collect();

        let mut hsetnx: Vec<HsetnxReply> = Vec::with_capacity(kv.len());
        for (key, val) in kv {
            hsetnx.push(
                self.serialize_and_set_hash_field_if_not_exist(key, field, val, ttl)
                    .await?,
            );
        }

        logger::debug!(
            keys = ?keys_display,
            field = %field,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HSETNX multiple keys completed"
        );

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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            fields = ?fields_to_increment,
            "Redis HINCRBY commands Started"
        );

        let mut values_after_increment = Vec::with_capacity(fields_to_increment.len());
        for (field, increment) in fields_to_increment.iter() {
            values_after_increment.push(
                self.pool
                    .hincrby(redis_key.clone(), field.to_string(), *increment)
                    .await
                    .change_context(errors::RedisError::IncrementHashFieldFailed)?,
            )
        }

        logger::debug!(
            key = %redis_key,
            fields = ?fields_to_increment,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HINCRBY commands completed"
        );

        Ok(values_after_increment)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn hscan(
        &self,
        key: &RedisKey,
        pattern: &str,
        count: Option<u32>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            pattern = %pattern,
            "Redis HSCAN command Started"
        );

        let result = self
            .pool
            .next()
            .hscan::<&str, &str>(&redis_key, pattern, count)
            .filter_map(|value| async move {
                match value {
                    Ok(mut v) => {
                        let v = v.take_results()?;

                        let v: Vec<String> =
                            v.iter().filter_map(|(_, val)| val.as_string()).collect();
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
            .await;

        logger::debug!(
            key = %redis_key,
            pattern = %pattern,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HSCAN command completed"
        );

        Ok(result)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn scan(
        &self,
        pattern: &RedisKey,
        count: Option<u32>,
        scan_type: Option<ScanType>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_pattern = pattern.tenant_aware_key(self);
        logger::debug!(
            pattern = %redis_pattern,
            "Redis SCAN command Started"
        );

        let result = self
            .pool
            .next()
            .scan(redis_pattern.clone(), count, scan_type)
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
            .await;

        logger::debug!(
            pattern = %redis_pattern,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SCAN command completed"
        );

        Ok(result)
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        let redis_results = self.hscan(key, pattern, count).await?;
        let result: Vec<T> = redis_results
            .iter()
            .filter_map(|v| {
                let r: T = v.parse_struct(std::any::type_name::<T>()).ok()?;
                Some(r)
            })
            .collect();

        logger::debug!(
            key = %redis_key,
            pattern = %pattern,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HSCAN and deserialize completed"
        );

        Ok(result)
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            field = %field,
            "Redis HGET command Started"
        );

        let result = match self.pool.hget(redis_key.clone(), field).await {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(feature = "multitenancy_fallback")]
                {
                    self.pool.hget(key.tenant_unaware_key(self), field).await
                }

                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }
            }
        };

        logger::debug!(
            key = %redis_key,
            field = %field,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HGET command completed"
        );

        result.change_context(errors::RedisError::GetHashFieldFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_hash_fields<V>(&self, key: &RedisKey) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            "Redis HGETALL command Started"
        );

        let result = match self.pool.hgetall(redis_key.clone()).await {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(feature = "multitenancy_fallback")]
                {
                    self.pool.hgetall(key.tenant_unaware_key(self)).await
                }

                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }
            }
        };

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HGETALL command completed"
        );

        result.change_context(errors::RedisError::GetHashFieldFailed)
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            field = %field,
            type_name = %type_name,
            "Redis HGET and deserialize Started"
        );

        let value_bytes = self.get_hash_field::<Vec<u8>>(key, field).await?;

        if value_bytes.is_empty() {
            return Err(errors::RedisError::NotFound.into());
        }

        let result = value_bytes
            .parse_struct(type_name)
            .change_context(errors::RedisError::JsonDeserializationFailed);

        logger::debug!(
            key = %redis_key,
            field = %field,
            type_name = %type_name,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis HGET and deserialize completed"
        );

        result
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            "Redis SADD command Started"
        );

        let result = self.pool.sadd(redis_key.clone(), members).await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SADD command completed"
        );

        result.change_context(errors::RedisError::SetAddMembersFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_append_entry<F>(
        &self,
        stream: &RedisKey,
        entry_id: &RedisEntryId,
        fields: F,
    ) -> CustomResult<(), errors::RedisError>
    where
        F: TryInto<MultipleOrderedPairs> + Debug + Send + Sync,
        F::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);
        logger::debug!(
            key = %redis_key,
            entry_id = ?entry_id,
            "Redis XADD command Started"
        );

        let result = self
            .pool
            .xadd(redis_key.clone(), false, None, entry_id, fields)
            .await;

        logger::debug!(
            key = %redis_key,
            entry_id = ?entry_id,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XADD command completed"
        );

        result.change_context(errors::RedisError::StreamAppendFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_delete_entries<Ids>(
        &self,
        stream: &RedisKey,
        ids: Ids,
    ) -> CustomResult<usize, errors::RedisError>
    where
        Ids: Into<MultipleStrings> + Debug + Send + Sync,
    {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        let result = self.pool.xdel(redis_key.clone(), ids).await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XDEL command completed"
        );

        result.change_context(errors::RedisError::StreamDeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_trim_entries<C>(
        &self,
        stream: &RedisKey,
        xcap: C,
    ) -> CustomResult<usize, errors::RedisError>
    where
        C: TryInto<XCap> + Debug + Send + Sync,
        C::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        let result = self.pool.xtrim(redis_key.clone(), xcap).await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XTRIM command completed"
        );

        result.change_context(errors::RedisError::StreamTrimFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_acknowledge_entries<Ids>(
        &self,
        stream: &RedisKey,
        group: &str,
        ids: Ids,
    ) -> CustomResult<usize, errors::RedisError>
    where
        Ids: Into<MultipleIDs> + Debug + Send + Sync,
    {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        let result = self.pool.xack(redis_key.clone(), group, ids).await;

        logger::debug!(
            key = %redis_key,
            group = %group,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XACK command completed"
        );

        result.change_context(errors::RedisError::StreamAcknowledgeFailed)
    }

    /// Gets the length of a Redis stream.
    /// Logs the stream key and execution time.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_get_length(
        &self,
        stream: &RedisKey,
    ) -> CustomResult<usize, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        let result = self.pool.xlen(redis_key.clone()).await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XLEN command completed"
        );

        result.change_context(errors::RedisError::GetLengthFailed)
    }

    pub fn get_keys_with_prefix<K>(&self, keys: K) -> MultipleKeys
    where
        K: Into<MultipleKeys> + Debug + Send + Sync,
    {
        let multiple_keys: MultipleKeys = keys.into();
        let res = multiple_keys
            .inner()
            .iter()
            .filter_map(|key| key.as_str().map(RedisKey::from))
            .map(|k: RedisKey| k.tenant_aware_key(self))
            .collect::<Vec<_>>();

        MultipleKeys::from(res)
    }

    /// Reads entries from Redis streams.
    /// Logs the stream keys and execution time.
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
        let start = std::time::Instant::now();
        let strms = self.get_keys_with_prefix(streams);
        let streams_display = format!("{:?}", strms);

        let result = self
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
            });

        logger::debug!(
            streams = %streams_display,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XREAD command completed"
        );

        result
    }

    /// Reads entries from Redis streams with optional blocking and consumer group support.
    /// Logs the stream keys and execution time.
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
        let start = std::time::Instant::now();
        let strms = self.get_keys_with_prefix(streams);
        let streams_display = format!("{:?}", strms);

        let result = match group {
            Some((group_name, consumer_name)) => {
                self.pool
                    .xreadgroup_map(
                        group_name,
                        consumer_name,
                        count,
                        block,
                        false,
                        strms,
                        ids,
                    )
                    .await
            }
            None => {
                self.pool.xread_map(count, block, strms, ids).await
            }
        }
        .map_err(|err| match err.kind() {
            RedisErrorKind::NotFound | RedisErrorKind::Parse => {
                report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
            }
            _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
        });

        logger::debug!(
            streams = %streams_display,
            group = ?group,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XREAD/XREADGROUP command completed"
        );

        result
    }

    /// Appends elements to a Redis list.
    /// Logs the key and execution time.
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
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        let result = self.pool.rpush(redis_key.clone(), elements).await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis RPUSH command completed"
        );

        result.change_context(errors::RedisError::AppendElementsToListFailed)
    }

    /// Gets elements from a Redis list by range.
    /// Logs the key and execution time.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_list_elements(
        &self,
        key: &RedisKey,
        start_idx: i64,
        stop: i64,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        let result = self.pool.lrange(redis_key.clone(), start_idx, stop).await;

        logger::debug!(
            key = %redis_key,
            range = %format!("{}..{}", start_idx, stop),
            elapsed_us = %start.elapsed().as_micros(),
            "Redis LRANGE command completed"
        );

        result.change_context(errors::RedisError::GetListElementsFailed)
    }

    /// Gets the length of a Redis list.
    /// Logs the key and execution time.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_list_length(&self, key: &RedisKey) -> CustomResult<usize, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        let result = self.pool.llen(redis_key.clone()).await;

        logger::debug!(
            key = %redis_key,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis LLEN command completed"
        );

        result.change_context(errors::RedisError::GetListLengthFailed)
    }

    /// Pops elements from the left of a Redis list.
    /// Logs the key and execution time.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn lpop_list_elements(
        &self,
        key: &RedisKey,
        count: Option<usize>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = key.tenant_aware_key(self);

        let result = self.pool.lpop(redis_key.clone(), count).await;

        logger::debug!(
            key = %redis_key,
            count = ?count,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis LPOP command completed"
        );

        result.change_context(errors::RedisError::PopListElementsFailed)
    }

    //                                              Consumer Group API

    /// Creates a consumer group for a Redis stream.
    /// Logs the stream key and execution time.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_create(
        &self,
        stream: &RedisKey,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<(), errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        if matches!(
            id,
            RedisEntryId::AutoGeneratedID | RedisEntryId::UndeliveredEntryID
        ) {
            // FIXME: Replace with utils::when
            Err(errors::RedisError::InvalidRedisEntryId)?;
        }

        let result = self
            .pool
            .xgroup_create(redis_key.clone(), group, id, true)
            .await;

        logger::debug!(
            key = %redis_key,
            group = %group,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XGROUP CREATE command completed"
        );

        result.change_context(errors::RedisError::ConsumerGroupCreateFailed)
    }

    /// Destroys a consumer group for a Redis stream.
    /// Logs the stream key and execution time.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_destroy(
        &self,
        stream: &RedisKey,
        group: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        let result = self.pool.xgroup_destroy(redis_key.clone(), group).await;

        logger::debug!(
            key = %redis_key,
            group = %group,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XGROUP DESTROY command completed"
        );

        result.change_context(errors::RedisError::ConsumerGroupDestroyFailed)
    }

    /// Deletes a consumer from a consumer group.
    /// Logs the stream key and execution time.
    // the number of pending messages that the consumer had before it was deleted
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_delete_consumer(
        &self,
        stream: &RedisKey,
        group: &str,
        consumer: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        let result = self
            .pool
            .xgroup_delconsumer(redis_key.clone(), group, consumer)
            .await;

        logger::debug!(
            key = %redis_key,
            group = %group,
            consumer = %consumer,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XGROUP DELCONSUMER command completed"
        );

        result.change_context(errors::RedisError::ConsumerGroupRemoveConsumerFailed)
    }

    /// Sets the last delivered ID for a consumer group.
    /// Logs the stream key and execution time.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_set_last_id(
        &self,
        stream: &RedisKey,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<String, errors::RedisError> {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        let result = self.pool.xgroup_setid(redis_key.clone(), group, id).await;

        logger::debug!(
            key = %redis_key,
            group = %group,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XGROUP SETID command completed"
        );

        result.change_context(errors::RedisError::ConsumerGroupSetIdFailed)
    }

    /// Changes ownership of pending messages to a different consumer (XCLAIM).
    /// Logs the stream key and execution time.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_set_message_owner<Ids, R>(
        &self,
        stream: &RedisKey,
        group: &str,
        consumer: &str,
        min_idle_time: u64,
        ids: Ids,
    ) -> CustomResult<R, errors::RedisError>
    where
        Ids: Into<MultipleIDs> + Debug + Send + Sync,
        R: FromRedis + Unpin + Send + 'static,
    {
        let start = std::time::Instant::now();
        let redis_key = stream.tenant_aware_key(self);

        let result = self
            .pool
            .xclaim(
                redis_key.clone(),
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
            .await;

        logger::debug!(
            key = %redis_key,
            group = %group,
            consumer = %consumer,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis XCLAIM command completed"
        );

        result.change_context(errors::RedisError::ConsumerGroupClaimFailed)
    }

    /// Evaluates a Lua script on Redis.
    /// Logs the keys and execution time.
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
        let start = std::time::Instant::now();
        let keys_display = format!("{:?}", &key);

        let result: T = self
            .pool
            .eval(lua_script, key, values)
            .await
            .change_context(errors::RedisError::IncrementHashFieldFailed)?;

        logger::debug!(
            keys = %keys_display,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis EVAL command completed"
        );

        Ok(result)
    }

    /// Sets multiple keys if they don't exist and returns the values.
    /// Logs the keys and execution time.
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
        let start = std::time::Instant::now();
        let keys_display: Vec<String> = keys.iter().map(|(k, _)| k.tenant_aware_key(self)).collect();

        let futures = keys.iter().map(|(key, value)| {
            self.set_key_if_not_exists_and_get_value(key, (*value).to_owned(), ttl)
        });

        let del_result = futures::future::try_join_all(futures)
            .await
            .change_context(errors::RedisError::SetFailed)?;

        logger::debug!(
            keys = ?keys_display,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SETNX multiple keys completed"
        );

        Ok(del_result)
    }

    /// Sets a value in Redis if not already present, and returns the value (either existing or newly set).
    /// This operation is atomic using Redis transactions.
    /// Logs the key and execution time.
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
        let start = std::time::Instant::now();
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

        logger::debug!(
            key = %redis_key,
            ttl_seconds = %ttl_seconds,
            elapsed_us = %start.elapsed().as_micros(),
            "Redis SETNX+GET transaction completed"
        );

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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
                    .consumer_group_create(&"TEST1".into(), "GTEST", &RedisEntryId::AutoGeneratedID)
                    .await;

                let result2 = redis_conn
                    .consumer_group_create(
                        &"TEST3".into(),
                        "GTEST",
                        &RedisEntryId::UndeliveredEntryID,
                    )
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
                let _ = pool.set_key(&"key".into(), "value".to_string()).await;

                // Act
                let result = pool.delete_key(&"key".into()).await;

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
                let result = pool.delete_key(&"key not exists".into()).await;

                // Assert Setup
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");
        assert!(is_success);
    }

    #[tokio::test]
    async fn test_setting_keys_using_scripts() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let lua_script = r#"
                for i = 1, #KEYS do
                    redis.call("INCRBY", KEYS[i], ARGV[i])
                end
                return
                "#;
                let mut keys_and_values = HashMap::new();
                for i in 0..10 {
                    keys_and_values.insert(format!("key{i}"), i);
                }

                let key = keys_and_values.keys().cloned().collect::<Vec<_>>();
                let values = keys_and_values
                    .values()
                    .map(|val| val.to_string())
                    .collect::<Vec<String>>();

                // Act
                let result = pool
                    .evaluate_redis_script::<_, ()>(lua_script, key, values)
                    .await;

                // Assert Setup
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }
    #[tokio::test]
    async fn test_getting_keys_using_scripts() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");

                // First set some keys
                for i in 0..3 {
                    let key = format!("script_test_key{i}").into();
                    let _ = pool.set_key(&key, format!("value{i}")).await;
                }

                let lua_script = r#"
                local results = {}
                for i = 1, #KEYS do
                    results[i] = redis.call("GET", KEYS[i])
                end
                return results
                "#;

                let keys = vec![
                    "script_test_key0".to_string(),
                    "script_test_key1".to_string(),
                    "script_test_key2".to_string(),
                ];

                // Act
                let result = pool
                    .evaluate_redis_script::<_, Vec<String>>(lua_script, keys, vec![""])
                    .await;

                // Assert Setup
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_key_if_not_exists_and_get_value_new_key() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key = "test_new_key_string".into();
                let value = "test_value".to_string();

                // Act
                let result = pool
                    .set_key_if_not_exists_and_get_value(&key, value.clone(), Some(30))
                    .await;

                // Assert
                match result {
                    Ok(crate::types::SetGetReply::ValueSet(returned_value)) => {
                        returned_value == value
                    }
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_key_if_not_exists_and_get_value_existing_key() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key = "test_existing_key_string".into();
                let initial_value = "initial_value".to_string();
                let new_value = "new_value".to_string();

                // First, set an initial value using regular set_key
                let _ = pool.set_key(&key, initial_value.clone()).await;

                // Act - try to set a new value (should fail and return existing value)
                let result = pool
                    .set_key_if_not_exists_and_get_value(&key, new_value, Some(30))
                    .await;

                // Assert
                match result {
                    Ok(crate::types::SetGetReply::ValueExists(returned_value)) => {
                        returned_value == initial_value
                    }
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_key_if_not_exists_and_get_value_with_default_ttl() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key = "test_default_ttl_key_string".into();
                let value = "test_value".to_string();

                // Act - use None for TTL to test default behavior
                let result = pool
                    .set_key_if_not_exists_and_get_value(&key, value.clone(), None)
                    .await;

                // Assert
                match result {
                    Ok(crate::types::SetGetReply::ValueSet(returned_value)) => {
                        returned_value == value
                    }
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_key_if_not_exists_and_get_value_concurrent_access() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key_name = "test_concurrent_key_string";
                let value1 = "value1".to_string();
                let value2 = "value2".to_string();

                // Act - simulate concurrent access
                let pool1 = pool.clone("");
                let pool2 = pool.clone("");
                let key1 = key_name.into();
                let key2 = key_name.into();

                let (result1, result2) = tokio::join!(
                    pool1.set_key_if_not_exists_and_get_value(&key1, value1, Some(30)),
                    pool2.set_key_if_not_exists_and_get_value(&key2, value2, Some(30))
                );

                // Assert - one should succeed with ValueSet, one should fail with ValueExists
                let result1_is_set = matches!(result1, Ok(crate::types::SetGetReply::ValueSet(_)));
                let result2_is_set = matches!(result2, Ok(crate::types::SetGetReply::ValueSet(_)));
                let result1_is_exists =
                    matches!(result1, Ok(crate::types::SetGetReply::ValueExists(_)));
                let result2_is_exists =
                    matches!(result2, Ok(crate::types::SetGetReply::ValueExists(_)));

                // Exactly one should be ValueSet and one should be ValueExists
                (result1_is_set && result2_is_exists) || (result1_is_exists && result2_is_set)
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_get_multiple_keys_success() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");

                // Set up test data
                let keys = vec![
                    "multi_test_key1".into(),
                    "multi_test_key2".into(),
                    "multi_test_key3".into(),
                ];
                let values = ["value1", "value2", "value3"];

                // Set the keys
                for (key, value) in keys.iter().zip(values.iter()) {
                    let _ = pool.set_key(key, value.to_string()).await;
                }

                // Act
                let result = pool.get_multiple_keys::<String>(&keys).await;

                // Assert
                match result {
                    Ok(retrieved_values) => {
                        retrieved_values.len() == 3
                            && retrieved_values.first() == Some(&Some("value1".to_string()))
                            && retrieved_values.get(1) == Some(&Some("value2".to_string()))
                            && retrieved_values.get(2) == Some(&Some("value3".to_string()))
                    }
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_get_multiple_keys_with_missing_keys() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");

                let keys = vec![
                    "existing_key".into(),
                    "non_existing_key".into(),
                    "another_existing_key".into(),
                ];

                // Set only some keys
                let _ = pool
                    .set_key(
                        keys.first().expect("should not be none"),
                        "value1".to_string(),
                    )
                    .await;
                let _ = pool
                    .set_key(
                        keys.get(2).expect("should not be none"),
                        "value3".to_string(),
                    )
                    .await;

                // Act
                let result = pool.get_multiple_keys::<String>(&keys).await;

                // Assert
                match result {
                    Ok(retrieved_values) => {
                        retrieved_values.len() == 3
                            && *retrieved_values.first().expect("should not be none")
                                == Some("value1".to_string())
                            && retrieved_values.get(1).is_some_and(|v| v.is_none())
                            && *retrieved_values.get(2).expect("should not be none")
                                == Some("value3".to_string())
                    }
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_get_multiple_keys_empty_input() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");

                let keys: Vec<crate::types::RedisKey> = vec![];

                // Act
                let result = pool.get_multiple_keys::<String>(&keys).await;

                // Assert
                match result {
                    Ok(retrieved_values) => retrieved_values.is_empty(),
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_get_and_deserialize_multiple_keys() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");

                #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
                struct TestData {
                    id: u32,
                    name: String,
                }

                let test_data = [
                    TestData {
                        id: 1,
                        name: "test1".to_string(),
                    },
                    TestData {
                        id: 2,
                        name: "test2".to_string(),
                    },
                ];

                let keys = vec![
                    "serialize_test_key1".into(),
                    "serialize_test_key2".into(),
                    "non_existing_serialize_key".into(),
                ];

                // Set serialized data for first two keys
                for (i, data) in test_data.iter().enumerate() {
                    let _ = pool
                        .serialize_and_set_key(keys.get(i).expect("should not be none"), data)
                        .await;
                }

                // Act
                let result = pool
                    .get_and_deserialize_multiple_keys::<TestData>(&keys, "TestData")
                    .await;

                // Assert
                match result {
                    Ok(retrieved_data) => {
                        retrieved_data.len() == 3
                            && retrieved_data.first() == Some(&Some(test_data[0].clone()))
                            && retrieved_data.get(1) == Some(&Some(test_data[1].clone()))
                            && retrieved_data.get(2) == Some(&None)
                    }
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }
}
