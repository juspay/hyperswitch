//! Data types and type conversions
//! from `redis`'s internal data-types to custom data-types

use common_utils::{errors::CustomResult, ext_traits::ConfigExt, fp_utils::when};
use error_stack::ResultExt;
use redis::{IntoConnectionInfo, Value, Value as RedisCrateValue};

use crate::{constant, errors, RedisConnectionPool};

pub struct RedisValue {
    inner: RedisCrateValue,
}

impl std::ops::Deref for RedisValue {
    type Target = RedisCrateValue;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RedisValue {
    pub fn new(value: RedisCrateValue) -> Self {
        Self { inner: value }
    }
    pub fn into_inner(self) -> RedisCrateValue {
        self.inner
    }

    pub fn from_bytes(val: Vec<u8>) -> Self {
        Self {
            inner: RedisCrateValue::BulkString(val),
        }
    }
    pub fn from_string(value: String) -> Self {
        Self {
            inner: RedisCrateValue::SimpleString(value),
        }
    }

    /// Extract bytes from the underlying redis value.
    ///
    /// Returns `Some` for string-like variants (`BulkString`, `SimpleString`,
    /// `VerbatimString`) and `None` for all others.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            RedisCrateValue::BulkString(bytes) => Some(bytes.as_slice()),
            RedisCrateValue::SimpleString(string) => Some(string.as_bytes()),
            RedisCrateValue::VerbatimString { text, .. } => Some(text.as_bytes()),
            other => {
                tracing::debug!(
                    ?other,
                    "as_bytes() called on non-string RedisValue variant, returning None"
                );
                None
            }
        }
    }

    /// Convert to string if the value has a meaningful string representation.
    ///
    /// Returns `Some` for string-like and scalar variants, `None` for aggregates
    /// and `Nil`.
    pub fn as_string(&self) -> Option<String> {
        match &self.inner {
            RedisCrateValue::BulkString(bytes) => String::from_utf8(bytes.clone()).ok(),
            RedisCrateValue::SimpleString(string) => Some(string.clone()),
            RedisCrateValue::VerbatimString { text, .. } => Some(text.clone()),
            RedisCrateValue::Int(integer) => Some(integer.to_string()),
            RedisCrateValue::Double(double) => Some(double.to_string()),
            RedisCrateValue::Boolean(boolean) => Some(boolean.to_string()),
            RedisCrateValue::Okay => Some("OK".to_string()),
            RedisCrateValue::BigNumber(ref big_number) => Some(big_number.to_string()),
            other => {
                tracing::debug!(
                    ?other,
                    "as_string() called on non-string RedisValue variant, returning None"
                );
                None
            }
        }
    }
}

impl From<RedisValue> for RedisCrateValue {
    fn from(redis_value: RedisValue) -> Self {
        redis_value.inner
    }
}

/// Allows conversion from RedisValue to bytes for use with ToRedisArgs
impl redis::ToRedisArgs for RedisValue {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        match &self.inner {
            RedisCrateValue::BulkString(bytes) => bytes.write_redis_args(out),
            RedisCrateValue::SimpleString(string) => string.write_redis_args(out),
            RedisCrateValue::VerbatimString { text, .. } => text.write_redis_args(out),
            RedisCrateValue::Int(integer) => integer.write_redis_args(out),
            RedisCrateValue::Double(double) => double.to_string().write_redis_args(out),
            RedisCrateValue::Boolean(boolean) => {
                (if *boolean { 1i64 } else { 0i64 }).write_redis_args(out)
            }
            RedisCrateValue::Okay => "OK".write_redis_args(out),
            RedisCrateValue::BigNumber(ref big_number) => {
                big_number.to_string().write_redis_args(out)
            }
            RedisCrateValue::Nil => {
                // Nil cannot be meaningfully represented as a Redis command argument.
                // Writing empty bytes would turn "null" into "empty string", which
                // are semantically different. Skip the write so Redis sees a missing
                // argument rather than an empty one.
                tracing::warn!("Attempted to write Nil as a Redis command argument — skipping");
            }
            // Aggregate and error types cannot be serialized as a single Redis argument.
            // These variants are only expected in Redis *responses*, not as command
            // arguments. Writing empty bytes would silently corrupt data.
            RedisCrateValue::Array(_)
            | RedisCrateValue::Map(_)
            | RedisCrateValue::Set(_)
            | RedisCrateValue::Attribute { .. }
            | RedisCrateValue::Push { .. }
            | RedisCrateValue::ServerError(_) => {
                tracing::warn!(
                    variant = ?self.inner,
                    "Attempted to write an aggregate/error Redis value as a command argument — skipping. \
                     Aggregate types (Array, Map, Set, etc.) should not be used as command arguments."
                );
            }
            // Catch-all for future variants added to the non-exhaustive enum
            _ => {
                tracing::warn!(
                    variant = ?self.inner,
                    "Attempted to write an unknown Redis value as a command argument — skipping"
                );
            }
        }
    }
}

impl redis::ToSingleRedisArg for RedisValue {}

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
    pub fn max_in_flight_commands_as_usize(&self) -> Option<usize> {
        (self.max_in_flight_commands > 0).then_some(self.max_in_flight_commands)
    }

    /// Convert `reconnect_max_attempts` to `usize`.
    ///
    /// Logs a warning and falls back to [`constant::DEFAULT_RECONNECT_MAX_ATTEMPTS`]
    /// when the value overflows.
    pub fn reconnect_max_attempts_as_usize(&self) -> usize {
        usize::try_from(self.reconnect_max_attempts).unwrap_or_else(|_| {
            tracing::warn!(
                "reconnect_max_attempts ({}) exceeds usize, using default ({})",
                self.reconnect_max_attempts,
                constant::DEFAULT_RECONNECT_MAX_ATTEMPTS
            );
            constant::DEFAULT_RECONNECT_MAX_ATTEMPTS
        })
    }

    /// Build a standalone [`redis::ConnectionInfo`] with RESP3 protocol from host and port.
    pub fn build_standalone_connection_info(
        &self,
    ) -> CustomResult<redis::ConnectionInfo, errors::RedisError> {
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

impl redis::ToRedisArgs for RedisEntryId {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.to_stream_id().write_redis_args(out)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SetnxReply {
    KeySet,
    KeyNotSet, // Existing key
}

impl redis::FromRedisValue for SetnxReply {
    fn from_redis_value(value: Value) -> Result<Self, redis::ParsingError> {
        match value {
            // SET NX returns Okay on success
            Value::Okay => Ok(Self::KeySet),
            // Returns Nil if key already exists
            Value::Nil => Ok(Self::KeyNotSet),
            _ => {
                tracing::error!(received = ?value, "Unexpected SETNX command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected SETNX command reply: {:?}",
                    value
                )))
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum HsetnxReply {
    KeySet,
    KeyNotSet, // Existing key
}

impl redis::FromRedisValue for HsetnxReply {
    fn from_redis_value(value: Value) -> Result<Self, redis::ParsingError> {
        match value {
            Value::Int(1) => Ok(Self::KeySet),
            Value::Int(0) => Ok(Self::KeyNotSet),
            _ => {
                tracing::error!(received = ?value, "Unexpected HSETNX command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected HSETNX command reply: {:?}",
                    value
                )))
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MsetnxReply {
    KeysSet,
    KeysNotSet, // At least one existing key
}

impl redis::FromRedisValue for MsetnxReply {
    fn from_redis_value(value: Value) -> Result<Self, redis::ParsingError> {
        match value {
            Value::Int(1) => Ok(Self::KeysSet),
            Value::Int(0) => Ok(Self::KeysNotSet),
            _ => {
                tracing::error!(received = ?value, "Unexpected MSETNX command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected MSETNX command reply: {:?}",
                    value
                )))
            }
        }
    }
}

/// Converts a `redis::Value` to `Option<String>`.
///
/// - String-like variants (`BulkString`, `SimpleString`, `VerbatimString`) → decoded string
/// - Scalar variants (`Int`, `Double`, `Boolean`, `Okay`, `BigNumber`) → string representation
/// - `Nil` and aggregate types (`Array`, `Map`, `Set`, `Attribute`, `Push`, `ServerError`) → `None`
pub fn redis_value_to_option_string(value: &Value) -> Option<String> {
    match value {
        Value::BulkString(bytes) => std::str::from_utf8(bytes)
            .ok()
            .map(|utf8_str| utf8_str.to_string()),
        Value::SimpleString(string) => Some(string.clone()),
        Value::VerbatimString { text, .. } => Some(text.clone()),
        Value::Int(integer) => Some(integer.to_string()),
        Value::Double(double) => Some(double.to_string()),
        Value::Boolean(boolean) => Some(boolean.to_string()),
        Value::Okay => Some("OK".to_string()),
        Value::BigNumber(ref big_number) => Some(big_number.to_string()),
        // Nil and aggregate types have no meaningful single-string representation
        Value::Nil
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Attribute { .. }
        | Value::Push { .. }
        | Value::ServerError(_) => None,
        // Catch-all for future variants added to the non-exhaustive enum
        _ => None,
    }
}

/// Converts a stream entry's field map (`HashMap<String, Value>`) into
/// `HashMap<String, Option<String>>`, preserving `Nil` as `None`.
pub fn stream_fields_to_option_strings(
    fields: std::collections::HashMap<String, Value>,
) -> std::collections::HashMap<String, Option<String>> {
    fields
        .into_iter()
        .map(|(field_name, redis_value)| (field_name, redis_value_to_option_string(&redis_value)))
        .collect()
}

/// Entries within a single stream, as `(entry_id, fields)`.
pub type StreamEntries = Vec<(String, std::collections::HashMap<String, String>)>;

/// Grouped result of a stream read: stream key → list of `(entry_id, fields)`.
pub type StreamReadResult = std::collections::HashMap<String, StreamEntries>;

/// Whether to trim by entry count or by minimum entry ID.
#[derive(Debug, Clone, Copy)]
pub enum StreamCapKind {
    MinID,
    MaxLen,
}

#[derive(Debug, Clone, Copy)]
pub enum StreamCapTrim {
    Exact,
    AlmostExact,
}

/// Configuration for a stream trim (`XTRIM`) operation.
///
/// Bundles the kind (MaxLen vs MinID), trim precision (Exact vs Approx),
/// and the threshold value into a single cohesive unit.
///
/// # Examples
///
/// ```
/// use redis_interface::types::{StreamCapKind, StreamCapTrim, StreamTrimConfig};
///
/// let config = StreamTrimConfig::new(StreamCapKind::MaxLen, StreamCapTrim::AlmostExact, "1000");
/// let options = config.to_trim_options().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct StreamTrimConfig {
    /// What to compare against — entry count or minimum ID.
    pub kind: StreamCapKind,
    /// How precisely to match the threshold.
    pub trim: StreamCapTrim,
    /// The threshold value: a numeric string for `MaxLen`, a stream ID for `MinID`.
    pub threshold: String,
}

impl StreamTrimConfig {
    /// Create a new trim configuration.
    pub fn new(kind: StreamCapKind, trim: StreamCapTrim, threshold: impl Into<String>) -> Self {
        Self {
            kind,
            trim,
            threshold: threshold.into(),
        }
    }

    /// Convert to the `redis` crate's `StreamTrimOptions`.
    ///
    /// Returns an error if `kind` is `MaxLen` and the threshold
    /// cannot be parsed as a `usize`.
    pub fn to_trim_options(
        self,
    ) -> Result<redis::streams::StreamTrimOptions, StreamTrimThresholdError> {
        let trim_mode = match self.trim {
            StreamCapTrim::Exact => redis::streams::StreamTrimmingMode::Exact,
            StreamCapTrim::AlmostExact => redis::streams::StreamTrimmingMode::Approx,
        };

        match self.kind {
            StreamCapKind::MaxLen => {
                let max_len: usize = self
                    .threshold
                    .parse()
                    .map_err(|_| StreamTrimThresholdError(self.threshold.clone()))?;
                Ok(redis::streams::StreamTrimOptions::maxlen(
                    trim_mode, max_len,
                ))
            }
            StreamCapKind::MinID => Ok(redis::streams::StreamTrimOptions::minid(
                trim_mode,
                self.threshold,
            )),
        }
    }
}

/// Error returned when a stream trim threshold cannot be parsed as a number.
#[derive(Debug)]
pub struct StreamTrimThresholdError(String);

impl std::fmt::Display for StreamTrimThresholdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Invalid stream trim threshold: {}", self.0)
    }
}

impl std::error::Error for StreamTrimThresholdError {}

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

impl redis::FromRedisValue for DelReply {
    fn from_redis_value(value: Value) -> Result<Self, redis::ParsingError> {
        match value {
            Value::Int(1) => Ok(Self::KeyDeleted),
            Value::Int(0) => Ok(Self::KeyNotDeleted),
            _ => {
                tracing::error!(received = ?value, "Unexpected DEL command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected DEL command reply: {:?}",
                    value
                )))
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SaddReply {
    KeySet,
    KeyNotSet,
}

impl redis::FromRedisValue for SaddReply {
    fn from_redis_value(value: Value) -> Result<Self, redis::ParsingError> {
        match value {
            Value::Int(1) => Ok(Self::KeySet),
            Value::Int(0) => Ok(Self::KeyNotSet),
            _ => {
                tracing::error!(received = ?value, "Unexpected SADD command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected SADD command reply: {:?}",
                    value
                )))
            }
        }
    }
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

#[derive(Debug, Clone)]
pub struct RedisKey(String);

impl RedisKey {
    pub fn tenant_aware_key(&self, pool: &RedisConnectionPool) -> String {
        pool.add_prefix(&self.0)
    }

    pub fn tenant_unaware_key(&self, _pool: &RedisConnectionPool) -> String {
        self.0.clone()
    }
}

impl<T: AsRef<str>> From<T> for RedisKey {
    fn from(value: T) -> Self {
        let value = value.as_ref();

        Self(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use redis::FromRedisValue;

    use super::*;

    // ── redis_value_to_option_string ───────────────────────────────────────

    #[test]
    fn test_redis_value_bulk_string_valid_utf8() {
        let value = Value::BulkString("hello".as_bytes().to_vec());
        assert_eq!(
            redis_value_to_option_string(&value),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_redis_value_bulk_string_invalid_utf8() {
        let value = Value::BulkString(vec![0xff, 0xfe]);
        assert_eq!(redis_value_to_option_string(&value), None);
    }

    #[test]
    fn test_redis_value_simple_string() {
        let value = Value::SimpleString("OK".to_string());
        assert_eq!(redis_value_to_option_string(&value), Some("OK".to_string()));
    }

    #[test]
    fn test_redis_value_int() {
        let value = Value::Int(42);
        assert_eq!(redis_value_to_option_string(&value), Some("42".to_string()));
    }

    #[test]
    fn test_redis_value_nil() {
        let value = Value::Nil;
        assert_eq!(redis_value_to_option_string(&value), None);
    }

    #[test]
    fn test_redis_value_okay() {
        let value = Value::Okay;
        assert_eq!(redis_value_to_option_string(&value), Some("OK".to_string()));
    }

    #[test]
    fn test_redis_value_double() {
        let value = Value::Double(2.5);
        assert_eq!(
            redis_value_to_option_string(&value),
            Some("2.5".to_string())
        );
    }

    #[test]
    fn test_redis_value_boolean() {
        let value = Value::Boolean(true);
        assert_eq!(
            redis_value_to_option_string(&value),
            Some("true".to_string())
        );
    }

    #[test]
    fn test_redis_value_verbatim_string() {
        let value = Value::VerbatimString {
            format: redis::VerbatimFormat::Text,
            text: "raw text".to_string(),
        };
        assert_eq!(
            redis_value_to_option_string(&value),
            Some("raw text".to_string())
        );
    }

    #[test]
    fn test_redis_value_array() {
        let value = Value::Array(vec![]);
        assert_eq!(redis_value_to_option_string(&value), None);
    }

    // ── stream_fields_to_option_strings ───────────────────────────────────

    #[test]
    fn test_stream_fields_all_string_values() {
        let fields = std::collections::HashMap::from([
            (
                "name".to_string(),
                Value::BulkString("test".as_bytes().to_vec()),
            ),
            ("count".to_string(), Value::Int(5)),
        ]);
        let result = stream_fields_to_option_strings(fields);
        assert_eq!(result.get("name").unwrap(), &Some("test".to_string()));
        assert_eq!(result.get("count").unwrap(), &Some("5".to_string()));
    }

    #[test]
    fn test_stream_fields_nil_preserved_as_none() {
        let fields = std::collections::HashMap::from([
            (
                "present".to_string(),
                Value::BulkString("value".as_bytes().to_vec()),
            ),
            ("absent".to_string(), Value::Nil),
        ]);
        let result = stream_fields_to_option_strings(fields);
        assert_eq!(result.get("present").unwrap(), &Some("value".to_string()));
        assert!(result.get("absent").unwrap().is_none());
    }

    #[test]
    fn test_stream_fields_invalid_utf8_becomes_none() {
        let fields = std::collections::HashMap::from([(
            "bad".to_string(),
            Value::BulkString(vec![0xff, 0xfe]),
        )]);
        let result = stream_fields_to_option_strings(fields);
        assert!(result.get("bad").unwrap().is_none());
    }

    #[test]
    fn test_stream_fields_empty_map() {
        let fields = std::collections::HashMap::new();
        let result = stream_fields_to_option_strings(fields);
        assert!(result.is_empty());
    }

    // ── SetnxReply::from_redis_value ──────────────────────────────────────

    #[test]
    fn test_setnx_reply_okay() {
        let reply = SetnxReply::from_redis_value(Value::Okay);
        assert_eq!(reply.unwrap(), SetnxReply::KeySet);
    }

    #[test]
    fn test_setnx_reply_nil() {
        let reply = SetnxReply::from_redis_value(Value::Nil);
        assert_eq!(reply.unwrap(), SetnxReply::KeyNotSet);
    }

    #[test]
    fn test_setnx_reply_unexpected_value() {
        let reply = SetnxReply::from_redis_value(Value::Int(99));
        assert!(reply.is_err());
    }

    // ── DelReply::from_redis_value ────────────────────────────────────────

    #[test]
    fn test_del_reply_one() {
        let reply = DelReply::from_redis_value(Value::Int(1));
        assert_eq!(reply.unwrap(), DelReply::KeyDeleted);
    }

    #[test]
    fn test_del_reply_zero() {
        let reply = DelReply::from_redis_value(Value::Int(0));
        assert_eq!(reply.unwrap(), DelReply::KeyNotDeleted);
    }

    #[test]
    fn test_del_reply_unexpected_value() {
        let reply = DelReply::from_redis_value(Value::Nil);
        assert!(reply.is_err());
    }

    // ── HsetnxReply::from_redis_value ──────────────────────────────────────

    #[test]
    fn test_hsetnx_reply_key_set() {
        let reply = HsetnxReply::from_redis_value(Value::Int(1));
        assert_eq!(reply.unwrap(), HsetnxReply::KeySet);
    }

    #[test]
    fn test_hsetnx_reply_key_not_set() {
        let reply = HsetnxReply::from_redis_value(Value::Int(0));
        assert_eq!(reply.unwrap(), HsetnxReply::KeyNotSet);
    }

    #[test]
    fn test_hsetnx_reply_unexpected_value() {
        let reply = HsetnxReply::from_redis_value(Value::Nil);
        assert!(reply.is_err());
    }

    // ── MsetnxReply::from_redis_value ──────────────────────────────────────

    #[test]
    fn test_msetnx_reply_keys_set() {
        let reply = MsetnxReply::from_redis_value(Value::Int(1));
        assert_eq!(reply.unwrap(), MsetnxReply::KeysSet);
    }

    #[test]
    fn test_msetnx_reply_keys_not_set() {
        let reply = MsetnxReply::from_redis_value(Value::Int(0));
        assert_eq!(reply.unwrap(), MsetnxReply::KeysNotSet);
    }

    #[test]
    fn test_msetnx_reply_unexpected_value() {
        let reply = MsetnxReply::from_redis_value(Value::Nil);
        assert!(reply.is_err());
    }

    // ── SaddReply::from_redis_value ────────────────────────────────────────

    #[test]
    fn test_sadd_reply_key_set() {
        let reply = SaddReply::from_redis_value(Value::Int(1));
        assert_eq!(reply.unwrap(), SaddReply::KeySet);
    }

    #[test]
    fn test_sadd_reply_key_not_set() {
        let reply = SaddReply::from_redis_value(Value::Int(0));
        assert_eq!(reply.unwrap(), SaddReply::KeyNotSet);
    }

    #[test]
    fn test_sadd_reply_unexpected_value() {
        let reply = SaddReply::from_redis_value(Value::Nil);
        assert!(reply.is_err());
    }

    // ── RedisEntryId::to_stream_id ────────────────────────────────────────

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

    // ── DelReply helper methods ────────────────────────────────────────────

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

    // ── RedisValue constructors and accessors ──────────────────────────────

    #[test]
    fn test_redis_value_new_and_into_inner() {
        let inner = Value::Int(42);
        let rv = RedisValue::new(inner.clone());
        assert_eq!(*rv, inner);
        assert_eq!(rv.into_inner(), inner);
    }

    #[test]
    fn test_redis_value_from_bytes() {
        let rv = RedisValue::from_bytes(b"hello".to_vec());
        assert_eq!(rv.as_bytes(), Some(&b"hello"[..]));
        assert_eq!(rv.as_string(), Some("hello".to_string()));
    }

    #[test]
    fn test_redis_value_from_string() {
        let rv = RedisValue::from_string("world".to_string());
        assert_eq!(rv.as_string(), Some("world".to_string()));
        assert_eq!(rv.as_bytes(), Some(b"world".as_slice()));
    }

    #[test]
    fn test_redis_value_as_bytes_non_string() {
        let rv = RedisValue::new(Value::Int(7));
        assert!(rv.as_bytes().is_none());
    }

    #[test]
    fn test_redis_value_as_string_non_string() {
        let rv = RedisValue::new(Value::Int(7));
        assert_eq!(rv.as_string(), Some("7".to_string()));
    }

    // ── RedisSettings::validate ────────────────────────────────────────────

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
        let result = settings.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_redis_settings_validate_cluster_without_urls() {
        let settings = RedisSettings {
            cluster_enabled: true,
            cluster_urls: vec![],
            ..RedisSettings::default()
        };
        let result = settings.validate();
        assert!(result.is_err());
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
        let result = settings.validate();
        assert!(result.is_err());
    }

    // ── RedisSettings::Default ─────────────────────────────────────────────

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

    // ── RedisValue::ToRedisArgs ────────────────────────────────────────────

    #[test]
    fn test_redis_value_to_redis_args_bulk_string() {
        let value = RedisValue::from_bytes(b"hello".to_vec());
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"hello".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_simple_string() {
        let value = RedisValue::from_string("world".to_string());
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"world".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_int() {
        let value = RedisValue::new(Value::Int(42));
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"42".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_nil_writes_nothing() {
        let value = RedisValue::new(Value::Nil);
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        // Nil should not write any bytes — empty args, not empty byte slice
        assert!(args.is_empty());
    }

    #[test]
    fn test_redis_value_to_redis_args_array_writes_nothing() {
        let value = RedisValue::new(Value::Array(vec![Value::BulkString(b"hello".to_vec())]));
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        // Aggregate types should not write any bytes
        assert!(args.is_empty());
    }

    // ── RedisEntryId::ToRedisArgs ──────────────────────────────────────────

    #[test]
    fn test_entry_id_to_redis_args_user_specified() {
        let id = RedisEntryId::UserSpecifiedID {
            milliseconds: "1234567890".to_string(),
            sequence_number: "0".to_string(),
        };
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&id, &mut args);
        assert_eq!(args, vec![b"1234567890-0".as_slice()]);
    }

    #[test]
    fn test_entry_id_to_redis_args_auto_generated() {
        let id = RedisEntryId::AutoGeneratedID;
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&id, &mut args);
        assert_eq!(args, vec![b"*".as_slice()]);
    }

    #[test]
    fn test_entry_id_to_redis_args_after_last() {
        let id = RedisEntryId::AfterLastID;
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&id, &mut args);
        assert_eq!(args, vec![b"$".as_slice()]);
    }

    #[test]
    fn test_entry_id_to_redis_args_undelivered() {
        let id = RedisEntryId::UndeliveredEntryID;
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&id, &mut args);
        assert_eq!(args, vec![b">".as_slice()]);
    }

    // ── SetGetReply::get_value ─────────────────────────────────────────────

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

    // ── RedisKey ───────────────────────────────────────────────────────────

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
}
