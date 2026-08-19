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
// Deja: raw redis replies are captured as `deja::value::RedisWireValue`, the
// one canonical serde-native wire type; the client conversions live in the
// deja crate next to the type (feature `fred` on the deja dependency).
#[cfg(feature = "deja")]
use deja::value::RedisWireValue;
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
    metrics::{track_redis_call, RedisOperation},
    types::{
        DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisKey, SaddReply, SetGetReply,
        SetnxReply, StreamEntries, StreamReadResult, StreamTrimConfig,
    },
};

impl super::RedisConnectionWithContext {
    /// Prefix `key` with the tenant key prefix of the underlying pool.
    pub fn add_prefix(&self, key: &str) -> String {
        self.redis_conn.add_prefix(key)
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_key",
            codec = deja::codec::ResultCodec::<(), errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "SET",
                    "has_ttl": true,
                })
            },
        )
    )]
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_key<V>(&self, key: &RedisKey, value: V) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetKey,
            self.redis_conn.pool.set(
                key.tenant_aware_key(&self.redis_conn),
                value,
                Some(Expiration::EX(self.redis_conn.config.default_ttl.into())),
                None,
                false,
            ),
        )
        .await
        .change_context(errors::RedisError::SetFailed)
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_key_without_modifying_ttl",
            codec = deja::codec::ResultCodec::<(), errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "SET_KEEP_TTL",
                })
            },
        )
    )]
    pub async fn set_key_without_modifying_ttl<V>(
        &self,
        key: &RedisKey,
        value: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<RedisValue> + Debug + Send + Sync,
        V::Error: Into<fred::error::RedisError> + Send + Sync,
    {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetKeyWithoutModifyingTtl,
            self.redis_conn.pool.set(
                key.tenant_aware_key(&self.redis_conn),
                value,
                Some(Expiration::KEEPTTL),
                None,
                false,
            ),
        )
        .await
        .change_context(errors::RedisError::SetFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_multiple_keys_if_not_exist",
            args = {
                serde_json::json!({
                    "command": "MSETNX",
                })
            },
        )
    )]
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

        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetMultipleKeysIfNotExist,
            self.redis_conn.pool.msetnx(map),
        )
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

        #[cfg(feature = "deja")]
        {
            self.set_key_with_expiry(key, serialized.as_slice(), seconds)
                .await
        }

        #[cfg(not(feature = "deja"))]
        {
            track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                RedisOperation::SerializeAndSetKeyWithExpiry,
                self.redis_conn.pool.set(
                    key.tenant_aware_key(&self.redis_conn),
                    serialized.as_slice(),
                    Some(Expiration::EX(seconds)),
                    None,
                    false,
                ),
            )
            .await
            .change_context(errors::RedisError::SetExFailed)
        }
    }

    // Deja hermetic boundary for GET.
    //
    // The boundary lives on this inner method, which fetches the RAW redis reply
    // (`fred::types::RedisValue`) and converts it into the serde-native
    // `RedisWireValue`. Because the recorded/replayed type is concrete and
    // serde-native, the typed `ResultCodec` works without leaking a serde bound
    // onto the public `get_key<V>`.
    #[cfg(feature = "deja")]
    #[instrument(level = "DEBUG", skip(self))]
    #[deja::redis(
        operation = "get_key",
        codec = deja::codec::ResultCodec::<RedisWireValue, errors::RedisError>,
        state_read = key.tenant_aware_key(&self.redis_conn),
        args = {
            serde_json::json!({
                "key": key.as_str(),
                "command": "GET",
            })
        },
    )]
    async fn get_key_raw(
        &self,
        key: &RedisKey,
    ) -> CustomResult<RedisWireValue, errors::RedisError> {
        match track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::GetKey,
            self.redis_conn
                .pool
                .get::<RedisValue, _>(key.tenant_aware_key(&self.redis_conn)),
        )
        .await
        .change_context(errors::RedisError::GetFailed)
        {
            Ok(v) => Ok(v.into()),
            Err(_err) => {
                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }

                #[cfg(feature = "multitenancy_fallback")]
                {
                    track_redis_call(
                        self.request_id.as_deref(),
                        self.redis_conn.event_emitter.as_ref(),
                        RedisOperation::GetKey,
                        self.redis_conn
                            .pool
                            .get::<RedisValue, _>(key.tenant_unaware_key(&self.redis_conn)),
                    )
                    .await
                    .change_context(errors::RedisError::GetFailed)
                    .map(RedisWireValue::from)
                }
            }
        }
    }

    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_key<V>(&self, key: &RedisKey) -> CustomResult<V, errors::RedisError>
    where
        V: FromRedis + Unpin + Send + 'static,
    {
        #[cfg(feature = "deja")]
        {
            let raw = self.get_key_raw(key).await?;
            let value: RedisValue = raw.try_into().map_err(|err: fred::error::RedisError| {
                report!(err).change_context(errors::RedisError::GetFailed)
            })?;
            V::from_value(value).change_context(errors::RedisError::GetFailed)
        }

        #[cfg(not(feature = "deja"))]
        {
            match track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                RedisOperation::GetKey,
                self.redis_conn
                    .pool
                    .get(key.tenant_aware_key(&self.redis_conn)),
            )
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
                        track_redis_call(
                            self.request_id.as_deref(),
                            self.redis_conn.event_emitter.as_ref(),
                            RedisOperation::GetKey,
                            self.redis_conn
                                .pool
                                .get(key.tenant_unaware_key(&self.redis_conn)),
                        )
                        .await
                        .change_context(errors::RedisError::GetFailed)
                    }
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

        let tenant_aware_keys: Vec<String> = keys
            .iter()
            .map(|key| key.tenant_aware_key(&self.redis_conn))
            .collect();
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::GetMultipleKeys,
            self.redis_conn.pool.mget(tenant_aware_keys),
        )
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
        let tenant_aware_keys: Vec<String> = keys
            .iter()
            .map(|key| key.tenant_aware_key(&self.redis_conn))
            .collect();

        let futures = tenant_aware_keys.iter().map(|redis_key| {
            track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                RedisOperation::GetKey,
                self.redis_conn.pool.get::<Option<V>, _>(redis_key),
            )
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
        V: FromRedis + Unpin + Send + 'static,
    {
        if self.redis_conn.config.cluster_enabled {
            // Use individual GET commands for cluster mode to avoid CROSSSLOT errors
            self.get_multiple_keys_with_parallel_get(keys).await
        } else {
            // Use MGET for non-cluster mode for better performance
            self.get_multiple_keys_with_mget(keys).await
        }
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "get_multiple_keys",
            read_set = keys.iter().map(|key| key.tenant_aware_key(&self.redis_conn)).collect::<Vec<_>>(),
            args = {
                serde_json::json!({
                    "key_count": keys.len(),
                    "keys": keys.iter().map(|key| key.as_str()).collect::<Vec<_>>(),
                    "command": "MGET",
                })
            },
            result = {
                (
                    match &__deja_result {
                        Ok(_) => serde_json::json!({"ok": true}),
                        Err(e) => serde_json::json!({"ok": false, "error": format!("{:?}", e)}),
                    },
                    __deja_result.is_err(),
                )
            },
        )
    )]
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
                        .map(|key| key.tenant_unaware_key(&self.redis_conn).into())
                        .collect();

                    self.get_keys_by_mode(&tenant_unaware_keys).await
                }
            }
        }
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "exists",
            codec = deja::codec::ResultCodec::<bool, errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "EXISTS",
                })
            },
        )
    )]
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn exists<V>(&self, key: &RedisKey) -> CustomResult<bool, errors::RedisError>
    where
        V: Into<MultipleKeys> + Unpin + Send + 'static,
    {
        match track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::Exists,
            self.redis_conn
                .pool
                .exists(key.tenant_aware_key(&self.redis_conn)),
        )
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
                    track_redis_call(
                        self.request_id.as_deref(),
                        self.redis_conn.event_emitter.as_ref(),
                        RedisOperation::Exists,
                        self.redis_conn
                            .pool
                            .exists(key.tenant_unaware_key(&self.redis_conn)),
                    )
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

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "delete_key",
            codec = deja::codec::ResultCodec::<DelReply, errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "DEL",
                })
            },
        )
    )]
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn delete_key(&self, key: &RedisKey) -> CustomResult<DelReply, errors::RedisError> {
        match track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::DeleteKey,
            self.redis_conn
                .pool
                .del(key.tenant_aware_key(&self.redis_conn)),
        )
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
                    track_redis_call(
                        self.request_id.as_deref(),
                        self.redis_conn.event_emitter.as_ref(),
                        RedisOperation::DeleteKey,
                        self.redis_conn
                            .pool
                            .del(key.tenant_unaware_key(&self.redis_conn)),
                    )
                    .await
                    .change_context(errors::RedisError::DeleteFailed)
                }
            }
        }
    }

    // deja: NO boundary — the inner delete_key calls carry record/replay; an outer
    // boundary would nest and omit them. See `docs/design/deja-non-boundaries.md`.
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

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_key_with_expiry",
            codec = deja::codec::ResultCodec::<(), errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "SETEX",
                    "ttl_seconds": seconds,
                })
            },
        )
    )]
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
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetKeyWithExpiry,
            self.redis_conn.pool.set(
                key.tenant_aware_key(&self.redis_conn),
                value,
                Some(Expiration::EX(seconds)),
                None,
                false,
            ),
        )
        .await
        .change_context(errors::RedisError::SetExFailed)
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_key_if_not_exists_with_expiry",
            codec = deja::codec::ResultCodec::<SetnxReply, errors::RedisError>,
            // Declare the key we WRITE so the seed plan's pristine rule masks a
            // later read-back of it (e.g. the lock GET after this SETNX). Without
            // this the read-back is seeded as "present", and the replayed Execute
            // SETNX then sees the lock held → never acquires → retry storm.
            state_write = key.tenant_aware_key(&self.redis_conn),
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "SETNX",
                    "ttl_seconds": seconds,
                })
            },
        )
    )]
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
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetKeyIfNotExistsWithExpiry,
            self.redis_conn.pool.set(
                key.tenant_aware_key(&self.redis_conn),
                value,
                Some(Expiration::EX(
                    seconds.unwrap_or(self.redis_conn.config.default_ttl.into()),
                )),
                Some(SetOptions::NX),
                false,
            ),
        )
        .await
        .change_context(errors::RedisError::SetFailed)
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_expiry",
            codec = deja::codec::ResultCodec::<(), errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "EXPIRE",
                    "ttl_seconds": seconds,
                })
            },
        )
    )]
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expiry(
        &self,
        key: &RedisKey,
        seconds: i64,
    ) -> CustomResult<(), errors::RedisError> {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetExpiry,
            self.redis_conn
                .pool
                .expire(key.tenant_aware_key(&self.redis_conn), seconds),
        )
        .await
        .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_expire_at",
            codec = deja::codec::ResultCodec::<(), errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "EXPIREAT",
                    "timestamp": timestamp,
                })
            },
        )
    )]
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn set_expire_at(
        &self,
        key: &RedisKey,
        timestamp: i64,
    ) -> CustomResult<(), errors::RedisError> {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetExpireAt,
            self.redis_conn
                .pool
                .expire_at(key.tenant_aware_key(&self.redis_conn), timestamp),
        )
        .await
        .change_context(errors::RedisError::SetExpiryFailed)
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "get_ttl",
            codec = deja::codec::ResultCodec::<i64, errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "TTL",
                })
            },
        )
    )]
    #[instrument(level = "DEBUG", skip(self))]
    pub async fn get_ttl(&self, key: &RedisKey) -> CustomResult<i64, errors::RedisError> {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::GetTtl,
            self.redis_conn
                .pool
                .ttl(key.tenant_aware_key(&self.redis_conn)),
        )
        .await
        .change_context(errors::RedisError::GetFailed)
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_hash_fields",
            codec = deja::codec::ResultCodec::<(), errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "HSET",
                    "ttl_seconds": ttl,
                })
            },
        )
    )]
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

        let output: Result<(), _> = track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetHashFields,
            self.redis_conn
                .pool
                .hset(key.tenant_aware_key(&self.redis_conn), map),
        )
        .await
        .change_context(errors::RedisError::SetHashFailed);
        // setting expiry for the key
        #[cfg(not(feature = "deja"))]
        {
            output
                .async_and_then(|_| {
                    self.set_expiry(
                        key,
                        ttl.unwrap_or(self.redis_conn.config.default_hash_ttl.into()),
                    )
                })
                .await
        }
        // Deja: set expiry via a RAW `pool.expire`, NOT the instrumented
        // `set_expiry`. This method already carries its own `set_hash_fields`
        // Ok-only codec boundary; nesting the instrumented `set_expiry` under it
        // would orphan the inner EXPIRE event on replay (the outer no-op
        // substitution skips it → "omitted" divergence). Inlining keeps
        // HSET+EXPIRE as one recorded write that round-trips cleanly.
        #[cfg(feature = "deja")]
        {
            output
                .async_and_then(|_| async {
                    let _: () = track_redis_call(
                        self.request_id.as_deref(),
                        self.redis_conn.event_emitter.as_ref(),
                        RedisOperation::SetExpiry,
                        self.redis_conn.pool.expire(
                            key.tenant_aware_key(&self.redis_conn),
                            ttl.unwrap_or(self.redis_conn.config.default_hash_ttl.into()),
                        ),
                    )
                    .await
                    .change_context(errors::RedisError::SetExpiryFailed)?;
                    Ok(())
                })
                .await
        }
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_hash_field_if_not_exist",
            codec = deja::codec::ResultCodec::<HsetnxReply, errors::RedisError>,
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "HSETNX",
                    "field": field,
                    "ttl_seconds": ttl,
                })
            },
        )
    )]
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
        let output: Result<HsetnxReply, _> = track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetHashFieldIfNotExist,
            self.redis_conn
                .pool
                .hsetnx(key.tenant_aware_key(&self.redis_conn), field, value),
        )
        .await
        .change_context(errors::RedisError::SetHashFieldFailed);

        #[cfg(not(feature = "deja"))]
        {
            output
                .async_and_then(|inner| async {
                    self.set_expiry(
                        key,
                        ttl.unwrap_or(self.redis_conn.config.default_hash_ttl)
                            .into(),
                    )
                    .await?;
                    Ok(inner)
                })
                .await
        }
        // Deja: raw `pool.expire` (not the instrumented `set_expiry`) for the same
        // reason as `set_hash_fields`: this method's own `set_hash_field_if_not_exist`
        // Ok-only boundary would skip a nested instrumented EXPIRE on replay,
        // orphaning its recorded event.
        #[cfg(feature = "deja")]
        {
            output
                .async_and_then(|inner| async {
                    let _: () = track_redis_call(
                        self.request_id.as_deref(),
                        self.redis_conn.event_emitter.as_ref(),
                        RedisOperation::SetExpiry,
                        self.redis_conn.pool.expire(
                            key.tenant_aware_key(&self.redis_conn),
                            ttl.unwrap_or(self.redis_conn.config.default_hash_ttl)
                                .into(),
                        ),
                    )
                    .await
                    .change_context(errors::RedisError::SetExpiryFailed)?;
                    Ok(inner)
                })
                .await
        }
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
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "increment_fields_in_hash",
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "HINCRBY",
                    "field_count": fields_to_increment.len(),
                })
            },
        )
    )]
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
                track_redis_call(
                    self.request_id.as_deref(),
                    self.redis_conn.event_emitter.as_ref(),
                    RedisOperation::IncrementFieldsInHash,
                    self.redis_conn.pool.hincrby(
                        key.tenant_aware_key(&self.redis_conn),
                        field.to_string(),
                        *increment,
                    ),
                )
                .await
                .change_context(errors::RedisError::IncrementHashFieldFailed)?,
            )
        }

        Ok(values_after_increment)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "hscan",
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "HSCAN",
                    "pattern": pattern,
                })
            },
        )
    )]
    pub async fn hscan(
        &self,
        key: &RedisKey,
        pattern: &str,
        count: Option<u32>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        use futures::{FutureExt, StreamExt};

        Ok(track_redis_call(self.request_id.as_deref(), self.redis_conn.event_emitter.as_ref(),
            RedisOperation::Hscan,
            self.redis_conn.pool
                .next()
                .hscan::<&str, &str>(&key.tenant_aware_key(&self.redis_conn), pattern, count)
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
                .map(crate::metrics::ScanOutcome),
        )
        .await
        .0)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "scan",
            args = {
                serde_json::json!({
                    "pattern": pattern.as_str(),
                    "command": "SCAN",
                })
            },
        )
    )]
    pub async fn scan(
        &self,
        pattern: &RedisKey,
        count: Option<u32>,
        scan_type: Option<crate::types::RedisScanType>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        use futures::{FutureExt, StreamExt};

        let fred_scan_type = scan_type.map(fred::types::ScanType::from);

        Ok(track_redis_call(self.request_id.as_deref(), self.redis_conn.event_emitter.as_ref(),
            RedisOperation::Scan,
            self.redis_conn.pool
                .next()
                .scan(pattern.tenant_aware_key(&self.redis_conn), count, fred_scan_type)
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
                .map(crate::metrics::ScanOutcome),
        )
        .await
        .0)
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

    // Deja hermetic boundary for HGET — same shape as `get_key_raw`: the boundary
    // lives on this inner method which fetches the RAW reply and converts it into
    // the serde-native `RedisWireValue`, so the typed `ResultCodec` substitutes the field read
    // WITHOUT leaking a serde bound onto the public `get_hash_field<V>`. A
    // record-only `result={"ok":bool}` capture would record NO value, leaving
    // replay nothing to reconstruct `V` from — the read would fall through to
    // live redis.
    #[cfg(feature = "deja")]
    #[instrument(level = "DEBUG", skip(self))]
    #[deja::redis(
        operation = "get_hash_field",
        codec = deja::codec::ResultCodec::<RedisWireValue, errors::RedisError>,
        state_read = format!("{}:{}", key.tenant_aware_key(&self.redis_conn), field),
        args = {
            serde_json::json!({
                "key": key.as_str(),
                "command": "HGET",
                "field": field,
            })
        },
    )]
    async fn get_hash_field_raw(
        &self,
        key: &RedisKey,
        field: &str,
    ) -> CustomResult<RedisWireValue, errors::RedisError> {
        match track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::GetHashField,
            self.redis_conn
                .pool
                .hget::<RedisValue, _, _>(key.tenant_aware_key(&self.redis_conn), field),
        )
        .await
        .change_context(errors::RedisError::GetHashFieldFailed)
        {
            Ok(v) => Ok(v.into()),
            Err(_err) => {
                #[cfg(feature = "multitenancy_fallback")]
                {
                    track_redis_call(
                        self.request_id.as_deref(),
                        self.redis_conn.event_emitter.as_ref(),
                        RedisOperation::GetHashField,
                        self.redis_conn.pool.hget::<RedisValue, _, _>(
                            key.tenant_unaware_key(&self.redis_conn),
                            field,
                        ),
                    )
                    .await
                    .change_context(errors::RedisError::GetHashFieldFailed)
                    .map(RedisWireValue::from)
                }

                #[cfg(not(feature = "multitenancy_fallback"))]
                {
                    Err(_err)
                }
            }
        }
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
        #[cfg(feature = "deja")]
        {
            let raw = self.get_hash_field_raw(key, field).await?;
            let value: RedisValue = raw.try_into().map_err(|err: fred::error::RedisError| {
                report!(err).change_context(errors::RedisError::GetHashFieldFailed)
            })?;
            V::from_value(value).change_context(errors::RedisError::GetHashFieldFailed)
        }

        #[cfg(not(feature = "deja"))]
        {
            match track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                RedisOperation::GetHashField,
                self.redis_conn
                    .pool
                    .hget(key.tenant_aware_key(&self.redis_conn), field),
            )
            .await
            .change_context(errors::RedisError::GetHashFieldFailed)
            {
                Ok(v) => Ok(v),
                Err(_err) => {
                    #[cfg(feature = "multitenancy_fallback")]
                    {
                        track_redis_call(
                            self.request_id.as_deref(),
                            self.redis_conn.event_emitter.as_ref(),
                            RedisOperation::GetHashField,
                            self.redis_conn
                                .pool
                                .hget(key.tenant_unaware_key(&self.redis_conn), field),
                        )
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
    }

    #[cfg(feature = "deja")]
    #[instrument(level = "DEBUG", skip(self))]
    #[deja::redis(
        operation = "get_hash_fields",
        codec = deja::codec::ResultCodec::<RedisWireValue, errors::RedisError>,
        state_read = key.tenant_aware_key(&self.redis_conn),
        args = {
            serde_json::json!({
                "key": key.as_str(),
                "command": "HGETALL",
            })
        },
    )]
    async fn get_hash_fields_raw(
        &self,
        key: &RedisKey,
    ) -> CustomResult<RedisWireValue, errors::RedisError> {
        match track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::GetHashFields,
            self.redis_conn
                .pool
                .hgetall::<RedisValue, _>(key.tenant_aware_key(&self.redis_conn)),
        )
        .await
        .change_context(errors::RedisError::GetHashFieldFailed)
        {
            Ok(v) => Ok(v.into()),
            Err(_err) => {
                #[cfg(feature = "multitenancy_fallback")]
                {
                    track_redis_call(
                        self.request_id.as_deref(),
                        self.redis_conn.event_emitter.as_ref(),
                        RedisOperation::GetHashFields,
                        self.redis_conn
                            .pool
                            .hgetall::<RedisValue, _>(key.tenant_unaware_key(&self.redis_conn)),
                    )
                    .await
                    .change_context(errors::RedisError::GetHashFieldFailed)
                    .map(RedisWireValue::from)
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
        #[cfg(feature = "deja")]
        {
            let raw = self.get_hash_fields_raw(key).await?;
            let value: RedisValue = raw.try_into().map_err(|err: fred::error::RedisError| {
                report!(err).change_context(errors::RedisError::GetHashFieldFailed)
            })?;
            V::from_value(value).change_context(errors::RedisError::GetHashFieldFailed)
        }

        #[cfg(not(feature = "deja"))]
        {
            match track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                RedisOperation::GetHashFields,
                self.redis_conn
                    .pool
                    .hgetall(key.tenant_aware_key(&self.redis_conn)),
            )
            .await
            .change_context(errors::RedisError::GetHashFieldFailed)
            {
                Ok(v) => Ok(v),
                Err(_err) => {
                    #[cfg(feature = "multitenancy_fallback")]
                    {
                        track_redis_call(
                            self.request_id.as_deref(),
                            self.redis_conn.event_emitter.as_ref(),
                            RedisOperation::GetHashFields,
                            self.redis_conn
                                .pool
                                .hgetall(key.tenant_unaware_key(&self.redis_conn)),
                        )
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
    pub async fn delete_hash_fields<F>(
        &self,
        key: &RedisKey,
        fields: F,
    ) -> CustomResult<usize, errors::RedisError>
    where
        F: Into<MultipleKeys> + Debug + Send + Sync,
    {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::DeleteHashFields,
            self.redis_conn
                .pool
                .hdel(key.tenant_aware_key(&self.redis_conn), fields),
        )
        .await
        .change_context(errors::RedisError::DeleteHashFieldFailed)
    }

    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "sadd",
            codec = deja::codec::ResultCodec::<SaddReply, errors::RedisError>,
            state_write = key.tenant_aware_key(&self.redis_conn),
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "SADD",
                })
            },
        )
    )]
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
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::Sadd,
            self.redis_conn
                .pool
                .sadd(key.tenant_aware_key(&self.redis_conn), members),
        )
        .await
        .change_context(errors::RedisError::SetAddMembersFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "stream_append_entry",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XADD",
                })
            },
        )
    )]
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

        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::StreamAppendEntry,
            self.redis_conn.pool.xadd(
                stream.tenant_aware_key(&self.redis_conn),
                false,
                None,
                entry_id,
                fred_fields,
            ),
        )
        .await
        .change_context(errors::RedisError::StreamAppendFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "stream_delete_entries",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XDEL",
                })
            },
        )
    )]
    pub async fn stream_delete_entries(
        &self,
        stream: &RedisKey,
        ids: Vec<String>,
    ) -> CustomResult<usize, errors::RedisError> {
        let fred_ids: MultipleStrings = ids.into();
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::StreamDeleteEntries,
            self.redis_conn
                .pool
                .xdel(stream.tenant_aware_key(&self.redis_conn), fred_ids),
        )
        .await
        .change_context(errors::RedisError::StreamDeleteFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "stream_trim_entries",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XTRIM",
                })
            },
        )
    )]
    pub async fn stream_trim_entries(
        &self,
        stream: &RedisKey,
        config: StreamTrimConfig,
    ) -> CustomResult<usize, errors::RedisError> {
        let xcap = fred::types::XCap::try_from(config)
            .change_context(errors::RedisError::StreamTrimFailed)
            .attach_printable("Failed to convert StreamTrimConfig to fred::types::XCap")?;
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::StreamTrimEntries,
            self.redis_conn
                .pool
                .xtrim(stream.tenant_aware_key(&self.redis_conn), xcap),
        )
        .await
        .change_context(errors::RedisError::StreamTrimFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "stream_acknowledge_entries",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XACK",
                    "group": group,
                })
            },
        )
    )]
    pub async fn stream_acknowledge_entries(
        &self,
        stream: &RedisKey,
        group: &str,
        ids: Vec<String>,
    ) -> CustomResult<usize, errors::RedisError> {
        let fred_ids: MultipleIDs = ids.into();
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::StreamAcknowledgeEntries,
            self.redis_conn
                .pool
                .xack(stream.tenant_aware_key(&self.redis_conn), group, fred_ids),
        )
        .await
        .change_context(errors::RedisError::StreamAcknowledgeFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "stream_get_length",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XLEN",
                })
            },
        )
    )]
    pub async fn stream_get_length(
        &self,
        stream: &RedisKey,
    ) -> CustomResult<usize, errors::RedisError> {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::StreamGetLength,
            self.redis_conn
                .pool
                .xlen(stream.tenant_aware_key(&self.redis_conn)),
        )
        .await
        .change_context(errors::RedisError::GetLengthFailed)
    }

    fn get_keys_with_prefix(&self, streams: &[RedisKey]) -> MultipleKeys {
        let res: Vec<String> = streams
            .iter()
            .map(|k| k.tenant_aware_key(&self.redis_conn))
            .collect();
        MultipleKeys::from(res)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "stream_read_entries",
            args = {
                serde_json::json!({
                    "command": "XREAD",
                })
            },
        )
    )]
    pub async fn stream_read_entries(
        &self,
        streams: &[RedisKey],
        ids: Vec<String>,
        read_count: Option<u64>,
    ) -> CustomResult<StreamReadResult, errors::RedisError> {
        let strms = self.get_keys_with_prefix(streams);
        let ids: MultipleIDs = ids.into();
        let reply: XReadResponse<String, String, String, String> = track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::StreamReadEntries,
            self.redis_conn.pool.xread_map(
                Some(read_count.unwrap_or(self.redis_conn.config.default_stream_read_count)),
                None,
                strms,
                ids,
            ),
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
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "stream_read_with_options",
            args = {
                serde_json::json!({
                    "command": if group.is_some() { "XREADGROUP" } else { "XREAD" },
                })
            },
        )
    )]
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
                track_redis_call(
                    self.request_id.as_deref(),
                    self.redis_conn.event_emitter.as_ref(),
                    RedisOperation::StreamReadWithOptions,
                    self.redis_conn.pool.xreadgroup_map(
                        group_name,
                        consumer_name,
                        count,
                        block,
                        false,
                        strms,
                        ids,
                    ),
                )
                .await
            }
            None => {
                track_redis_call(
                    self.request_id.as_deref(),
                    self.redis_conn.event_emitter.as_ref(),
                    RedisOperation::StreamReadWithOptions,
                    self.redis_conn.pool.xread_map(count, block, strms, ids),
                )
                .await
            }
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
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "append_elements_to_list",
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "RPUSH",
                })
            },
        )
    )]
    pub async fn append_elements_to_list<V>(
        &self,
        key: &RedisKey,
        elements: V,
    ) -> CustomResult<(), errors::RedisError>
    where
        V: TryInto<MultipleValues> + Debug + Send,
        V::Error: Into<fred::error::RedisError> + Send,
    {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::AppendElementsToList,
            self.redis_conn
                .pool
                .rpush(key.tenant_aware_key(&self.redis_conn), elements),
        )
        .await
        .change_context(errors::RedisError::AppendElementsToListFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "get_list_elements",
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "LRANGE",
                    "start": start,
                    "stop": stop,
                })
            },
        )
    )]
    pub async fn get_list_elements(
        &self,
        key: &RedisKey,
        start: i64,
        stop: i64,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::GetListElements,
            self.redis_conn
                .pool
                .lrange(key.tenant_aware_key(&self.redis_conn), start, stop),
        )
        .await
        .change_context(errors::RedisError::GetListElementsFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "get_list_length",
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "LLEN",
                })
            },
        )
    )]
    pub async fn get_list_length(&self, key: &RedisKey) -> CustomResult<usize, errors::RedisError> {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::GetListLength,
            self.redis_conn
                .pool
                .llen(key.tenant_aware_key(&self.redis_conn)),
        )
        .await
        .change_context(errors::RedisError::GetListLengthFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "lpop_list_elements",
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "LPOP",
                    "count": count,
                })
            },
        )
    )]
    pub async fn lpop_list_elements(
        &self,
        key: &RedisKey,
        count: Option<usize>,
    ) -> CustomResult<Vec<String>, errors::RedisError> {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::LpopListElements,
            self.redis_conn
                .pool
                .lpop(key.tenant_aware_key(&self.redis_conn), count),
        )
        .await
        .change_context(errors::RedisError::PopListElementsFailed)
    }

    //                                              Consumer Group API

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "consumer_group_create",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XGROUP_CREATE",
                    "group": group,
                })
            },
        )
    )]
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

        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::ConsumerGroupCreate,
            self.redis_conn.pool.xgroup_create(
                stream.tenant_aware_key(&self.redis_conn),
                group,
                id,
                true,
            ),
        )
        .await
        .change_context(errors::RedisError::ConsumerGroupCreateFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "consumer_group_destroy",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XGROUP_DESTROY",
                    "group": group,
                })
            },
        )
    )]
    pub async fn consumer_group_destroy(
        &self,
        stream: &RedisKey,
        group: &str,
    ) -> CustomResult<crate::types::ConsumerGroupDestroyReply, errors::RedisError> {
        let reply: crate::types::ConsumerGroupDestroyReply = track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::ConsumerGroupDestroy,
            self.redis_conn
                .pool
                .xgroup_destroy(stream.tenant_aware_key(&self.redis_conn), group),
        )
        .await
        .change_context(errors::RedisError::ConsumerGroupDestroyFailed)?;
        Ok(reply)
    }

    // the number of pending messages that the consumer had before it was deleted
    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "consumer_group_delete_consumer",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XGROUP_DELCONSUMER",
                    "group": group,
                    "consumer": consumer,
                })
            },
        )
    )]
    pub async fn consumer_group_delete_consumer(
        &self,
        stream: &RedisKey,
        group: &str,
        consumer: &str,
    ) -> CustomResult<usize, errors::RedisError> {
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::ConsumerGroupDeleteConsumer,
            self.redis_conn.pool.xgroup_delconsumer(
                stream.tenant_aware_key(&self.redis_conn),
                group,
                consumer,
            ),
        )
        .await
        .change_context(errors::RedisError::ConsumerGroupRemoveConsumerFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "consumer_group_set_last_id",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XGROUP_SETID",
                    "group": group,
                })
            },
        )
    )]
    pub async fn consumer_group_set_last_id(
        &self,
        stream: &RedisKey,
        group: &str,
        id: &RedisEntryId,
    ) -> CustomResult<String, errors::RedisError> {
        let id_str = id.to_stream_id();
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::ConsumerGroupSetLastId,
            self.redis_conn.pool.xgroup_setid::<(), _, _, _>(
                stream.tenant_aware_key(&self.redis_conn),
                group,
                &id_str,
            ),
        )
        .await
        .change_context(errors::RedisError::ConsumerGroupSetIdFailed)?;
        Ok(id_str)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "consumer_group_set_message_owner",
            args = {
                serde_json::json!({
                    "key": stream.as_str(),
                    "command": "XCLAIM",
                    "group": group,
                    "consumer": consumer,
                })
            },
            // Generic `R: FromRedis` is NOT `Debug`, so the default `result_debug`
            // capture would not compile. Record only the ok/err verdict (record-only
            // leaf — no replay reconstruction needed).
            result = {
                (
                    match &__deja_result {
                        Ok(_) => serde_json::json!({"ok": true}),
                        Err(e) => serde_json::json!({"ok": false, "error": format!("{:?}", e)}),
                    },
                    __deja_result.is_err(),
                )
            },
        )
    )]
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
        track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::ConsumerGroupSetMessageOwner,
            self.redis_conn.pool.xclaim(
                stream.tenant_aware_key(&self.redis_conn),
                group,
                consumer,
                min_idle_time,
                fred_ids,
                None,
                None,
                None,
                false,
                false,
            ),
        )
        .await
        .change_context(errors::RedisError::ConsumerGroupClaimFailed)
    }

    #[instrument(level = "DEBUG", skip(self))]
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            replay = Substitute,
            operation = "evaluate_redis_script",
            args = {
                serde_json::json!({
                    "command": "EVAL",
                    "key_count": key.len(),
                })
            },
            // Generic `T` is NOT `Debug`, so the default `result_debug` capture
            // would not compile. Record only the ok/err verdict (record-only leaf —
            // an EVAL result is never reconstructed without an explicit override).
            result = {
                (
                    match &__deja_result {
                        Ok(_) => serde_json::json!({"ok": true}),
                        Err(e) => serde_json::json!({"ok": false, "error": format!("{:?}", e)}),
                    },
                    __deja_result.is_err(),
                )
            },
        )
    )]
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
        let val: T = track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::EvaluateRedisScript,
            self.redis_conn.pool.eval(lua_script, key, values),
        )
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
    #[cfg_attr(
        feature = "deja",
        deja::redis(
            operation = "set_key_if_not_exists_and_get_value",
            args = {
                serde_json::json!({
                    "key": key.as_str(),
                    "command": "SETNX_GET",
                    "ttl_seconds": ttl,
                })
            },
            // `SetGetReply<V>` carries a generic `V` with no `Debug` bound, so the
            // default `result_debug` capture would not compile. Record only the
            // ok/err verdict (record-only leaf — no replay reconstruction needed;
            // an RMW transaction always re-executes live on replay).
            result = {
                (
                    match &__deja_result {
                        Ok(_) => serde_json::json!({"ok": true}),
                        Err(e) => serde_json::json!({"ok": false, "error": format!("{:?}", e)}),
                    },
                    __deja_result.is_err(),
                )
            },
        )
    )]
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
        let redis_key = key.tenant_aware_key(&self.redis_conn);
        let ttl_seconds = ttl.unwrap_or(self.redis_conn.config.default_ttl.into());

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
        let mut results: Vec<RedisValue> = track_redis_call(
            self.request_id.as_deref(),
            self.redis_conn.event_emitter.as_ref(),
            RedisOperation::SetKeyIfNotExistsAndGetValue,
            trx.exec(true),
        )
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
