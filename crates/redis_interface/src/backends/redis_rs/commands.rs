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
use tracing::instrument;

use super::types::redis_value_to_option_string;
use crate::{
    constant::redis_rs_commands::{
        REDIS_ARG_COUNT, REDIS_ARG_EX, REDIS_ARG_MATCH, REDIS_ARG_NX, REDIS_ARG_TYPE,
        REDIS_COMMAND_GET, REDIS_COMMAND_HSCAN, REDIS_COMMAND_SCAN, REDIS_COMMAND_SET,
    },
    errors,
    types::{
        DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisKey, SaddReply, SetGetReply,
        SetnxReply, StreamCapKind, StreamCapTrim, StreamEntries, StreamReadResult,
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
            let reply: (u64, Vec<redis::Value>) = command
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
    pub async fn stream_append_entry<F, V>(
        &self,
        stream: &RedisKey,
        entry_id: &RedisEntryId,
        fields: &[(F, V)],
    ) -> CustomResult<(), errors::RedisError>
    where
        F: Into<String> + Clone + Debug + Send + Sync,
        V: Into<String> + Clone + Debug + Send + Sync,
    {
        let pairs: Vec<(String, String)> = fields
            .iter()
            .map(|(f, v)| (f.clone().into(), v.clone().into()))
            .collect();

        let mut conn = self.pool.clone();
        let _: Option<String> = conn
            .xadd_map(
                stream.tenant_aware_key(self),
                entry_id.to_stream_id(),
                &pairs,
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
    /// Returns a [`StreamReadResult`] (stream key → list of `(entry_id, fields)`),
    /// so callers never need to parse backend-specific reply types.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_entries(
        &self,
        streams: &[RedisKey],
        ids: &[String],
        read_count: Option<u64>,
    ) -> CustomResult<StreamReadResult, errors::RedisError> {
        let mut conn = self.pool.clone();
        let stream_keys: Vec<String> = streams
            .iter()
            .map(|stream| stream.tenant_aware_key(self))
            .collect();

        let count = read_count.unwrap_or(self.config.default_stream_read_count);

        let options = StreamReadOptions::default()
            .count(usize::try_from(count).change_context(errors::RedisError::StreamReadFailed)?);

        let reply: redis::streams::StreamReadReply = conn
            .xread_options(&stream_keys, ids, &options)
            .await
            .map_err(|err| {
                let kind = err.kind();
                match kind {
                    redis::ErrorKind::UnexpectedReturnType | redis::ErrorKind::Parse => {
                        report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
                    }
                    _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
                }
            })?;

        Ok(reply
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
            .collect())
    }

    /// Read stream entries and return them grouped by stream key with optional field values.
    /// Read entries from streams with options (XREAD / XREADGROUP)
    /// Returns a [`StreamReadResult`] (stream key → list of `(entry_id, fields)`),
    /// so callers never need to parse backend-specific reply types.
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn stream_read_with_options(
        &self,
        streams: &[RedisKey],
        ids: &[String],
        count: Option<u64>,
        block: Option<u64>,
        group: Option<(&str, &str)>,
    ) -> CustomResult<StreamReadResult, errors::RedisError> {
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

        let reply: redis::streams::StreamReadReply = conn
            .xread_options(&stream_keys, ids, &options)
            .await
            .map_err(|err| {
                let kind = err.kind();
                match kind {
                    redis::ErrorKind::UnexpectedReturnType | redis::ErrorKind::Parse => {
                        report!(err).change_context(errors::RedisError::StreamEmptyOrNotAvailable)
                    }
                    _ => report!(err).change_context(errors::RedisError::StreamReadFailed),
                }
            })?;

        Ok(reply
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
            .collect())
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
        let results: Vec<redis::Value> = pipe
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
