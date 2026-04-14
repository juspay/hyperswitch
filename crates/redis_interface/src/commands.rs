//! An interface to abstract the `redis` commands
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
use futures::StreamExt;
use redis::{
    streams::{StreamReadOptions, StreamTrimOptions, StreamTrimmingMode},
    AsyncCommands, ExistenceCheck, FromRedisValue, ScanOptions, SetExpiry, SetOptions,
    ToSingleRedisArg,
};
use router_env::tracing;
use tracing::instrument;

use crate::{
    errors,
    types::{
        DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisKey, SaddReply, SetGetReply,
        SetnxReply, StreamCapKind, StreamCapTrim, Value,
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

    // ─── Key Commands ────────────────────────────────────────────────────────

    #[instrument(level = "DEBUG", skip(self, value), fields(key = %key.tenant_aware_key(self)))]
    pub async fn set_key<V>(&self, key: &RedisKey, value: V) -> CustomResult<(), errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + Send + Sync + ToSingleRedisArg,
    {
        let mut conn = self.pool.clone();
        let options =
            SetOptions::default().with_expiration(SetExpiry::EX(self.config.default_ttl as u64));
        let _: Option<String> = conn
            .set_options(key.tenant_aware_key(self), value, options)
            .await
            .change_context(errors::RedisError::SetFailed)?;
        Ok(())
    }

    pub async fn set_key_without_modifying_ttl<V>(
        &self,
        key: &RedisKey,
        value: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + Send + Sync + ToSingleRedisArg,
    {
        let mut conn = self.pool.clone();
        let options = SetOptions::default().with_expiration(SetExpiry::KEEPTTL);
        let _: Option<String> = conn
            .set_options(key.tenant_aware_key(self), value, options)
            .await
            .change_context(errors::RedisError::SetFailed)?;
        Ok(())
    }

    pub async fn set_multiple_keys_if_not_exist<K, V>(
        &self,
        items: &[(K, V)],
    ) -> CustomResult<MsetnxReply, errors::RedisError>
    where
        K: redis::ToRedisArgs + Debug + Send + Sync,
        V: redis::ToRedisArgs + Debug + Send + Sync,
    {
        let mut conn = self.pool.clone();
        conn.mset_nx(items)
            .await
            .change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self, value), fields(key = %key.tenant_aware_key(self)))]
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

    #[instrument(level = "DEBUG", skip(self, value), fields(key = %key.tenant_aware_key(self)))]
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

    #[instrument(level = "DEBUG", skip(self, value), fields(key = %key.tenant_aware_key(self)))]
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

    #[instrument(level = "DEBUG", skip(self, value), fields(key = %key.tenant_aware_key(self), ttl_seconds = seconds))]
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

        let mut conn = self.pool.clone();
        let options = SetOptions::default().with_expiration(SetExpiry::EX(seconds as u64));
        let _: Option<String> = conn
            .set_options(key.tenant_aware_key(self), serialized.as_slice(), options)
            .await
            .change_context(errors::RedisError::SetExFailed)?;
        Ok(())
    }

    #[instrument(level = "DEBUG", skip(self), fields(key = %key.tenant_aware_key(self)))]
    pub async fn get_key<V>(&self, key: &RedisKey) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedisValue + Send + 'static,
    {
        let mut conn = self.pool.clone();
        match conn
            .get::<_, V>(key.tenant_aware_key(self))
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
                    conn.get::<_, V>(key.tenant_unaware_key(self))
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
        V: FromRedisValue + Send + 'static,
    {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let tenant_aware_keys: Vec<String> =
            keys.iter().map(|key| key.tenant_aware_key(self)).collect();
        let mut conn = self.pool.clone();
        conn.mget(&tenant_aware_keys)
            .await
            .change_context(errors::RedisError::GetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    async fn get_multiple_keys_with_parallel_get<V>(
        &self,
        keys: &[RedisKey],
    ) -> CustomResult<Vec<Option<V>>, errors::RedisError>
    where
        V: FromRedisValue + Send + 'static,
    {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        let tenant_aware_keys: Vec<String> =
            keys.iter().map(|key| key.tenant_aware_key(self)).collect();

        let futures = tenant_aware_keys.iter().map(|redis_key| {
            let mut conn = self.pool.clone();
            let key = redis_key.clone();
            async move { conn.get::<_, Option<V>>(&key).await }
        });

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
        V: FromRedisValue + Send + 'static,
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
        V: FromRedisValue + Send + 'static,
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
        V: Send + 'static,
    {
        let mut conn = self.pool.clone();
        match conn
            .exists::<_, bool>(key.tenant_aware_key(self))
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
                    conn.exists::<_, bool>(key.tenant_unaware_key(self))
                        .await
                        .change_context(errors::RedisError::GetFailed)
                }
            }
        }
    }

    #[instrument(level = "DEBUG", skip(self), fields(key = %key.tenant_aware_key(self), type_name = type_name))]
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

    #[instrument(level = "DEBUG", skip(self), fields(key = %key.tenant_aware_key(self)))]
    pub async fn delete_key(&self, key: &RedisKey) -> CustomResult<DelReply, errors::RedisError> {
        let mut conn = self.pool.clone();
        let deleted_count: usize = conn
            .del(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::DeleteFailed)?;

        let reply = if deleted_count > 0 {
            DelReply::KeyDeleted
        } else {
            DelReply::KeyNotDeleted
        };

        match reply {
            DelReply::KeyDeleted => Ok(reply),
            DelReply::KeyNotDeleted => {
                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Ok(reply)
                }

                #[cfg(feature = "multitenancy_fallback")]
                {
                    let fallback_count: usize = conn
                        .del(key.tenant_unaware_key(self))
                        .await
                        .change_context(errors::RedisError::DeleteFailed)?;
                    Ok(if fallback_count > 0 {
                        DelReply::KeyDeleted
                    } else {
                        DelReply::KeyNotDeleted
                    })
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

    #[instrument(level = "DEBUG", skip(self, value), fields(key = %key.tenant_aware_key(self), ttl_seconds = seconds))]
    pub async fn set_key_with_expiry<V>(
        &self,
        key: &RedisKey,
        value: V,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + Send + Sync + ToSingleRedisArg,
    {
        let mut conn = self.pool.clone();
        let options = SetOptions::default().with_expiration(SetExpiry::EX(seconds as u64));
        let _: Option<String> = conn
            .set_options(key.tenant_aware_key(self), value, options)
            .await
            .change_context(errors::RedisError::SetExFailed)?;
        Ok(())
    }

    #[instrument(level = "DEBUG", skip(self, value), fields(key = %key.tenant_aware_key(self), ttl_seconds = seconds.unwrap_or(self.config.default_ttl.into())))]
    pub async fn set_key_if_not_exists_with_expiry<V>(
        &self,
        key: &RedisKey,
        value: V,
        seconds: Option<i64>,
    ) -> CustomResult<SetnxReply, errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + Send + Sync + ToSingleRedisArg,
    {
        let ttl = seconds.unwrap_or(self.config.default_ttl.into());
        let mut conn = self.pool.clone();
        let options = SetOptions::default()
            .conditional_set(ExistenceCheck::NX)
            .with_expiration(SetExpiry::EX(ttl as u64));
        let result: Option<String> = conn
            .set_options(key.tenant_aware_key(self), value, options)
            .await
            .change_context(errors::RedisError::SetFailed)?;

        // SET NX returns OK on success, nil if key already exists
        let reply = if result.is_some() {
            SetnxReply::KeySet
        } else {
            SetnxReply::KeyNotSet
        };
        Ok(reply)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expiry(
        &self,
        key: &RedisKey,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.expire::<_, ()>(key.tenant_aware_key(self), seconds)
            .await
            .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expire_at(
        &self,
        key: &RedisKey,
        timestamp: i64,
    ) -> CustomResult<(), errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.expire_at::<_, ()>(key.tenant_aware_key(self), timestamp)
            .await
            .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_ttl(&self, key: &RedisKey) -> CustomResult<i64, errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.ttl::<_, i64>(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetFailed)
    }

    // ─── Hash Commands ───────────────────────────────────────────────────────

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_hash_fields<F, V>(
        &self,
        key: &RedisKey,
        items: &[(F, V)],
        ttl: Option<i64>,
    ) -> CustomResult<(), errors::RedisError>
    where
        F: redis::ToRedisArgs + Debug + Send + Sync,
        V: redis::ToRedisArgs + Debug + Send + Sync,
    {
        let mut conn = self.pool.clone();
        let _: () = conn
            .hset_multiple(key.tenant_aware_key(self), items)
            .await
            .change_context(errors::RedisError::SetHashFailed)?;

        // setting expiry for the key
        self.set_expiry(key, ttl.unwrap_or(self.config.default_hash_ttl.into()))
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
        V: redis::ToRedisArgs + ToSingleRedisArg + Debug + Send + Sync,
    {
        let mut conn = self.pool.clone();
        let output: Result<HsetnxReply, _> = conn
            .hset_nx::<_, _, _, HsetnxReply>(key.tenant_aware_key(self), field, value)
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
        for (field, increment) in fields_to_increment.iter() {
            let mut conn = self.pool.clone();
            values_after_increment.push(
                conn.hincr::<_, _, _, usize>(
                    key.tenant_aware_key(self),
                    field.to_string(),
                    *increment,
                )
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
        let mut conn = self.pool.clone();

        // Build HSCAN command with MATCH and optional COUNT
        let mut cmd = redis::cmd("HSCAN");
        cmd.arg(key.tenant_aware_key(self))
            .arg("MATCH")
            .arg(pattern);

        if let Some(c) = count {
            cmd.arg("COUNT").arg(c);
        }

        // Use iter_async to get an async iterator that handles cursor management
        let mut iter = cmd
            .iter_async::<String>(&mut conn)
            .await
            .change_context(errors::RedisError::GetHashFieldFailed)?;

        // HSCAN returns alternating field/value pairs; we want the values (odd indices)
        let mut results: Vec<String> = Vec::new();
        let mut index = 0;
        while let Some(item) = iter.next().await {
            let item = item.change_context(errors::RedisError::GetHashFieldFailed)?;
            if index % 2 == 1 {
                results.push(item);
            }
            index += 1;
        }

        Ok(results)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn scan(
        &self,
        pattern: &RedisKey,
        count: Option<u32>,
        _scan_type: Option<()>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        let mut conn = self.pool.clone();

        // Build ScanOptions with MATCH pattern and optional COUNT
        let mut opts = ScanOptions::default().with_pattern(pattern.tenant_aware_key(self));

        if let Some(c) = count {
            opts = opts.with_count(c as usize);
        }

        // Use scan_options which returns an AsyncIter
        let mut iter = conn
            .scan_options(opts)
            .await
            .change_context(errors::RedisError::GetFailed)?;

        // Collect all items from the iterator
        let mut results: Vec<String> = Vec::new();
        while let Some(item) = iter.next().await {
            let item = item.change_context(errors::RedisError::GetFailed)?;
            results.push(item);
        }

        Ok(results)
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
        V: FromRedisValue + Send + 'static,
    {
        let mut conn = self.pool.clone();
        match conn
            .hget::<_, _, V>(key.tenant_aware_key(self), field)
            .await
            .change_context(errors::RedisError::GetHashFieldFailed)
        {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(feature = "multitenancy_fallback")]
                {
                    conn.hget::<_, _, V>(key.tenant_unaware_key(self), field)
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
        V: FromRedisValue + Send + 'static,
    {
        let mut conn = self.pool.clone();
        match conn
            .hgetall::<_, V>(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetHashFieldFailed)
        {
            Ok(v) => Ok(v),
            Err(_err) => {
                #[cfg(feature = "multitenancy_fallback")]
                {
                    conn.hgetall::<_, V>(key.tenant_unaware_key(self))
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

    // ─── Set Commands ────────────────────────────────────────────────────────

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn sadd<V>(
        &self,
        key: &RedisKey,
        members: V,
    ) -> CustomResult<SaddReply, errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + Send + Sync,
    {
        let mut conn = self.pool.clone();
        conn.sadd::<_, _, SaddReply>(key.tenant_aware_key(self), members)
            .await
            .change_context(errors::RedisError::SetAddMembersFailed)
    }

    // ─── Stream Commands ─────────────────────────────────────────────────────

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_append_entry<F>(
        &self,
        stream: &RedisKey,
        entry_id: &RedisEntryId,
        fields: F,
    ) -> CustomResult<(), errors::RedisError>
    where
        F: redis::ToRedisArgs + Debug + Send + Sync,
    {
        let mut conn = self.pool.clone();
        let _: Option<String> = conn
            .xadd_map(
                stream.tenant_aware_key(self),
                entry_id.to_stream_id(),
                fields,
            )
            .await
            .change_context(errors::RedisError::StreamAppendFailed)?;
        Ok(())
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_delete_entries(
        &self,
        stream: &RedisKey,
        ids: &[String],
    ) -> CustomResult<usize, errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.xdel(stream.tenant_aware_key(self), ids)
            .await
            .change_context(errors::RedisError::StreamDeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_trim_entries(
        &self,
        stream: &RedisKey,
        cap_kind: StreamCapKind,
        cap_trim: StreamCapTrim,
        threshold: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        let mut conn = self.pool.clone();

        let trim_mode = match cap_trim {
            StreamCapTrim::AlmostExact => StreamTrimmingMode::Approx,
            StreamCapTrim::Exact => StreamTrimmingMode::Exact,
        };

        let options = match cap_kind {
            StreamCapKind::MaxLen => {
                let max_len: usize = threshold
                    .parse()
                    .map_err(|_| errors::RedisError::StreamTrimFailed)?;
                StreamTrimOptions::maxlen(trim_mode, max_len)
            }
            StreamCapKind::MinID => StreamTrimOptions::minid(trim_mode, threshold),
        };

        conn.xtrim_options(stream.tenant_aware_key(self), &options)
            .await
            .change_context(errors::RedisError::StreamTrimFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_acknowledge_entries(
        &self,
        stream: &RedisKey,
        group: &str,
        ids: &[String],
    ) -> CustomResult<usize, errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.xack(stream.tenant_aware_key(self), group, ids)
            .await
            .change_context(errors::RedisError::StreamAcknowledgeFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_get_length(
        &self,
        stream: &RedisKey,
    ) -> CustomResult<usize, errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.xlen(stream.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetLengthFailed)
    }

    /// Read entries from one or more streams using XREAD
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_entries(
        &self,
        streams: &[RedisKey],
        ids: &[String],
        read_count: Option<u64>,
    ) -> CustomResult<redis::streams::StreamReadReply, errors::RedisError> {
        let mut conn = self.pool.clone();
        let stream_keys: Vec<String> = streams.iter().map(|s| s.tenant_aware_key(self)).collect();

        let count = read_count.unwrap_or(self.config.default_stream_read_count);

        let options = StreamReadOptions::default().count(count as usize);

        conn.xread_options(&stream_keys, ids, &options)
            .await
            .map_err(|err| {
                let kind = err.kind();
                match kind {
                    redis::ErrorKind::UnexpectedReturnType | redis::ErrorKind::Parse => {
                        report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
                    }
                    _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
                }
            })
    }

    /// Read entries from streams with options (XREAD / XREADGROUP)
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_with_options(
        &self,
        streams: &[RedisKey],
        ids: &[String],
        count: Option<u64>,
        block: Option<u64>,
        group: Option<(&str, &str)>,
    ) -> CustomResult<redis::streams::StreamReadReply, errors::RedisError> {
        let mut conn = self.pool.clone();
        let stream_keys: Vec<String> = streams.iter().map(|s| s.tenant_aware_key(self)).collect();

        let mut options = StreamReadOptions::default();

        if let Some(count_val) = count {
            options = options.count(count_val as usize);
        }
        if let Some(block_ms) = block {
            options = options.block(block_ms as usize);
        }
        if let Some((group_name, consumer_name)) = group {
            options = options.group(group_name, consumer_name);
        }

        conn.xread_options(&stream_keys, ids, &options)
            .await
            .map_err(|err| {
                let kind = err.kind();
                match kind {
                    redis::ErrorKind::UnexpectedReturnType | redis::ErrorKind::Parse => {
                        report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
                    }
                    _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
                }
            })
    }

    // ─── List Commands ───────────────────────────────────────────────────────

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn append_elements_to_list<V>(
        &self,
        key: &RedisKey,
        elements: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + Send + Sync,
    {
        let mut conn = self.pool.clone();
        conn.rpush::<_, _, ()>(key.tenant_aware_key(self), elements)
            .await
            .change_context(errors::RedisError::AppendElementsToListFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_list_elements(
        &self,
        key: &RedisKey,
        start: isize,
        stop: isize,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.lrange::<_, Vec<String>>(key.tenant_aware_key(self), start, stop)
            .await
            .change_context(errors::RedisError::GetListElementsFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_list_length(&self, key: &RedisKey) -> CustomResult<usize, errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.llen::<_, usize>(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::GetListLengthFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn lpop_list_elements(
        &self,
        key: &RedisKey,
        count: Option<usize>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        let mut conn = self.pool.clone();
        let non_zero_count = count.and_then(std::num::NonZeroUsize::new);
        conn.lpop::<_, Vec<String>>(key.tenant_aware_key(self), non_zero_count)
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
            Err(errors::RedisError::InvalidRedisEntryId)?;
        }

        let mut conn = self.pool.clone();
        let _: () = conn
            .xgroup_create_mkstream(stream.tenant_aware_key(self), group, id.to_stream_id())
            .await
            .change_context(errors::RedisError::ConsumerGroupCreateFailed)?;
        Ok(())
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_destroy(
        &self,
        stream: &RedisKey,
        group: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        let mut conn = self.pool.clone();
        let destroyed: bool = conn
            .xgroup_destroy(stream.tenant_aware_key(self), group)
            .await
            .change_context(errors::RedisError::ConsumerGroupDestroyFailed)?;
        Ok(if destroyed { 1 } else { 0 })
    }

    // the number of pending messages that the consumer had before it was deleted
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_delete_consumer(
        &self,
        stream: &RedisKey,
        group: &str,
        consumer: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        let mut conn = self.pool.clone();
        conn.xgroup_delconsumer(stream.tenant_aware_key(self), group, consumer)
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
        let mut conn = self.pool.clone();
        let _: () = conn
            .xgroup_setid(stream.tenant_aware_key(self), group, id.to_stream_id())
            .await
            .change_context(errors::RedisError::ConsumerGroupSetIdFailed)?;
        Ok(id.to_stream_id().to_string())
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn consumer_group_set_message_owner<R>(
        &self,
        stream: &RedisKey,
        group: &str,
        consumer: &str,
        min_idle_time: u64,
        ids: &[String],
    ) -> CustomResult<R, errors::RedisError>
    where
        R: FromRedisValue + Send + 'static,
    {
        let mut conn = self.pool.clone();
        conn.xclaim(
            stream.tenant_aware_key(self),
            group,
            consumer,
            min_idle_time,
            ids,
        )
        .await
        .change_context(errors::RedisError::ConsumerGroupClaimFailed)
    }

    // ─── Lua Scripting ───────────────────────────────────────────────────────

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn evaluate_redis_script<V, T>(
        &self,
        lua_script: &'static str,
        key: Vec<String>,
        values: V,
    ) -> CustomResult<T, errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + Send + Sync,
        T: serde::de::DeserializeOwned + FromRedisValue,
    {
        let mut conn = self.pool.clone();
        let script = redis::Script::new(lua_script);
        let mut invocation = script.prepare_invoke();

        for k in &key {
            invocation.key(k);
        }
        invocation.arg(values);

        let val: T = invocation
            .invoke_async(&mut conn)
            .await
            .change_context(errors::RedisError::ScriptExecutionFailed)?;
        Ok(val)
    }

    // ─── Transactions ────────────────────────────────────────────────────────

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_multiple_keys_if_not_exists_and_get_values<V>(
        &self,
        keys: &[(RedisKey, V)],
        ttl: Option<i64>,
    ) -> CustomResult<Vec<SetGetReply<V>>, errors::RedisError>
    where
        V: redis::ToRedisArgs
            + Debug
            + FromRedisValue
            + ToOwned<Owned = V>
            + Send
            + Sync
            + serde::de::DeserializeOwned,
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
    /// This operation is atomic using Redis transactions (MULTI/EXEC pipeline).
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key_if_not_exists_and_get_value<V>(
        &self,
        key: &RedisKey,
        value: V,
        ttl: Option<i64>,
    ) -> CustomResult<SetGetReply<V>, errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + FromRedisValue + Send + Sync + serde::de::DeserializeOwned,
    {
        let redis_key = key.tenant_aware_key(self);
        let ttl_seconds = ttl.unwrap_or(self.config.default_ttl.into());

        let mut conn = self.pool.clone();

        // Build an atomic pipeline (MULTI/EXEC)
        let mut pipe = redis::pipe();
        pipe.atomic();

        // SET key value EX ttl NX
        pipe.cmd("SET")
            .arg(&redis_key)
            .arg(&value)
            .arg("EX")
            .arg(ttl_seconds)
            .arg("NX");

        // GET key
        pipe.cmd("GET").arg(&redis_key);

        // Execute the transaction
        let results: Vec<Value> = pipe
            .query_async(&mut conn)
            .await
            .change_context(errors::RedisError::SetFailed)
            .attach_printable("Failed to execute the redis transaction")?;

        let msg = "Got unexpected number of results from transaction";
        if results.len() < 2 {
            return Err(report!(errors::RedisError::SetFailed).attach_printable(msg));
        }

        let set_result = results[0].clone();
        let get_result = results[1].clone();

        // Parse the GET result to get the actual value
        let actual_value: V = FromRedisValue::from_redis_value(get_result)
            .change_context(errors::RedisError::SetFailed)
            .attach_printable("Failed to convert from redis value")?;

        // Check if SET NX succeeded or failed
        match set_result {
            // SET NX returns Okay if key was set (newer redis crate)
            Value::Okay => Ok(SetGetReply::ValueSet(actual_value)),
            // SET NX returns "OK" if key was set (older format)
            Value::SimpleString(ref s) if s == "OK" => Ok(SetGetReply::ValueSet(actual_value)),
            Value::BulkString(ref s) if s == b"OK" => Ok(SetGetReply::ValueSet(actual_value)),
            // SET NX returns Nil if key already exists
            Value::Nil => Ok(SetGetReply::ValueExists(actual_value)),
            _ => Err(report!(errors::RedisError::SetFailed))
                .attach_printable("Unexpected result from SET NX operation"),
        }
    }
}

/// Custom reply types for delete and set operations to indicate whether the key was actually deleted/set or not.
#[cfg(test)]
mod tests {
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
                let mut keys_and_values = std::collections::HashMap::new();
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
                let unique_id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let key: crate::types::RedisKey =
                    format!("test_default_ttl_key_{}", unique_id).into();
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

    #[tokio::test]
    async fn test_set_key_with_expiry() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key = "test_set_with_expiry_key".into();
                let value = "test_value".to_string();

                // Act
                let result = pool.set_key_with_expiry(&key, value.clone(), 60).await;

                // Assert - should succeed
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_key_without_modifying_ttl() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key = "test_set_keep_ttl_key".into();
                let initial_value = "initial".to_string();
                let new_value = "new_value".to_string();

                // First set a key with TTL
                let _ = pool.set_key_with_expiry(&key, initial_value, 60).await;

                // Act - update value while keeping TTL
                let result = pool
                    .set_key_without_modifying_ttl(&key, new_value.clone())
                    .await;

                // Assert - should succeed
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_key_if_not_exists_with_expiry_new_key() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key = "test_setnx_new_key".into();
                let value = "test_value".to_string();

                // Act
                let result = pool
                    .set_key_if_not_exists_with_expiry(&key, value.clone(), Some(60))
                    .await;

                // Assert - should return KeySet
                matches!(result, Ok(crate::types::SetnxReply::KeySet))
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_key_if_not_exists_with_expiry_existing_key() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key = "test_setnx_existing_key".into();
                let initial_value = "initial".to_string();
                let new_value = "new_value".to_string();

                // First set a key
                let _ = pool.set_key(&key, initial_value.clone()).await;

                // Act - try to set again (should fail)
                let result = pool
                    .set_key_if_not_exists_with_expiry(&key, new_value.clone(), Some(60))
                    .await;

                // Assert - should return KeyNotSet
                matches!(result, Ok(crate::types::SetnxReply::KeyNotSet))
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_hash_fields() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key = "test_hash_fields_key".into();
                let fields: Vec<(&str, &str)> = vec![
                    ("field1", "value1"),
                    ("field2", "value2"),
                    ("field3", "value3"),
                ];

                // Act
                let result = pool.set_hash_fields(&key, &fields, Some(60)).await;

                // Assert - should succeed
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_stream_append_and_get_length() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let stream = "test_stream_append_key".into();
                let fields: Vec<(&str, &str)> = vec![("field1", "value1")];

                // Act - append entry
                let append_result = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;

                // Get length
                let length_result = pool.stream_get_length(&stream).await;

                // Assert
                append_result.is_ok() && length_result.is_ok() && length_result.unwrap() >= 1
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_consumer_group_operations() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let stream: crate::types::RedisKey =
                    format!("test_stream_group_ops_{}", unique_id).into();
                let group = format!("test_group_{}", unique_id);
                let fields: Vec<(&str, &str)> = vec![("field1", "value1")];

                // First add an entry
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;

                // Act - create group with valid ID
                let create_result = pool
                    .consumer_group_create(
                        &stream,
                        &group,
                        &RedisEntryId::UserSpecifiedID {
                            milliseconds: "0".to_string(),
                            sequence_number: "0".to_string(),
                        },
                    )
                    .await;

                // Assert - should succeed
                create_result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_stream_acknowledge_entries() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let stream = "test_stream_ack_key".into();
                let group = "test_ack_group";
                let fields: Vec<(&str, &str)> = vec![("field1", "value1")];

                // Setup - add entry and create group
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;
                let _ = pool
                    .consumer_group_create(
                        &stream,
                        group,
                        &RedisEntryId::UserSpecifiedID {
                            milliseconds: "0".to_string(),
                            sequence_number: "0".to_string(),
                        },
                    )
                    .await;

                // Act - acknowledge non-existent ID (should still succeed with 0)
                let ack_result = pool
                    .stream_acknowledge_entries(&stream, group, &["0-1".to_string()])
                    .await;

                // Assert
                ack_result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_delete_key_returns_correct_reply() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let existing_key = "test_delete_existing".into();
                let non_existing_key = "test_delete_non_existing".into();

                // Set up an existing key
                let _ = pool.set_key(&existing_key, "value".to_string()).await;

                // Act
                let delete_existing = pool.delete_key(&existing_key).await;
                let delete_non_existing = pool.delete_key(&non_existing_key).await;

                // Assert
                matches!(delete_existing, Ok(crate::types::DelReply::KeyDeleted))
                    && matches!(
                        delete_non_existing,
                        Ok(crate::types::DelReply::KeyNotDeleted)
                    )
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }
}
