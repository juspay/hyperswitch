//! Shared data types — backend-neutral.
//!
//! All struct/enum *definitions* live here. Backend-specific trait
//! implementations (`FromRedisValue`, `FromRedis`, `ToRedisArgs`, `Deref`,
//! `From<RedisValue> for …`) live in each backend's `types.rs` instead.
//!
//! The one cfg-gated field that cannot be split is `RedisValue::inner`,
//! because Rust does not allow splitting a struct definition across files.

use common_utils::errors::CustomResult;

use crate::errors;

// ─── RedisValue — wrapper whose inner type depends on the active backend ─────

#[derive(Clone, Debug)]
pub struct RedisValue {
    #[cfg(feature = "redis-rs")]
    pub(crate) inner: redis::Value,
    #[cfg(not(feature = "redis-rs"))]
    pub(crate) inner: fred::types::RedisValue,
}

// Method impls are in backends/redis_rs/types.rs and backends/fred/types.rs.

// ─── Shared configuration types ─────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, Clone)]
#[serde(default)]
pub struct RedisSettings {
    pub host: String,
    pub port: u16,
    pub cluster_enabled: bool,
    pub cluster_urls: Vec<String>,
    pub use_legacy_version: bool,
    /// Number of reconnection attempts before giving up (default: 5).
    /// Passed to `ConnectionManagerConfig::set_number_of_retries`.
    pub pool_size: usize,
    /// Maximum number of connection retry attempts (default: 5).
    /// Passed to `ConnectionManagerConfig::set_number_of_retries`.
    pub reconnect_max_attempts: u32,
    /// Initial delay in milliseconds between reconnection attempts (default: 5).
    /// Passed to `ConnectionManagerConfig::set_min_delay`.
    pub reconnect_delay: u32,
    /// TTL in seconds
    pub default_ttl: u32,
    /// TTL for hash-tables in seconds
    pub default_hash_ttl: u32,
    pub stream_read_count: u64,
    pub auto_pipeline: bool,
    pub disable_auto_backpressure: bool,
    /// Maximum number of in-flight commands before backpressure is applied.
    /// Set to 0 to disable. Passed to `ConnectionManagerConfig::set_pipeline_buffer_size`
    /// or `ClusterClientBuilder::connection_concurrency_limit`.
    pub max_in_flight_commands: usize,
    /// Command timeout in seconds. Passed to `ConnectionManagerConfig::set_response_timeout`.
    pub default_command_timeout: u64,
    pub max_feed_count: u64,
    pub unresponsive_timeout: u64,
    pub unresponsive_check_interval: u64,
    /// Capacity of the broadcast channel used for pub/sub message distribution.
    pub broadcast_channel_capacity: usize,
    /// Maximum duration (in seconds) that Redis can be unreachable before the server shuts down.
    pub max_failure_threshold_seconds: u32,
}

impl RedisSettings {
    /// Validates the Redis configuration provided.
    pub fn validate(&self) -> CustomResult<(), errors::RedisError> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.host.is_default_or_empty(), || {
            Err(errors::RedisError::InvalidConfiguration(
                "Redis `host` must be specified".into(),
            ))
        })?;

        when(self.cluster_enabled && self.cluster_urls.is_empty(), || {
            Err(errors::RedisError::InvalidConfiguration(
                "Redis `cluster_urls` must be specified if `cluster_enabled` is `true`".into(),
            ))
        })?;

        when(
            self.default_command_timeout < self.unresponsive_timeout,
            || {
                Err(errors::RedisError::InvalidConfiguration(
                    "Unresponsive timeout cannot be greater than the command timeout".into(),
                ))
            },
        )?;

        when(
            self.unresponsive_check_interval > self.max_failure_threshold_seconds.into(),
            || {
                Err(errors::RedisError::InvalidConfiguration(
                    "Unresponsive check interval cannot be greater than the max failure threshold"
                        .into(),
                ))
            },
        )?;

        Ok(())
    }

    // ── Connection-building helpers ────────────────────────────────────────

    /// Normalize cluster URLs by prepending `"redis://"` if the scheme is missing.
    #[cfg(feature = "redis-rs")]
    pub fn normalize_cluster_urls(&self) -> Vec<String> {
        self.cluster_urls
            .iter()
            .map(|url| {
                if url.starts_with("redis://") {
                    url.clone()
                } else {
                    format!("redis://{url}")
                }
            })
            .collect()
    }

    /// Convert `max_in_flight_commands` to `Option<usize>`.
    ///
    /// Returns `None` when `max_in_flight_commands` is 0 (feature disabled).
    #[cfg(feature = "redis-rs")]
    pub fn max_in_flight_commands_as_usize(&self) -> Option<usize> {
        (self.max_in_flight_commands > 0).then_some(self.max_in_flight_commands)
    }

    /// Convert `reconnect_max_attempts` to `usize`.
    ///
    /// Logs a warning and falls back to [`crate::constant::redis_rs_commands::DEFAULT_RECONNECT_MAX_ATTEMPTS`]
    /// when the value overflows.
    #[cfg(feature = "redis-rs")]
    pub fn reconnect_max_attempts_as_usize(&self) -> usize {
        usize::try_from(self.reconnect_max_attempts).unwrap_or_else(|_| {
            tracing::warn!(
                "reconnect_max_attempts ({}) exceeds usize, using default ({})",
                self.reconnect_max_attempts,
                crate::constant::redis_rs_commands::DEFAULT_RECONNECT_MAX_ATTEMPTS
            );
            crate::constant::redis_rs_commands::DEFAULT_RECONNECT_MAX_ATTEMPTS
        })
    }

    /// Build a standalone [`redis::ConnectionInfo`] with RESP3 protocol from host and port.
    #[cfg(feature = "redis-rs")]
    pub fn build_standalone_connection_info(
        &self,
    ) -> CustomResult<redis::ConnectionInfo, errors::RedisError> {
        use error_stack::ResultExt;
        use redis::IntoConnectionInfo;

        let connection_url = format!("redis://{}:{}", self.host, self.port);
        let mut connection_info = connection_url
            .as_str()
            .into_connection_info()
            .change_context(errors::RedisError::RedisConnectionError)?;

        let redis_settings = connection_info
            .redis_settings()
            .clone()
            .set_protocol(redis::ProtocolVersion::RESP3);
        connection_info = connection_info.set_redis_settings(redis_settings);

        Ok(connection_info)
    }

    /// Build the base [`redis::aio::ConnectionManagerConfig`] from these settings.
    ///
    /// Sets reconnection retries, minimum delay, and optional response timeout.
    /// Callers can further customize (e.g. `set_pipeline_buffer_size`) before use.
    #[cfg(feature = "redis-rs")]
    pub fn build_connection_manager_config(&self) -> redis::aio::ConnectionManagerConfig {
        let mut config = redis::aio::ConnectionManagerConfig::new()
            .set_number_of_retries(self.reconnect_max_attempts_as_usize())
            .set_min_delay(std::time::Duration::from_millis(u64::from(
                self.reconnect_delay,
            )));

        if self.default_command_timeout > 0 {
            config = config.set_response_timeout(Some(std::time::Duration::from_secs(
                self.default_command_timeout,
            )));
        }

        config
    }

    /// Build a base [`redis::cluster::ClusterClientBuilder`] with common configuration.
    ///
    /// Sets retries, retry wait, response timeout, and RESP3 protocol.
    /// Callers can further customize (e.g. `push_sender`, `connection_concurrency_limit`).
    #[cfg(feature = "redis-rs")]
    pub fn build_cluster_client_builder(
        &self,
        nodes: Vec<String>,
    ) -> redis::cluster::ClusterClientBuilder {
        redis::cluster::ClusterClient::builder(nodes)
            .retries(self.reconnect_max_attempts)
            .min_retry_wait(u64::from(self.reconnect_delay))
            .response_timeout(std::time::Duration::from_secs(
                self.default_command_timeout.max(1),
            ))
            .use_protocol(redis::ProtocolVersion::RESP3)
    }
}

impl Default for RedisSettings {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            cluster_enabled: false,
            cluster_urls: vec![],
            use_legacy_version: false,
            pool_size: 5,
            reconnect_max_attempts: 5,
            reconnect_delay: 5,
            default_ttl: 300,
            stream_read_count: 1,
            default_hash_ttl: 900,
            auto_pipeline: true,
            disable_auto_backpressure: false,
            max_in_flight_commands: 5000,
            default_command_timeout: 30,
            max_feed_count: 200,
            unresponsive_timeout: 10,
            unresponsive_check_interval: 2,
            broadcast_channel_capacity: 32,
            max_failure_threshold_seconds: 5,
        }
    }
}

// ─── RedisEntryId ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum RedisEntryId {
    UserSpecifiedID {
        milliseconds: String,
        sequence_number: String,
    },
    AutoGeneratedID,
    AfterLastID,
    /// Applicable only with consumer groups
    UndeliveredEntryID,
}

impl RedisEntryId {
    /// Convert to the string representation used by Redis stream commands
    pub fn to_stream_id(&self) -> String {
        match self {
            Self::UserSpecifiedID {
                milliseconds,
                sequence_number,
            } => format!("{milliseconds}-{sequence_number}"),
            Self::AutoGeneratedID => "*".to_string(),
            Self::AfterLastID => "$".to_string(),
            Self::UndeliveredEntryID => ">".to_string(),
        }
    }
}

// Trait impls live in backends/redis_rs/types.rs and backends/fred/types.rs.

// ─── Reply type enums ────────────────────────────────────────────────────────

#[derive(Debug, Eq, PartialEq)]
pub enum SetnxReply {
    KeySet,
    KeyNotSet, // Existing key
}

#[derive(Debug, Eq, PartialEq)]
pub enum HsetnxReply {
    KeySet,
    KeyNotSet, // Existing key
}

#[derive(Debug, Eq, PartialEq)]
pub enum MsetnxReply {
    KeysSet,
    KeysNotSet, // At least one existing key
}

#[derive(Debug, Eq, PartialEq)]
pub enum DelReply {
    KeyDeleted,
    KeyNotDeleted, // Key not found
}

impl DelReply {
    pub fn is_key_deleted(&self) -> bool {
        matches!(self, Self::KeyDeleted)
    }

    pub fn is_key_not_deleted(&self) -> bool {
        matches!(self, Self::KeyNotDeleted)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SaddReply {
    KeySet,
    KeyNotSet,
}

#[derive(Debug)]
pub enum SetGetReply<T> {
    ValueSet(T),    // Value was set and this is the value that was set
    ValueExists(T), // Value already existed and this is the existing value
}

impl<T> SetGetReply<T> {
    pub fn get_value(&self) -> &T {
        match self {
            Self::ValueSet(value) => value,
            Self::ValueExists(value) => value,
        }
    }
}

// ─── Stream types ────────────────────────────────────────────────────────────

/// Entries within a single stream, as `(entry_id, fields)`.
pub type StreamEntries = Vec<(String, std::collections::HashMap<String, String>)>;

/// Grouped result of a stream read: stream key → list of `(entry_id, fields)`.
pub type StreamReadResult = std::collections::HashMap<String, StreamEntries>;

#[derive(Debug)]
pub enum StreamCapKind {
    MinID,
    MaxLen,
}

#[derive(Debug)]
pub enum StreamCapTrim {
    Exact,
    AlmostExact,
}

// Trait impls for StreamCapKind/StreamCapTrim live in backends/fred/types.rs.

// ─── RedisKey ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RedisKey(String);

impl RedisKey {
    pub fn tenant_aware_key(&self, pool: &crate::RedisConnectionPool) -> String {
        pool.add_prefix(&self.0)
    }

    pub fn tenant_unaware_key(&self, _pool: &crate::RedisConnectionPool) -> String {
        self.0.clone()
    }
}

impl<T: AsRef<str>> From<T> for RedisKey {
    fn from(value: T) -> Self {
        let value = value.as_ref();
        Self(value.to_string())
    }
}

// ─── Tests (backend-neutral only) ────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_id_user_specified() {
        let id = RedisEntryId::UserSpecifiedID {
            milliseconds: "1234567890".to_string(),
            sequence_number: "0".to_string(),
        };
        assert_eq!(id.to_stream_id(), "1234567890-0");
    }

    #[test]
    fn test_entry_id_auto_generated() {
        assert_eq!(RedisEntryId::AutoGeneratedID.to_stream_id(), "*");
    }

    #[test]
    fn test_entry_id_after_last() {
        assert_eq!(RedisEntryId::AfterLastID.to_stream_id(), "$");
    }

    #[test]
    fn test_entry_id_undelivered() {
        assert_eq!(RedisEntryId::UndeliveredEntryID.to_stream_id(), ">");
    }

    #[test]
    fn test_redis_settings_validate_valid_defaults() {
        let settings = RedisSettings::default();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_redis_settings_validate_empty_host() {
        let settings = RedisSettings {
            host: String::new(),
            ..RedisSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_redis_settings_validate_cluster_without_urls() {
        let settings = RedisSettings {
            cluster_enabled: true,
            cluster_urls: vec![],
            ..RedisSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_redis_settings_validate_cluster_with_urls() {
        let settings = RedisSettings {
            cluster_enabled: true,
            cluster_urls: vec!["redis://localhost:7000".to_string()],
            ..RedisSettings::default()
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_redis_settings_validate_unresponsive_timeout_exceeds_command_timeout() {
        let settings = RedisSettings {
            unresponsive_timeout: 60,
            default_command_timeout: 30,
            ..RedisSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_redis_settings_default_values() {
        let settings = RedisSettings::default();
        assert_eq!(settings.host, "127.0.0.1");
        assert_eq!(settings.port, 6379);
        assert!(!settings.cluster_enabled);
        assert!(settings.cluster_urls.is_empty());
        assert!(!settings.use_legacy_version);
        assert_eq!(settings.reconnect_max_attempts, 5);
        assert_eq!(settings.reconnect_delay, 5);
        assert_eq!(settings.default_ttl, 300);
        assert_eq!(settings.default_hash_ttl, 900);
        assert_eq!(settings.broadcast_channel_capacity, 32);
        assert_eq!(settings.max_failure_threshold_seconds, 5);
    }

    #[test]
    fn test_set_get_reply_value_set_get_value() {
        let reply: SetGetReply<String> = SetGetReply::ValueSet("hello".to_string());
        assert_eq!(reply.get_value(), "hello");
    }

    #[test]
    fn test_set_get_reply_value_exists_get_value() {
        let reply: SetGetReply<String> = SetGetReply::ValueExists("world".to_string());
        assert_eq!(reply.get_value(), "world");
    }

    #[test]
    fn test_redis_key_from_string() {
        let key: RedisKey = "my_key".into();
        assert_eq!(key.0, "my_key");
    }

    #[test]
    fn test_redis_key_from_string_ref() {
        let key: RedisKey = "my_key".to_string().into();
        assert_eq!(key.0, "my_key");
    }

    #[test]
    fn test_del_reply_is_key_deleted() {
        assert!(DelReply::KeyDeleted.is_key_deleted());
        assert!(!DelReply::KeyNotDeleted.is_key_deleted());
    }

    #[test]
    fn test_del_reply_is_key_not_deleted() {
        assert!(DelReply::KeyNotDeleted.is_key_not_deleted());
        assert!(!DelReply::KeyDeleted.is_key_not_deleted());
    }
}
