//! Redis interface — compile-time backend selection via Cargo feature.
//!
//! By default the `redis-rs` crate is used. Enable the `fred` feature to switch.
//!
//! # Examples
//! ```ignore
//! use std::sync::Arc;
//! use common_utils::external_service::NoOpEventEmitter;
//! use redis_interface::{types::RedisSettings, RedisConnectionPool};
//!
//! #[tokio::main]
//! async fn main() {
//!     let emitter = Arc::new(NoOpEventEmitter);
//!     let redis_conn = RedisConnectionPool::new(&RedisSettings::default(), emitter).await;
//! }
//! ```

// Compile-time guards: exactly one backend must be active.
#[cfg(not(any(feature = "fred", feature = "redis-rs")))]
compile_error!("Either feature \"fred\" or \"redis-rs\" must be enabled for this crate.");

#[cfg(all(feature = "fred", feature = "redis-rs"))]
compile_error!("Features \"fred\" and \"redis-rs\" are mutually exclusive — enable only one.");

pub mod constant;
pub mod errors;
pub(crate) mod metrics;
pub mod types;

#[cfg(feature = "fred")]
mod module {
    pub mod fred;
}

#[cfg(feature = "redis-rs")]
mod module {
    pub mod redis_rs;
}

// Re-export the active backend's public types under unified names.
// All external code imports `redis_interface::RedisConnectionPool` etc.
// and is never aware of which backend is active.

#[cfg(feature = "fred")]
pub use fred::interfaces::{EventInterface, PubsubInterface};
#[cfg(feature = "fred")]
pub use module::fred::{
    PubSubMessage, RedisClient, RedisConfig, RedisConnectionPool, RedisConnectionWithContext,
    SubscriberClient,
};
#[cfg(feature = "redis-rs")]
pub use module::redis_rs::{
    redis_value_to_option_string, PubSubMessage, PublisherClient, RedisConfig, RedisConn,
    RedisConnectionPool, RedisConnectionWithContext, SubscriberClient,
};

pub use self::types::*;

#[cfg(test)]
mod test;

const CACHE_CAS_SCRIPT: &str = r#"
local current = redis.call('GET', KEYS[1])
if current == ARGV[2] then return 1 end
if current ~= ARGV[1] then return 0 end
redis.call('SET', KEYS[1], ARGV[2], 'KEEPTTL')
return 1
"#;

impl RedisConnectionWithContext {
    /// Return the existing bytes, or reserve an absent key with an expiring token.
    /// SET NX GET performs both operations atomically in one trip (Redis 7+).
    #[cfg_attr(feature = "deja", deja::redis(
        operation = "cache_read_or_reserve",
        codec = deja::codec::ResultCodec::<Option<Vec<u8>>, errors::RedisError>,
        args = serde_json::json!({"key": key.as_str(), "tenant": self.redis_conn.key_prefix, "ttl_seconds": ttl}),
    ))]
    #[tracing::instrument(level = "DEBUG", skip_all)]
    pub async fn get_key_or_reserve(
        &self,
        key: &RedisKey,
        value: &[u8],
        ttl: Option<i64>,
    ) -> common_utils::errors::CustomResult<Option<Vec<u8>>, errors::RedisError> {
        use error_stack::ResultExt;
        let key = key.tenant_aware_key(&self.redis_conn);
        let ttl = ttl.unwrap_or(self.redis_conn.config.default_ttl.into());
        #[cfg(feature = "redis-rs")]
        {
            let mut conn = self.redis_conn.pool.clone();
            metrics::track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                metrics::RedisOperation::SetKeyIfNotExistsAndGetValue,
                redis::Cmd::with_capacity(7, key.len() + value.len() + 24)
                    .arg("SET")
                    .arg(key)
                    .arg(value)
                    .arg("NX")
                    .arg("GET")
                    .arg("EX")
                    .arg(ttl)
                    .query_async(&mut conn),
            )
            .await
            .change_context(errors::RedisError::SetFailed)
        }
        #[cfg(feature = "fred")]
        {
            use fred::{
                interfaces::KeysInterface,
                types::{Expiration, SetOptions},
            };
            metrics::track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                metrics::RedisOperation::SetKeyIfNotExistsAndGetValue,
                self.redis_conn.pool.set(
                    key,
                    value,
                    Some(Expiration::EX(ttl)),
                    Some(SetOptions::NX),
                    true,
                ),
            )
            .await
            .change_context(errors::RedisError::SetFailed)
        }
    }

    /// Reserve a malformed cache entry only if it still matches `expected`.
    #[cfg_attr(feature = "deja", deja::redis(
        operation = "cache_reserve_if_unchanged",
        codec = deja::codec::ResultCodec::<bool, errors::RedisError>,
        args = serde_json::json!({"key": key.as_str(), "tenant": self.redis_conn.key_prefix, "expected": expected, "ttl_seconds": ttl}),
    ))]
    #[tracing::instrument(level = "DEBUG", skip_all)]
    pub async fn reserve_cache_if_unchanged(
        &self,
        key: &RedisKey,
        expected: &[u8],
        reservation: &[u8],
        ttl: Option<i64>,
    ) -> common_utils::errors::CustomResult<bool, errors::RedisError> {
        use error_stack::ResultExt;
        const SCRIPT: &str = r#"
if redis.call('GET', KEYS[1]) ~= ARGV[1] then return 0 end
redis.call('SET', KEYS[1], ARGV[2], 'EX', ARGV[3])
return 1
"#;
        let key = key.tenant_aware_key(&self.redis_conn);
        let ttl = ttl.unwrap_or(self.redis_conn.config.default_ttl.into());
        #[cfg(feature = "redis-rs")]
        {
            let mut conn = self.redis_conn.pool.clone();
            metrics::track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                metrics::RedisOperation::EvaluateRedisScript,
                redis::cmd("EVAL")
                    .arg(SCRIPT)
                    .arg(1)
                    .arg(key)
                    .arg(expected)
                    .arg(reservation)
                    .arg(ttl)
                    .query_async(&mut conn),
            )
            .await
            .change_context(errors::RedisError::ScriptExecutionFailed)
        }
        #[cfg(feature = "fred")]
        {
            use fred::{
                interfaces::ClientLike,
                types::{ClusterHash, CustomCommand, RedisValue},
            };
            let hash = if self.redis_conn.config.cluster_enabled {
                ClusterHash::from(key.as_str())
            } else {
                ClusterHash::FirstKey
            };
            let args: Vec<RedisValue> = vec![
                SCRIPT.into(),
                1.into(),
                key.into(),
                expected.into(),
                reservation.into(),
                ttl.into(),
            ];
            metrics::track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                metrics::RedisOperation::EvaluateRedisScript,
                self.redis_conn
                    .pool
                    .custom(CustomCommand::new_static("EVAL", hash, false), args),
            )
            .await
            .change_context(errors::RedisError::ScriptExecutionFailed)
        }
    }

    /// Store `value` if `expected` still matches.
    /// Also succeeds if `value` is already cached.
    #[cfg_attr(feature = "deja", deja::redis(
        operation = "cache_compare_and_set",
        codec = deja::codec::ResultCodec::<bool, errors::RedisError>,
        args = serde_json::json!({"key": key.as_str(), "tenant": self.redis_conn.key_prefix, "expected": expected}),
    ))]
    #[tracing::instrument(level = "DEBUG", skip_all)]
    pub async fn compare_and_set_cache(
        &self,
        key: &RedisKey,
        expected: &[u8],
        value: &[u8],
    ) -> common_utils::errors::CustomResult<bool, errors::RedisError> {
        use error_stack::ResultExt;
        let key = key.tenant_aware_key(&self.redis_conn);
        #[cfg(feature = "redis-rs")]
        {
            static SCRIPT: std::sync::LazyLock<redis::Script> =
                std::sync::LazyLock::new(|| redis::Script::new(CACHE_CAS_SCRIPT));
            let mut conn = self.redis_conn.pool.clone();
            let mut command = "EVALSHA";
            let mut body = SCRIPT.get_hash();
            loop {
                let result = metrics::track_redis_call(
                    self.request_id.as_deref(),
                    self.redis_conn.event_emitter.as_ref(),
                    metrics::RedisOperation::EvaluateRedisScript,
                    redis::Cmd::with_capacity(
                        6,
                        body.len() + key.len() + expected.len() + value.len() + 12,
                    )
                    .arg(command)
                    .arg(body)
                    .arg(1)
                    .arg(&key)
                    .arg(expected)
                    .arg(value)
                    .query_async(&mut conn),
                )
                .await;
                match result {
                    Err(ref err)
                        if command == "EVALSHA"
                            && err.kind()
                                == redis::ErrorKind::Server(redis::ServerErrorKind::NoScript) =>
                    {
                        command = "EVAL";
                        body = CACHE_CAS_SCRIPT;
                    }
                    result => {
                        return result.change_context(errors::RedisError::ScriptExecutionFailed)
                    }
                }
            }
        }
        #[cfg(feature = "fred")]
        {
            use fred::{
                interfaces::ClientLike,
                types::{ClusterHash, CustomCommand, RedisValue},
            };
            // Hash the key explicitly; Fred 8's Offset policy hashes the script.
            let hash = if self.redis_conn.config.cluster_enabled {
                ClusterHash::from(key.as_str())
            } else {
                ClusterHash::FirstKey
            };
            // Full EVAL avoids NOSCRIPT retries and keeps unrelated fills pipelined.
            let command = CustomCommand::new_static("EVAL", hash, false);
            let args: Vec<RedisValue> = vec![
                CACHE_CAS_SCRIPT.into(),
                1.into(),
                key.into(),
                expected.into(),
                value.into(),
            ];
            metrics::track_redis_call(
                self.request_id.as_deref(),
                self.redis_conn.event_emitter.as_ref(),
                metrics::RedisOperation::EvaluateRedisScript,
                self.redis_conn.pool.custom(command, args),
            )
            .await
            .change_context(errors::RedisError::ScriptExecutionFailed)
        }
    }
}
