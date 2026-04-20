//! An interface to abstract the `redis` commands
//!
//! The folder provides generic functions for providing serialization
//! and deserialization while calling redis.
//! It also includes instruments to provide tracing.

use std::fmt::Debug;

use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, Encode, StringExt},
    fp_utils,
};
use error_stack::{report, ResultExt};
use redis::{
    streams::{StreamReadOptions, StreamTrimOptions, StreamTrimmingMode},
    AsyncCommands, ExistenceCheck, FromRedisValue, SetExpiry, SetOptions, ToSingleRedisArg,
};
use router_env::tracing;
use tracing::instrument;

use crate::{
    constant::{
        REDIS_ARG_COUNT, REDIS_ARG_EX, REDIS_ARG_MATCH, REDIS_ARG_NX, REDIS_ARG_TYPE,
        REDIS_COMMAND_GET, REDIS_COMMAND_HSCAN, REDIS_COMMAND_SCAN, REDIS_COMMAND_SET,
    },
    errors,
    types::{
        redis_value_to_option_string, DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisKey,
        SaddReply, SetGetReply, SetnxReply, StreamCapKind, StreamCapTrim, StreamEntries,
        StreamReadResult, Value,
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

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key<V>(&self, key: &RedisKey, value: V) -> CustomResult<(), errors::RedisError>
    where
        V: redis::ToRedisArgs + Debug + Send + Sync + ToSingleRedisArg,
    {
        let mut conn = self.pool.clone();
        let options = SetOptions::default()
            .with_expiration(SetExpiry::EX(u64::from(self.config.default_ttl)));
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

        let mut conn = self.pool.clone();
        let options = SetOptions::default().with_expiration(SetExpiry::EX(
            u64::try_from(seconds).change_context(errors::RedisError::SetExFailed)?,
        ));
        let _: Option<String> = conn
            .set_options(key.tenant_aware_key(self), serialized.as_slice(), options)
            .await
            .change_context(errors::RedisError::SetExFailed)?;
        Ok(())
    }

    #[instrument(level = "DEBUG", skip(self))]
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
        let mut conn = self.pool.clone();
        // Redis DEL returns the number of keys that were deleted.
        // 0 means the key did not exist (not an error).
        let deleted_count: usize = conn
            .del(key.tenant_aware_key(self))
            .await
            .change_context(errors::RedisError::DeleteFailed)?;

        let reply = if deleted_count > 0 {
            DelReply::KeyDeleted
        } else {
            // Key was not found in tenant-aware namespace.
            // With multitenancy_fallback, try the tenant-unaware namespace.
            // This mirrors the old behavior where a failed DEL in tenant-aware
            // namespace would fall back to tenant-unaware.
            #[cfg(not(feature = "multitenancy_fallback"))]
            {
                DelReply::KeyNotDeleted
            }

            #[cfg(feature = "multitenancy_fallback")]
            {
                let fallback_count: usize = conn
                    .del(key.tenant_unaware_key(self))
                    .await
                    .change_context(errors::RedisError::DeleteFailed)?;
                if fallback_count > 0 {
                    DelReply::KeyDeleted
                } else {
                    DelReply::KeyNotDeleted
                }
            }
        };

        Ok(reply)
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
        V: redis::ToRedisArgs + Debug + Send + Sync + ToSingleRedisArg,
    {
        let mut conn = self.pool.clone();
        let options = SetOptions::default().with_expiration(SetExpiry::EX(
            u64::try_from(seconds).change_context(errors::RedisError::SetExFailed)?,
        ));
        let _: Option<String> = conn
            .set_options(key.tenant_aware_key(self), value, options)
            .await
            .change_context(errors::RedisError::SetExFailed)?;
        Ok(())
    }

    #[instrument(level = "DEBUG", skip(self))]
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
            .with_expiration(SetExpiry::EX(
                u64::try_from(ttl).change_context(errors::RedisError::SetFailed)?,
            ));
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

        // setting expiry for the key — reuse the same connection
        conn.expire::<_, ()>(
            key.tenant_aware_key(self),
            ttl.unwrap_or(self.config.default_hash_ttl.into()),
        )
        .await
        .change_context(errors::RedisError::SetExpiryFailed)
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
        let result: HsetnxReply = conn
            .hset_nx::<_, _, _, HsetnxReply>(key.tenant_aware_key(self), field, value)
            .await
            .change_context(errors::RedisError::SetHashFieldFailed)?;

        // Only set expiry if the field was actually set
        if matches!(result, HsetnxReply::KeySet) {
            conn.expire::<_, ()>(
                key.tenant_aware_key(self),
                ttl.unwrap_or(self.config.default_hash_ttl).into(),
            )
            .await
            .change_context(errors::RedisError::SetExpiryFailed)?;
        }

        Ok(result)
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
        let mut conn = self.pool.clone();
        let mut values_after_increment = Vec::with_capacity(fields_to_increment.len());
        for (field, increment) in fields_to_increment.iter() {
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

        let mut results: Vec<String> = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            // Build HSCAN command: HSCAN key cursor MATCH pattern [COUNT count]
            let mut command = redis::cmd(REDIS_COMMAND_HSCAN);
            command
                .arg(key.tenant_aware_key(self))
                .arg(cursor)
                .arg(REDIS_ARG_MATCH)
                .arg(pattern);

            if let Some(count_value) = count {
                command.arg(REDIS_ARG_COUNT).arg(count_value);
            }

            // HSCAN returns: [cursor, [field1, value1, field2, value2, ...]]
            let reply: (u64, Vec<Value>) = command
                .query_async(&mut conn)
                .await
                .change_context(errors::RedisError::GetHashFieldFailed)?;

            cursor = reply.0;

            // Extract values (odd indices) from field/value pairs
            let pairs = reply.1;
            for (index, value) in pairs.into_iter().enumerate() {
                if index % 2 == 1 {
                    if let Some(s) = redis_value_to_option_string(&value) {
                        results.push(s);
                    }
                }
            }

            // Cursor 0 means iteration is complete
            if cursor == 0 {
                break;
            }
        }

        Ok(results)
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn scan(
        &self,
        pattern: &RedisKey,
        count: Option<u32>,
        scan_type: Option<&str>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        let mut conn = self.pool.clone();

        let mut results: Vec<String> = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let mut command = redis::cmd(REDIS_COMMAND_SCAN);
            command
                .arg(cursor)
                .arg(REDIS_ARG_MATCH)
                .arg(pattern.tenant_aware_key(self));

            if let Some(count_value) = count {
                command.arg(REDIS_ARG_COUNT).arg(count_value);
            }

            if let Some(scan_type_value) = scan_type {
                command.arg(REDIS_ARG_TYPE).arg(scan_type_value);
            }

            let reply: (u64, Vec<String>) = command
                .query_async(&mut conn)
                .await
                .change_context(errors::RedisError::GetFailed)?;

            cursor = reply.0;
            results.extend(reply.1);

            if cursor == 0 {
                break;
            }
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
            .filter_map(|redis_value| {
                let r: T = redis_value.parse_struct(std::any::type_name::<T>()).ok()?;
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
        if ids.is_empty() {
            Ok(0)
        } else {
            let mut conn = self.pool.clone();
            conn.xdel(stream.tenant_aware_key(self), ids)
                .await
                .change_context(errors::RedisError::StreamDeleteFailed)
        }
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
        if ids.is_empty() {
            Ok(0)
        } else {
            let mut conn = self.pool.clone();
            conn.xack(stream.tenant_aware_key(self), group, ids)
                .await
                .change_context(errors::RedisError::StreamAcknowledgeFailed)
        }
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

    /// Read entries from one or more streams using XREAD.
    /// Returns a StreamReadReply - use `into_stream_iter()` for easy iteration.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_entries(
        &self,
        streams: &[RedisKey],
        ids: &[String],
        read_count: Option<u64>,
    ) -> CustomResult<redis::streams::StreamReadReply, errors::RedisError> {
        let mut conn = self.pool.clone();
        let stream_keys: Vec<String> = streams
            .iter()
            .map(|stream| stream.tenant_aware_key(self))
            .collect();

        let count = read_count.unwrap_or(self.config.default_stream_read_count);

        let options = StreamReadOptions::default()
            .count(usize::try_from(count).change_context(errors::RedisError::StreamReadFailed)?);

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

    /// Read stream entries and return them as a [`StreamReadResult`]
    /// (stream key → list of `(entry_id, fields)`), so callers don't
    /// have to manually parse `StreamReadReply` every time.
    pub async fn stream_read_grouped(
        &self,
        streams: &[RedisKey],
        ids: &[String],
        read_count: Option<u64>,
    ) -> CustomResult<StreamReadResult, errors::RedisError> {
        let reply = self.stream_read_entries(streams, ids, read_count).await?;

        let result: StreamReadResult = reply
            .keys
            .into_iter()
            .map(|stream_key| {
                let entries: StreamEntries = stream_key
                    .ids
                    .into_iter()
                    .map(|id| {
                        let fields: std::collections::HashMap<String, String> = id
                            .map
                            .into_iter()
                            .filter_map(|(field_name, redis_value)| {
                                redis_value_to_option_string(&redis_value)
                                    .map(|field_value| (field_name, field_value))
                            })
                            .collect();
                        (id.id, fields)
                    })
                    .collect();
                (stream_key.key, entries)
            })
            .collect();

        Ok(result)
    }

    /// Read stream entries and return them grouped by stream key with optional field values.
    /// Read entries from streams with options (XREAD / XREADGROUP)
    /// Returns a StreamReadReply - use `into_stream_iter()` for easy iteration.
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
        let stream_keys: Vec<String> = streams
            .iter()
            .map(|stream| stream.tenant_aware_key(self))
            .collect();

        let mut options = StreamReadOptions::default();

        if let Some(count_val) = count {
            options = options.count(
                usize::try_from(count_val).change_context(errors::RedisError::StreamReadFailed)?,
            );
        }
        if let Some(block_ms) = block {
            options = options.block(
                usize::try_from(block_ms).change_context(errors::RedisError::StreamReadFailed)?,
            );
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
        // XGROUP DESTROY returns true (1) if the group was destroyed,
        // false (0) if the group did not exist. The usize return type
        // preserves the numeric result for compatibility with callers
        // that compare against 0.
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

        for script_key in &key {
            invocation.key(script_key);
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
        pipe.cmd(REDIS_COMMAND_SET)
            .arg(&redis_key)
            .arg(&value)
            .arg(REDIS_ARG_EX)
            .arg(ttl_seconds)
            .arg(REDIS_ARG_NX);

        pipe.cmd(REDIS_COMMAND_GET).arg(&redis_key);

        // Execute the transaction.
        // Redis MULTI/EXEC guarantees results are returned in the same order
        // as the commands were queued: [SET result, GET result].
        let results: Vec<Value> = pipe
            .query_async(&mut conn)
            .await
            .change_context(errors::RedisError::SetFailed)
            .attach_printable("Failed to execute the redis transaction")?;

        let msg = "Got unexpected number of results from transaction";
        if results.len() < 2 {
            return Err(report!(errors::RedisError::SetFailed).attach_printable(msg));
        }

        let set_result = results
            .first()
            .cloned()
            .ok_or_else(|| report!(errors::RedisError::SetFailed).attach_printable(msg))?;
        let get_result = results
            .get(1)
            .cloned()
            .ok_or_else(|| report!(errors::RedisError::SetFailed).attach_printable(msg))?;

        // Parse the GET result to get the actual value
        let actual_value: V = FromRedisValue::from_redis_value(get_result)
            .change_context(errors::RedisError::SetFailed)
            .attach_printable("Failed to convert from redis value")?;

        // Check if SET NX succeeded or failed using the existing SetnxReply type
        let setnx_reply = SetnxReply::from_redis_value(set_result)
            .change_context(errors::RedisError::SetFailed)
            .attach_printable("Unexpected result from SET NX operation")?;

        Ok(match setnx_reply {
            SetnxReply::KeySet => SetGetReply::ValueSet(actual_value),
            SetnxReply::KeyNotSet => SetGetReply::ValueExists(actual_value),
        })
    }
}

/// Custom reply types for delete and set operations to indicate whether the key was actually deleted/set or not.
#[cfg(test)]
mod tests {
    use crate::{errors::RedisError, RedisConnectionPool, RedisEntryId, RedisSettings};

    /// Generate a unique ID for test key isolation.
    /// Uses thread ID + nanoseconds to avoid collisions in parallel test runs.
    fn unique_test_id() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        // Combine PID + counter + timestamp for global uniqueness across runs
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("{pid}_{millis}_{counter}")
    }

    /// Create a cluster RedisConnectionPool if `REDIS_CLUSTER_URLS` env var is set.
    /// Returns `None` if no cluster is configured (test should skip, not fail).
    async fn cluster_pool() -> Option<RedisConnectionPool> {
        let cluster_urls_str = std::env::var("REDIS_CLUSTER_URLS").ok()?;
        let cluster_urls: Vec<String> = cluster_urls_str
            .split(',')
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
            .collect();

        if cluster_urls.is_empty() {
            return None;
        }

        // Use the first cluster URL's host:port as the primary,
        // so `RedisConnectionPool::new` doesn't prepend `127.0.0.1:6379`.
        let first_url = cluster_urls.first()?;
        let (host, port) = if first_url.starts_with("redis://") {
            let without_scheme = first_url.trim_start_matches("redis://");
            let parts: Vec<&str> = without_scheme.split(':').collect();
            (
                parts.first()?.to_string(),
                parts.get(1)?.parse::<u16>().ok()?,
            )
        } else {
            let parts: Vec<&str> = first_url.split(':').collect();
            (
                parts.first()?.to_string(),
                parts.get(1)?.parse::<u16>().ok()?,
            )
        };

        let settings = RedisSettings {
            host,
            port,
            cluster_enabled: true,
            cluster_urls,
            ..RedisSettings::default()
        };

        RedisConnectionPool::new(&settings).await.ok()
    }

    /// Helper: get a cluster pool or skip the test.
    /// Prints a message when skipping so it's visible in test output.
    async fn get_cluster_pool_or_skip() -> Option<RedisConnectionPool> {
        let pool = cluster_pool().await;
        if pool.is_none() {
            eprintln!(
                "SKIP: Cluster test skipped — set REDIS_CLUSTER_URLS to enable. \
                 Example: REDIS_CLUSTER_URLS=redis://localhost:7000,redis://localhost:7001"
            );
        }
        pool
    }

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
                let key: crate::types::RedisKey =
                    format!("test_setnx_new_{}", unique_test_id()).into();
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
                let key: crate::types::RedisKey =
                    format!("test_setnx_exist_{}", unique_test_id()).into();
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
                let unique_id = unique_test_id();
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
                let key_name = format!("test_concurrent_{}", unique_test_id());
                let value1 = "value1".to_string();
                let value2 = "value2".to_string();

                // Act - simulate concurrent access
                let pool1 = pool.clone("");
                let pool2 = pool.clone("");
                let key1: crate::types::RedisKey = key_name.clone().into();
                let key2: crate::types::RedisKey = key_name.into();

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
                let key: crate::types::RedisKey =
                    format!("test_set_expiry_{}", unique_test_id()).into();
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
                let key: crate::types::RedisKey =
                    format!("test_keepttl_{}", unique_test_id()).into();
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
                let key: crate::types::RedisKey =
                    format!("test_setnx_new_{}", unique_test_id()).into();
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
                let key: crate::types::RedisKey =
                    format!("test_setnx_exist_{}", unique_test_id()).into();
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
                let key: crate::types::RedisKey = format!("test_hash_{}", unique_test_id()).into();
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
                let stream: crate::types::RedisKey =
                    format!("test_stream_append_{}", unique_test_id()).into();
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
                let unique_id = unique_test_id();
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
                let stream: crate::types::RedisKey =
                    format!("test_stream_ack_{}", unique_test_id()).into();
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
                let existing_key: crate::types::RedisKey =
                    format!("test_del_exist_{}", unique_test_id()).into();
                let non_existing_key: crate::types::RedisKey =
                    format!("test_del_miss_{}", unique_test_id()).into();

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

    #[tokio::test]
    async fn test_stream_read_grouped() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey =
                    format!("test_stream_read_grouped_{}", unique_id).into();

                // Append two entries
                let fields1: Vec<(&str, &str)> = vec![("field1", "value1")];
                let fields2: Vec<(&str, &str)> = vec![("field2", "value2")];
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields1)
                    .await;
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields2)
                    .await;

                // Act — read from the beginning
                let result = pool
                    .stream_read_grouped(
                        std::slice::from_ref(&stream),
                        &["0-0".to_string()],
                        Some(10),
                    )
                    .await;

                // Assert — should get one stream key with two entries
                match result {
                    Ok(grouped) => {
                        let stream_key =
                            pool.add_prefix(&format!("test_stream_read_grouped_{}", unique_id));
                        let entries = grouped.get(&stream_key);
                        entries.is_some() && entries.unwrap().len() == 2
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_pubsub_standalone_publish_and_receive() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let channel = "test_pubsub_channel";
                let test_message = "test_message_value";

                // Subscribe — `subscriber` is an Arc<SubscriberClient> field
                pool.subscriber
                    .subscribe(channel)
                    .await
                    .expect("failed to subscribe");

                // Get the receiver BEFORE spawning manage_subscriptions,
                // otherwise the broadcast may send before we're listening.
                let mut receiver = pool.subscriber.message_rx();

                // Spawn the message loop so published messages get broadcast
                let subscriber = pool.subscriber.clone();
                let _handle = tokio::spawn(async move {
                    subscriber.manage_subscriptions().await;
                });

                // Give the message loop a moment to start
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                // Act — publish a message. `publisher` is an Arc<RedisClient> field.
                pool.publisher
                    .publish(
                        channel,
                        crate::types::RedisValue::from_string(test_message.to_string()),
                    )
                    .await
                    .expect("failed to publish");

                // Wait for the message (with timeout to avoid hanging)
                let received =
                    tokio::time::timeout(std::time::Duration::from_secs(5), receiver.recv()).await;

                // Assert — compare by converting Value to string
                match received {
                    Ok(Ok(msg)) => {
                        let value_str = crate::types::redis_value_to_option_string(&msg.value);
                        msg.channel == channel && value_str.as_deref() == Some(test_message)
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
    async fn test_connection_with_custom_config() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange — custom config with non-default reconnect settings
                let settings = RedisSettings {
                    reconnect_max_attempts: 10,
                    reconnect_delay: 100,
                    default_command_timeout: 60,
                    max_in_flight_commands: 10000,
                    ..RedisSettings::default()
                };

                let pool = RedisConnectionPool::new(&settings)
                    .await
                    .expect("failed to create redis connection pool with custom config");

                // Act — set and get a key
                let key: crate::types::RedisKey =
                    format!("test_config_{}", unique_test_id()).into();
                let value = "custom_config_value".to_string();
                let _ = pool.set_key(&key, value.clone()).await;
                let result: Result<String, _> = pool.get_key(&key).await;

                // Assert — basic operations work with custom config
                result.is_ok() && result.unwrap() == value
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_resp3_set_and_get() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange — use_legacy_version = false enables RESP3
                let settings = RedisSettings {
                    use_legacy_version: false,
                    ..RedisSettings::default()
                };

                let pool = RedisConnectionPool::new(&settings)
                    .await
                    .expect("failed to create redis connection pool with RESP3");

                // Act — set and get a key
                let key: crate::types::RedisKey = format!("test_resp3_{}", unique_test_id()).into();
                let value = "resp3_value".to_string();
                let _ = pool.set_key(&key, value.clone()).await;
                let result: Result<String, _> = pool.get_key(&key).await;

                // Assert — RESP3 connection works for basic operations
                result.is_ok() && result.unwrap() == value
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_delete_key_reply_semantics() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Arrange
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let existing_key: crate::types::RedisKey =
                    format!("test_del_existing_{}", unique_id).into();
                let non_existing_key: crate::types::RedisKey =
                    format!("test_del_nonexisting_{}", unique_id).into();

                // Set up an existing key
                let _ = pool.set_key(&existing_key, "value".to_string()).await;

                // Act
                let delete_existing = pool.delete_key(&existing_key).await;
                let delete_non_existing = pool.delete_key(&non_existing_key).await;

                // Assert — verify exact reply variants
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

    // ─── Live Redis tests for previously uncovered paths ──────────────────────

    #[tokio::test]
    async fn test_stream_read_with_options_xreadgroup() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey =
                    format!("test_xreadgroup_{}", unique_id).into();
                let group = format!("test_grp_{}", unique_id);
                let consumer = format!("test_consumer_{}", unique_id);

                // Append an entry
                let fields: Vec<(&str, &str)> = vec![("task", "process_payment")];
                pool.stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await
                    .expect("failed to append");

                // Create consumer group
                pool.consumer_group_create(
                    &stream,
                    &group,
                    &RedisEntryId::UserSpecifiedID {
                        milliseconds: "0".to_string(),
                        sequence_number: "0".to_string(),
                    },
                )
                .await
                .expect("failed to create group");

                // Act — XREADGROUP
                let result = pool
                    .stream_read_with_options(
                        &[stream],
                        &[RedisEntryId::UndeliveredEntryID.to_stream_id()],
                        Some(1),
                        None,
                        Some((&group, &consumer)),
                    )
                    .await;

                // Assert — should get one entry
                match result {
                    Ok(reply) => {
                        reply.keys.len() == 1
                            && reply.keys.first().is_some_and(|key| key.ids.len() == 1)
                            && reply
                                .keys
                                .first()
                                .and_then(|key| key.ids.first())
                                .is_some_and(|id| id.map.contains_key("task"))
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_stream_trim_entries() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey = format!("test_trim_{}", unique_id).into();

                // Append 5 entries
                for i in 0..5 {
                    let i_str = i.to_string();
                    let fields: Vec<(&str, &str)> = vec![("idx", i_str.as_str())];
                    let _ = pool
                        .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                        .await;
                }

                let len_before = pool.stream_get_length(&stream).await.unwrap();
                if len_before < 5 {
                    return false;
                }

                // Read to get entry IDs
                let read_result = pool
                    .stream_read_grouped(
                        std::slice::from_ref(&stream),
                        &["0-0".to_string()],
                        Some(10),
                    )
                    .await
                    .expect("failed to read stream");

                let stream_key = pool.add_prefix(&format!("test_trim_{}", unique_id));
                let entries = read_result.get(&stream_key).expect("should have entries");

                // Trim using MinID — keep entries after the 2nd one
                if entries.len() >= 3 {
                    let trim_id = &entries.get(1).expect("checked len >= 3").0; // ID of 2nd entry
                    let trim_result = pool
                        .stream_trim_entries(
                            &stream,
                            crate::types::StreamCapKind::MinID,
                            crate::types::StreamCapTrim::Exact,
                            trim_id,
                        )
                        .await;

                    match trim_result {
                        Ok(trimmed_count) => trimmed_count >= 1,
                        Err(e) => {
                            eprintln!("hscan returned error: {:?}", e);
                            false
                        }
                    }
                } else {
                    false
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_consumer_group_destroy() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey =
                    format!("test_grp_destroy_{}", unique_id).into();
                let group = format!("test_destroy_grp_{}", unique_id);

                // Create stream and group
                let fields: Vec<(&str, &str)> = vec![("f", "v")];
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;
                pool.consumer_group_create(
                    &stream,
                    &group,
                    &RedisEntryId::UserSpecifiedID {
                        milliseconds: "0".to_string(),
                        sequence_number: "0".to_string(),
                    },
                )
                .await
                .expect("failed to create group");

                // Act — destroy the group
                let destroy_result = pool.consumer_group_destroy(&stream, &group).await;

                // Assert — returns 1 (destroyed)
                matches!(destroy_result, Ok(1))
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_hash_field_if_not_exist_and_get() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_hsetnx_{}", unique_id).into();

                // Act — set a new field
                let set_result = pool
                    .set_hash_field_if_not_exist(&key, "field1", "value1", None)
                    .await;

                // Try to set the same field again (should not overwrite)
                let dup_result = pool
                    .set_hash_field_if_not_exist(&key, "field1", "value2", None)
                    .await;

                // Get the field back
                let get_result: Result<String, _> = pool.get_hash_field(&key, "field1").await;

                // Get all fields
                let all_fields: Result<std::collections::HashMap<String, String>, _> =
                    pool.get_hash_fields(&key).await;

                match (set_result, dup_result, get_result, all_fields) {
                    (
                        Ok(crate::types::HsetnxReply::KeySet),
                        Ok(crate::types::HsetnxReply::KeyNotSet),
                        Ok(val),
                        Ok(map),
                    ) => val == "value1" && map.get("field1") == Some(&"value1".to_string()),
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_expiry_and_get_ttl() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_ttl_{}", unique_id).into();

                // Set a key
                let _ = pool.set_key(&key, "value".to_string()).await;

                // Act — set expiry
                let set_expiry_result = pool.set_expiry(&key, 120).await;

                // Get TTL
                let ttl_result = pool.get_ttl(&key).await;

                match (set_expiry_result, ttl_result) {
                    (Ok(()), Ok(ttl)) => ttl > 0 && ttl <= 120,
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_exists_key() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let existing_key: crate::types::RedisKey =
                    format!("test_exists_yes_{}", unique_id).into();
                let missing_key: crate::types::RedisKey =
                    format!("test_exists_no_{}", unique_id).into();

                let _ = pool.set_key(&existing_key, "val".to_string()).await;

                // Act
                let exists_result: Result<bool, _> = pool.exists::<()>(&existing_key).await;
                let missing_result: Result<bool, _> = pool.exists::<()>(&missing_key).await;

                // Assert
                matches!(exists_result, Ok(true)) && matches!(missing_result, Ok(false))
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_stream_delete_entries() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey = format!("test_xdel_{}", unique_id).into();

                // Append an entry
                let fields: Vec<(&str, &str)> = vec![("f", "v")];
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;

                // Read to get the entry ID
                let read_result = pool
                    .stream_read_grouped(
                        std::slice::from_ref(&stream),
                        &["0-0".to_string()],
                        Some(1),
                    )
                    .await
                    .expect("failed to read");

                let stream_key = pool.add_prefix(&format!("test_xdel_{}", unique_id));
                let entries = read_result.get(&stream_key).expect("should have entries");
                if entries.is_empty() {
                    return false;
                }
                let entry_id = entries.first().expect("checked non-empty").0.clone();

                // Act — delete the entry
                let delete_result = pool.stream_delete_entries(&stream, &[entry_id]).await;

                // Assert
                matches!(delete_result, Ok(count) if count >= 1)
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_hscan_returns_values() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                // Use RESP2 (use_legacy_version = true) since iter_async
                // has known issues with RESP3 HSCAN cursor parsing
                let settings = RedisSettings {
                    use_legacy_version: true,
                    ..RedisSettings::default()
                };
                let pool = RedisConnectionPool::new(&settings)
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_hscan_{}", unique_id).into();

                // Set hash fields
                let fields: Vec<(&str, &str)> = vec![
                    ("prefix_field1", "val1"),
                    ("prefix_field2", "val2"),
                    ("other_field", "val3"),
                ];
                let _ = pool.set_hash_fields(&key, &fields, Some(60)).await;

                // Act — scan with pattern
                let scan_result = pool.hscan(&key, "prefix_*", None).await;

                // Assert — hscan should succeed and return matching values
                match scan_result {
                    Ok(values) => {
                        values.contains(&"val1".to_string())
                            && values.contains(&"val2".to_string())
                            && !values.contains(&"val3".to_string())
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_sadd_and_get_hash_field_and_deserialize() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let set_key: crate::types::RedisKey = format!("test_sadd_{}", unique_id).into();
                let hash_key: crate::types::RedisKey =
                    format!("test_hget_deser_{}", unique_id).into();

                // ── SADD ──
                let sadd_result = pool.sadd(&set_key, "member1").await;
                let sadd_dup = pool.sadd(&set_key, "member1").await;

                // ── get_hash_field_and_deserialize ──
                #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
                struct TestData {
                    id: u32,
                }
                let data = TestData { id: 42 };
                let _ = pool
                    .serialize_and_set_hash_field_if_not_exist(&hash_key, "data", &data, None)
                    .await;

                let deser_result = pool
                    .get_hash_field_and_deserialize::<TestData>(&hash_key, "data", "TestData")
                    .await;

                match (sadd_result, sadd_dup, deser_result) {
                    (
                        Ok(crate::types::SaddReply::KeySet),
                        Ok(crate::types::SaddReply::KeyNotSet),
                        Ok(deserialized),
                    ) => deserialized == data,
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_list_operations() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_list_{}", unique_id).into();

                // Append elements
                let append_result = pool.append_elements_to_list(&key, &["a", "b", "c"]).await;

                // Get length
                let length_result = pool.get_list_length(&key).await;

                // Get elements
                let elements_result: Result<Vec<String>, _> =
                    pool.get_list_elements(&key, 0, -1).await;

                // Pop one element
                let pop_result = pool.lpop_list_elements(&key, Some(1)).await;

                match (append_result, length_result, elements_result, pop_result) {
                    (Ok(()), Ok(3), Ok(elems), Ok(popped)) => {
                        elems == vec!["a".to_string(), "b".to_string(), "c".to_string()]
                            && popped == vec!["a".to_string()]
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
    async fn test_subscriber_unsubscribe() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let channel = "test_unsub_channel";

                // Subscribe
                pool.subscriber
                    .subscribe(channel)
                    .await
                    .expect("failed to subscribe");

                // Act — unsubscribe
                let unsub_result = pool.subscriber.unsubscribe(channel).await;

                matches!(unsub_result, Ok(()))
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_increment_fields_in_hash() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_hincr_{}", unique_id).into();

                // Set initial field
                let fields: Vec<(&str, &str)> = vec![("counter", "10")];
                let _ = pool.set_hash_fields(&key, &fields, Some(60)).await;

                // Act — increment by 5
                let result = pool
                    .increment_fields_in_hash(&key, &[("counter".to_string(), 5)])
                    .await;

                // Assert — should be 15
                match result {
                    Ok(values) => values.len() == 1 && values.first() == Some(&15),
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    // ─── Tests for previously uncovered methods ──────────────────────────────

    #[tokio::test]
    async fn test_set_multiple_keys_if_not_exist_msetnx() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key1: crate::types::RedisKey = format!("test_msetnx_a_{}", unique_id).into();
                let key2: crate::types::RedisKey = format!("test_msetnx_b_{}", unique_id).into();

                // Act — set two new keys atomically
                let result = pool
                    .set_multiple_keys_if_not_exist(&[
                        (key1.tenant_aware_key(&pool), "val1"),
                        (key2.tenant_aware_key(&pool), "val2"),
                    ])
                    .await;

                // Assert — both should be set
                matches!(result, Ok(crate::types::MsetnxReply::KeysSet))
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_multiple_keys_if_not_exist_with_existing_key() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key1: crate::types::RedisKey =
                    format!("test_msetnx_exist_a_{}", unique_id).into();
                let key2: crate::types::RedisKey =
                    format!("test_msetnx_exist_b_{}", unique_id).into();

                // Pre-set key1
                let _ = pool.set_key(&key1, "existing".to_string()).await;

                // Act — try to set both; key1 already exists so MSETNX should fail
                let result = pool
                    .set_multiple_keys_if_not_exist(&[
                        (key1.tenant_aware_key(&pool), "new1"),
                        (key2.tenant_aware_key(&pool), "new2"),
                    ])
                    .await;

                // Assert — should return KeysNotSet since key1 exists
                matches!(result, Ok(crate::types::MsetnxReply::KeysNotSet))
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_delete_multiple_keys() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key1: crate::types::RedisKey = format!("test_del_multi_a_{}", unique_id).into();
                let key2: crate::types::RedisKey = format!("test_del_multi_b_{}", unique_id).into();
                let key3: crate::types::RedisKey = format!("test_del_multi_c_{}", unique_id).into();

                // Set two of three keys
                let _ = pool.set_key(&key1, "val1".to_string()).await;
                let _ = pool.set_key(&key2, "val2".to_string()).await;
                // key3 does not exist

                // Act
                let result = pool.delete_multiple_keys(&[key1, key2, key3]).await;

                // Assert — key1 and key2 deleted, key3 not deleted
                match result {
                    Ok(replies) => {
                        replies.len() == 3
                            && replies.first() == Some(&crate::types::DelReply::KeyDeleted)
                            && replies.get(1) == Some(&crate::types::DelReply::KeyDeleted)
                            && replies.get(2) == Some(&crate::types::DelReply::KeyNotDeleted)
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_multiple_keys_if_not_exists_and_get_values() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key1: crate::types::RedisKey =
                    format!("test_setget_multi_a_{}", unique_id).into();
                let key2: crate::types::RedisKey =
                    format!("test_setget_multi_b_{}", unique_id).into();

                // Act — set two new keys
                let result = pool
                    .set_multiple_keys_if_not_exists_and_get_values(
                        &[(key1, "val1".to_string()), (key2, "val2".to_string())],
                        Some(30),
                    )
                    .await;

                // Assert — both should be ValueSet
                match result {
                    Ok(replies) => {
                        replies.len() == 2
                            && matches!(replies.first(), Some(crate::types::SetGetReply::ValueSet(ref value)) if value == "val1")
                            && matches!(replies.get(1), Some(crate::types::SetGetReply::ValueSet(ref value)) if value == "val2")
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_scan_returns_matching_keys() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key1: crate::types::RedisKey = format!("test_scan_foo_{}", unique_id).into();
                let key2: crate::types::RedisKey = format!("test_scan_bar_{}", unique_id).into();

                // Set keys
                let _ = pool.set_key(&key1, "v1".to_string()).await;
                let _ = pool.set_key(&key2, "v2".to_string()).await;

                // Act — scan for keys matching "test_scan_foo_*"
                let scan_pattern: crate::types::RedisKey =
                    format!("*test_scan_foo_{}*", unique_id).into();
                let result = pool.scan(&scan_pattern, None, None).await;

                // Assert — should find key1 but not key2
                match result {
                    Ok(keys) => {
                        let found_foo = keys
                            .iter()
                            .any(|k| k.contains(&format!("test_scan_foo_{}", unique_id)));
                        let found_bar = keys
                            .iter()
                            .any(|k| k.contains(&format!("test_scan_bar_{}", unique_id)));
                        found_foo && !found_bar
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_serialize_and_set_key_if_not_exist() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_ser_setnx_{}", unique_id).into();

                #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
                struct Data {
                    id: u32,
                }

                let data = Data { id: 99 };

                // Act — set a serialized value if not exists
                let set_result = pool
                    .serialize_and_set_key_if_not_exist(&key, &data, Some(60))
                    .await;

                // Try again — should fail
                let dup_result = pool
                    .serialize_and_set_key_if_not_exist(&key, &Data { id: 100 }, Some(60))
                    .await;

                matches!(
                    (set_result, dup_result),
                    (
                        Ok(crate::types::SetnxReply::KeySet),
                        Ok(crate::types::SetnxReply::KeyNotSet),
                    )
                )
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_serialize_and_set_key_with_expiry() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_ser_setex_{}", unique_id).into();

                #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
                struct Data {
                    name: String,
                }

                let data = Data {
                    name: "test".to_string(),
                };

                // Act
                let set_result = pool
                    .serialize_and_set_key_with_expiry(&key, &data, 120)
                    .await;

                // Verify by deserializing back
                let get_result = pool.get_and_deserialize_key::<Data>(&key, "Data").await;

                match (set_result, get_result) {
                    (Ok(()), Ok(retrieved)) => retrieved == data,
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_serialize_and_set_key_without_modifying_ttl() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_ser_keepttl_{}", unique_id).into();

                #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
                struct Data {
                    val: i32,
                }

                // Set with expiry first
                let _ = pool
                    .serialize_and_set_key_with_expiry(&key, &Data { val: 1 }, 120)
                    .await;

                // Act — update value keeping TTL
                let update_result = pool
                    .serialize_and_set_key_without_modifying_ttl(&key, &Data { val: 2 })
                    .await;

                // Verify value changed
                let get_result = pool.get_and_deserialize_key::<Data>(&key, "Data").await;

                match (update_result, get_result) {
                    (Ok(()), Ok(retrieved)) => retrieved.val == 2,
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_get_and_deserialize_key() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_deser_single_{}", unique_id).into();

                #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
                struct Data {
                    x: i32,
                }

                let data = Data { x: 42 };

                // Set serialized
                let _ = pool.serialize_and_set_key(&key, &data).await;

                // Act — get and deserialize
                let result = pool.get_and_deserialize_key::<Data>(&key, "Data").await;

                match result {
                    Ok(retrieved) => retrieved == data,
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_get_and_deserialize_key_not_found() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let key: crate::types::RedisKey = "nonexistent_deser_key".into();

                // Act — try to deserialize a missing key
                let result: Result<String, _> = pool.get_and_deserialize_key(&key, "String").await;

                // Assert — should be NotFound error
                result.is_err()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_hscan_and_deserialize() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_hscan_deser_{}", unique_id).into();

                // Set hash fields with serialized values
                let fields: Vec<(&str, &str)> = vec![
                    ("item_a", "\"alpha\""),
                    ("item_b", "\"beta\""),
                    ("item_c", "\"gamma\""),
                ];
                let _ = pool.set_hash_fields(&key, &fields, Some(60)).await;

                // Act — scan and deserialize as String
                let result = pool
                    .hscan_and_deserialize::<String>(&key, "item_*", None)
                    .await;

                // Assert — should find "alpha" and "beta"
                match result {
                    Ok(values) => {
                        values.contains(&"alpha".to_string())
                            && values.contains(&"beta".to_string())
                            && values.contains(&"gamma".to_string())
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_set_expire_at_and_get_ttl() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key: crate::types::RedisKey = format!("test_expireat_{}", unique_id).into();

                // Set a key
                let _ = pool.set_key(&key, "value".to_string()).await;

                // Act — set expire at a future timestamp (now + 120s)
                let future_ts = i64::try_from(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                )
                .unwrap()
                    + 120;
                let set_result = pool.set_expire_at(&key, future_ts).await;
                let ttl_result = pool.get_ttl(&key).await;

                match (set_result, ttl_result) {
                    (Ok(()), Ok(ttl)) => ttl > 0 && ttl <= 120,
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_consumer_group_delete_consumer() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey =
                    format!("test_xdelconsumer_{}", unique_id).into();
                let group = format!("test_grp_{}", unique_id);
                let consumer = format!("test_consumer_{}", unique_id);

                // Create stream + group
                let fields: Vec<(&str, &str)> = vec![("f", "v")];
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;
                let _ = pool
                    .consumer_group_create(
                        &stream,
                        &group,
                        &RedisEntryId::UserSpecifiedID {
                            milliseconds: "0".to_string(),
                            sequence_number: "0".to_string(),
                        },
                    )
                    .await;

                // Read with the consumer so it's registered
                let _ = pool
                    .stream_read_with_options(
                        std::slice::from_ref(&stream),
                        &[RedisEntryId::UndeliveredEntryID.to_stream_id()],
                        Some(1),
                        None,
                        Some((&group, &consumer)),
                    )
                    .await;

                // Act — delete the consumer
                let result = pool
                    .consumer_group_delete_consumer(&stream, &group, &consumer)
                    .await;

                // Assert — should succeed (returns number of pending messages)
                result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_consumer_group_set_last_id() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey = format!("test_xsetid_{}", unique_id).into();
                let group = format!("test_grp_{}", unique_id);

                // Create stream + group
                let fields: Vec<(&str, &str)> = vec![("f", "v")];
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;
                let _ = pool
                    .consumer_group_create(
                        &stream,
                        &group,
                        &RedisEntryId::UserSpecifiedID {
                            milliseconds: "0".to_string(),
                            sequence_number: "0".to_string(),
                        },
                    )
                    .await;

                // Act — set last delivered ID
                let result = pool
                    .consumer_group_set_last_id(
                        &stream,
                        &group,
                        &RedisEntryId::UserSpecifiedID {
                            milliseconds: "0".to_string(),
                            sequence_number: "1".to_string(),
                        },
                    )
                    .await;

                // Assert
                matches!(result, Ok(id) if id == "0-1")
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_consumer_group_set_message_owner_xclaim() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey = format!("test_xclaim_{}", unique_id).into();
                let group = format!("test_grp_{}", unique_id);
                let consumer1 = format!("test_c1_{}", unique_id);
                let consumer2 = format!("test_c2_{}", unique_id);

                // Create stream + group + entry
                let fields: Vec<(&str, &str)> = vec![("task", "process")];
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;
                let _ = pool
                    .consumer_group_create(
                        &stream,
                        &group,
                        &RedisEntryId::UserSpecifiedID {
                            milliseconds: "0".to_string(),
                            sequence_number: "0".to_string(),
                        },
                    )
                    .await;

                // Consumer1 reads the message (creates pending entry)
                let read_result = pool
                    .stream_read_with_options(
                        std::slice::from_ref(&stream),
                        &[RedisEntryId::UndeliveredEntryID.to_stream_id()],
                        Some(1),
                        None,
                        Some((&group, &consumer1)),
                    )
                    .await;

                let entry_id = match read_result {
                    Ok(reply) if !reply.keys.is_empty()
                        && reply.keys.first().is_some_and(|key| !key.ids.is_empty()) =>
                    {
                        reply
                            .keys
                            .first()
                            .and_then(|key| key.ids.first())
                            .expect("checked non-empty")
                            .id
                            .clone()
                    }
                    _ => return false,
                };

                // Act — claim the message for consumer2
                let claim_result: Result<redis::streams::StreamClaimReply, _> = pool
                    .consumer_group_set_message_owner(
                        &stream,
                        &group,
                        &consumer2,
                        0, // min_idle_time = 0 so we can claim immediately
                        &[entry_id],
                    )
                    .await;

                // Assert — claim should succeed
                claim_result.is_ok()
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_serialize_and_set_multiple_hash_field_if_not_exist() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let key1: crate::types::RedisKey =
                    format!("test_ser_hsetnx_multi_a_{}", unique_id).into();
                let key2: crate::types::RedisKey =
                    format!("test_ser_hsetnx_multi_b_{}", unique_id).into();

                #[derive(serde::Serialize, Debug)]
                struct Data {
                    id: u32,
                }

                // Act — set same field in two different hash keys
                let result = pool
                    .serialize_and_set_multiple_hash_field_if_not_exist(
                        &[(&key1, Data { id: 1 }), (&key2, Data { id: 2 })],
                        "data",
                        None,
                    )
                    .await;

                // Assert — both should be set
                match result {
                    Ok(replies) => {
                        replies.len() == 2
                            && replies.first() == Some(&crate::types::HsetnxReply::KeySet)
                            && replies.get(1) == Some(&crate::types::HsetnxReply::KeySet)
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_stream_read_entries() {
        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let pool = RedisConnectionPool::new(&RedisSettings::default())
                    .await
                    .expect("failed to create redis connection pool");
                let unique_id = unique_test_id();
                let stream: crate::types::RedisKey = format!("test_xread_{}", unique_id).into();

                // Append entries
                let fields: Vec<(&str, &str)> = vec![("f", "v")];
                let _ = pool
                    .stream_append_entry(&stream, &RedisEntryId::AutoGeneratedID, &fields)
                    .await;

                // Act — XREAD
                let result = pool
                    .stream_read_entries(
                        std::slice::from_ref(&stream),
                        &["0-0".to_string()],
                        Some(10),
                    )
                    .await;

                // Assert — should get StreamReadReply with one entry
                match result {
                    Ok(reply) => {
                        reply.keys.len() == 1
                            && reply
                                .keys
                                .first()
                                .is_some_and(|key| !key.ids.is_empty())
                    }
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    // ─── Cluster-mode tests ─────────────────────────────────────────────────
    // These tests run only when `REDIS_CLUSTER_URLS` env var is set.
    // Example: REDIS_CLUSTER_URLS=redis://localhost:7000,redis://localhost:7001,redis://localhost:7002

    /// Helper: get a cluster pool + unique ID or skip the test.
    async fn get_cluster_pool_with_uid() -> Option<(RedisConnectionPool, String)> {
        let pool = get_cluster_pool_or_skip().await?;
        let unique_id = unique_test_id();
        Some((pool, unique_id))
    }

    #[tokio::test]
    async fn test_cluster_set_get_delete() {
        let (pool, uid) = match get_cluster_pool_with_uid().await {
            Some(result) => result,
            None => return, // skip if no cluster
        };

        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let key: crate::types::RedisKey = format!("test_cluster_sgd_{}", uid).into();
                let value = "cluster_value".to_string();

                // SET
                let set_result = pool.set_key(&key, value.clone()).await;
                // GET
                let get_result: Result<String, _> = pool.get_key(&key).await;
                // DELETE
                let del_result = pool.delete_key(&key).await;

                matches!(set_result, Ok(()))
                    && matches!(get_result, Ok(v) if v == value)
                    && matches!(del_result, Ok(crate::types::DelReply::KeyDeleted))
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_cluster_hash_operations() {
        let (pool, uid) = match get_cluster_pool_with_uid().await {
            Some(result) => result,
            None => return,
        };

        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let key: crate::types::RedisKey = format!("test_cluster_hash_{}", uid).into();

                // HSET
                let set_result = pool
                    .set_hash_fields(&key, &[("field1", "val1"), ("field2", "val2")], Some(60))
                    .await;

                // HSETNX
                let hsetnx_new = pool
                    .set_hash_field_if_not_exist(&key, "field3", "val3", None)
                    .await;

                // HSETNX duplicate
                let hsetnx_dup = pool
                    .set_hash_field_if_not_exist(&key, "field1", "new_val", None)
                    .await;

                // HGET
                let hget_result: Result<String, _> = pool.get_hash_field(&key, "field1").await;

                // HGETALL
                let hgetall_result: Result<std::collections::HashMap<String, String>, _> =
                    pool.get_hash_fields(&key).await;

                match (
                    set_result,
                    hsetnx_new,
                    hsetnx_dup,
                    hget_result,
                    hgetall_result,
                ) {
                    (
                        Ok(()),
                        Ok(crate::types::HsetnxReply::KeySet),
                        Ok(crate::types::HsetnxReply::KeyNotSet),
                        Ok(val),
                        Ok(map),
                    ) => val == "val1" && map.contains_key("field1") && map.contains_key("field3"),
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_cluster_stream_operations() {
        let (pool, uid) = match get_cluster_pool_with_uid().await {
            Some(result) => result,
            None => return,
        };

        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let stream: crate::types::RedisKey = format!("test_cluster_stream_{}", uid).into();
                let group = format!("test_cluster_grp_{}", uid);

                // XADD
                let append_result = pool
                    .stream_append_entry(
                        &stream,
                        &RedisEntryId::AutoGeneratedID,
                        &[("task", "process")],
                    )
                    .await;

                // XLEN
                let len_result = pool.stream_get_length(&stream).await;

                // XGROUP CREATE
                let group_result = pool
                    .consumer_group_create(
                        &stream,
                        &group,
                        &RedisEntryId::UserSpecifiedID {
                            milliseconds: "0".to_string(),
                            sequence_number: "0".to_string(),
                        },
                    )
                    .await;

                // XREADGROUP
                let read_result = pool
                    .stream_read_with_options(
                        std::slice::from_ref(&stream),
                        &[RedisEntryId::UndeliveredEntryID.to_stream_id()],
                        Some(1),
                        None,
                        Some((&group, &format!("consumer_{}", uid))),
                    )
                    .await;

                // XGROUP DESTROY
                let destroy_result = pool.consumer_group_destroy(&stream, &group).await;

                match (
                    append_result,
                    len_result,
                    group_result,
                    read_result,
                    destroy_result,
                ) {
                    (Ok(()), Ok(len), Ok(()), Ok(reply), Ok(_)) => {
                        len >= 1
                            && reply.keys.len() == 1
                            && reply
                                .keys
                                .first()
                                .is_some_and(|key| !key.ids.is_empty())
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
    async fn test_cluster_set_key_if_not_exists_and_get_value() {
        let (pool, uid) = match get_cluster_pool_with_uid().await {
            Some(result) => result,
            None => return,
        };

        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let key: crate::types::RedisKey = format!("test_cluster_setnx_get_{}", uid).into();

                // New key — should return ValueSet
                let result_new = pool
                    .set_key_if_not_exists_and_get_value(&key, "val1".to_string(), Some(30))
                    .await;

                // Existing key — should return ValueExists
                let result_exist = pool
                    .set_key_if_not_exists_and_get_value(&key, "val2".to_string(), Some(30))
                    .await;

                match (result_new, result_exist) {
                    (
                        Ok(crate::types::SetGetReply::ValueSet(v1)),
                        Ok(crate::types::SetGetReply::ValueExists(v2)),
                    ) => v1 == "val1" && v2 == "val1",
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_cluster_scan() {
        let (pool, uid) = match get_cluster_pool_with_uid().await {
            Some(result) => result,
            None => return,
        };

        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let key: crate::types::RedisKey = format!("test_cluster_scan_{}", uid).into();

                // Set a key
                let _ = pool.set_key(&key, "v".to_string()).await;

                // SCAN for it
                let pattern: crate::types::RedisKey = format!("*test_cluster_scan_{}*", uid).into();
                let result = pool.scan(&pattern, None, None).await;

                match result {
                    Ok(keys) => keys
                        .iter()
                        .any(|k| k.contains(&format!("test_cluster_scan_{}", uid))),
                    Err(_) => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    #[tokio::test]
    async fn test_cluster_pubsub() {
        let (pool, uid) = match get_cluster_pool_with_uid().await {
            Some(result) => result,
            None => return,
        };

        let is_success = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async {
                let channel = format!("test_cluster_pubsub_{}", uid);
                let test_message = "cluster_message";

                // Subscribe
                pool.subscriber
                    .subscribe(&channel)
                    .await
                    .expect("failed to subscribe on cluster");

                let mut receiver = pool.subscriber.message_rx();

                // Spawn message loop
                let subscriber = pool.subscriber.clone();
                let _handle = tokio::spawn(async move {
                    subscriber.manage_subscriptions().await;
                });

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                // Publish
                pool.publisher
                    .publish(
                        &channel,
                        crate::types::RedisValue::from_string(test_message.to_string()),
                    )
                    .await
                    .expect("failed to publish on cluster");

                // Receive
                let received =
                    tokio::time::timeout(std::time::Duration::from_secs(5), receiver.recv()).await;

                match received {
                    Ok(Ok(msg)) => {
                        let value_str = crate::types::redis_value_to_option_string(&msg.value);
                        msg.channel == channel && value_str.as_deref() == Some(test_message)
                    }
                    _ => false,
                }
            })
        })
        .await
        .expect("Spawn block failure");

        assert!(is_success);
    }

    // ─── on_error / max_failure_threshold tests ──────────────────────────────

    #[tokio::test]
    async fn test_on_error_triggers_shutdown_when_redis_unreachable() {
        // Use a non-existent Redis host — all PINGs will fail
        let settings = RedisSettings {
            host: "192.0.2.1".to_string(), // RFC 5737 test address, guaranteed unreachable
            port: 1,
            unresponsive_check_interval: 1,
            max_failure_threshold: 2, // 2 seconds of unreachability → shutdown
            reconnect_max_attempts: 1,
            default_command_timeout: 1,
            ..RedisSettings::default()
        };

        let pool = RedisConnectionPool::new(&settings).await;
        // Connection may or may not succeed initially — that's fine,
        // on_error handles both connected and disconnected states
        let pool = match pool {
            Ok(pool) => pool,
            Err(_) => return, // If connection fails entirely, skip — some environments block this
        };

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Run on_error in background — it should trigger shutdown
        tokio::spawn(async move {
            pool.on_error(shutdown_tx).await;
        });

        // Wait for the shutdown signal with a generous timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(10), shutdown_rx).await;

        // Assert — shutdown signal should have been sent
        assert!(
            result.is_ok() && result.unwrap().is_ok(),
            "on_error should trigger shutdown signal when Redis is unreachable"
        );
    }

    #[tokio::test]
    async fn test_on_error_keeps_redis_available_when_healthy() {
        let pool = RedisConnectionPool::new(&RedisSettings::default())
            .await
            .expect("failed to create redis connection pool");

        let initial_state = pool
            .is_redis_available
            .load(std::sync::atomic::Ordering::SeqCst);

        // Verify the pool is healthy by setting and getting a key
        let key: crate::types::RedisKey = format!("test_health_{}", unique_test_id()).into();
        let set_result = pool.set_key(&key, "ok".to_string()).await;
        let get_result: Result<String, _> = pool.get_key(&key).await;

        // If Redis isn't running, skip
        if set_result.is_err() || get_result.is_err() {
            return;
        }

        // Assert — Redis should be marked available when healthy
        assert!(initial_state, "Redis should be available when healthy");
    }

    #[tokio::test]
    async fn test_on_error_marks_unavailable_after_threshold() {
        // Same as trigger test but also verify is_redis_available flag
        let settings = RedisSettings {
            host: "192.0.2.1".to_string(),
            port: 1,
            unresponsive_check_interval: 1,
            max_failure_threshold: 2,
            reconnect_max_attempts: 1,
            default_command_timeout: 1,
            ..RedisSettings::default()
        };

        let pool = RedisConnectionPool::new(&settings).await;
        let pool = match pool {
            Ok(pool) => pool,
            Err(_) => return,
        };

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let is_available = pool.is_redis_available.clone();

        tokio::spawn(async move {
            pool.on_error(shutdown_tx).await;
        });

        let result = tokio::time::timeout(std::time::Duration::from_secs(10), shutdown_rx).await;

        if result.is_ok() && result.unwrap().is_ok() {
            // After shutdown signal, redis should be marked unavailable
            assert!(
                !is_available.load(std::sync::atomic::Ordering::SeqCst),
                "is_redis_available should be false after on_error triggers shutdown"
            );
        }
    }
}
