//! Data types and type conversions
//! from backend-internal data-types to custom data-types.
//!
//! All types in this module compile under both `redis-rs` and `fred-rs` features.
//! Backend-specific trait implementations are gated with `#[cfg]`.

use common_utils::errors::CustomResult;

use crate::errors;

// ─── RedisValue — backend-agnostic wrapper ──────────────────────────────────

#[derive(Clone, Debug)]
pub struct RedisValue {
    #[cfg(feature = "redis-rs")]
    inner: redis::Value,
    #[cfg(feature = "fred-rs")]
    inner: fred::types::RedisValue,
}

impl RedisValue {
    #[cfg(feature = "redis-rs")]
    pub fn new(value: redis::Value) -> Self {
        Self { inner: value }
    }
    #[cfg(feature = "fred-rs")]
    pub fn new(value: fred::types::RedisValue) -> Self {
        Self { inner: value }
    }

    #[cfg(feature = "redis-rs")]
    pub fn from_bytes(val: Vec<u8>) -> Self {
        Self {
            inner: redis::Value::BulkString(val),
        }
    }
    #[cfg(feature = "fred-rs")]
    pub fn from_bytes(val: Vec<u8>) -> Self {
        Self {
            inner: fred::types::RedisValue::Bytes(val.into()),
        }
    }

    #[cfg(feature = "redis-rs")]
    pub fn from_string(value: String) -> Self {
        Self {
            inner: redis::Value::SimpleString(value),
        }
    }
    #[cfg(feature = "fred-rs")]
    pub fn from_string(value: String) -> Self {
        Self {
            inner: fred::types::RedisValue::String(value.into()),
        }
    }

    /// Extract bytes from the underlying redis value
    #[cfg(feature = "redis-rs")]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            redis::Value::BulkString(bytes) => Some(bytes.as_slice()),
            redis::Value::SimpleString(s) => Some(s.as_bytes()),
            _ => None,
        }
    }
    #[cfg(feature = "fred-rs")]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            fred::types::RedisValue::Bytes(b) => Some(b.as_ref()),
            fred::types::RedisValue::String(s) => Some(s.as_bytes()),
            _ => None,
        }
    }

    /// Convert to string if the value is a string type
    #[cfg(feature = "redis-rs")]
    pub fn as_string(&self) -> Option<String> {
        match &self.inner {
            redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone()).ok(),
            redis::Value::SimpleString(s) => Some(s.clone()),
            _ => None,
        }
    }
    #[cfg(feature = "fred-rs")]
    pub fn as_string(&self) -> Option<String> {
        match &self.inner {
            fred::types::RedisValue::String(s) => Some(s.to_string()),
            fred::types::RedisValue::Bytes(b) => String::from_utf8(b.to_vec()).ok(),
            _ => None,
        }
    }

    #[cfg(feature = "redis-rs")]
    pub fn into_inner(self) -> redis::Value {
        self.inner
    }
    #[cfg(feature = "fred-rs")]
    pub fn into_inner(self) -> fred::types::RedisValue {
        self.inner
    }
}

// Deref
#[cfg(feature = "redis-rs")]
impl std::ops::Deref for RedisValue {
    type Target = redis::Value;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
#[cfg(feature = "fred-rs")]
impl std::ops::Deref for RedisValue {
    type Target = fred::types::RedisValue;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// From<RedisValue> → backend value
#[cfg(feature = "redis-rs")]
impl From<RedisValue> for redis::Value {
    fn from(v: RedisValue) -> Self {
        v.inner
    }
}
#[cfg(feature = "fred-rs")]
impl From<RedisValue> for fred::types::RedisValue {
    fn from(v: RedisValue) -> Self {
        v.inner
    }
}

// ToRedisArgs — redis-rs only
#[cfg(feature = "redis-rs")]
impl redis::ToRedisArgs for RedisValue {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        match &self.inner {
            redis::Value::BulkString(bytes) => bytes.write_redis_args(out),
            redis::Value::SimpleString(s) => s.write_redis_args(out),
            _ => {
                // Fallback: serialize as empty bytes
                Vec::<u8>::new().write_redis_args(out)
            }
        }
    }
}
#[cfg(feature = "redis-rs")]
impl redis::ToSingleRedisArg for RedisValue {}

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
    /// Passed to `ConnectionManagerConfig::set_response_timeout` indirectly via command timeout.
    pub max_in_flight_commands: u64,
    /// Command timeout in seconds. Passed to `ConnectionManagerConfig::set_response_timeout`.
    pub default_command_timeout: u64,
    pub max_feed_count: u64,
    pub unresponsive_timeout: u64,
    pub unresponsive_check_interval: u64,
    /// Capacity of the broadcast channel used for pub/sub message distribution.
    pub broadcast_channel_capacity: usize,
    /// Maximum duration (in seconds) that Redis can be unreachable before the server shuts down.
    pub max_failure_threshold: u32,
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
                )
                .into())
            },
        )
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
            max_failure_threshold: 5,
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

// redis-rs: convert to string for ToRedisArgs
#[cfg(feature = "redis-rs")]
impl redis::ToRedisArgs for RedisEntryId {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.to_stream_id().write_redis_args(out)
    }
}

// fred-rs: convert to XID
#[cfg(feature = "fred-rs")]
impl From<RedisEntryId> for fred::types::XID {
    fn from(id: RedisEntryId) -> Self {
        match id {
            RedisEntryId::UserSpecifiedID {
                milliseconds,
                sequence_number,
            } => Self::Manual(fred::bytes_utils::format_bytes!(
                "{milliseconds}-{sequence_number}"
            )),
            RedisEntryId::AutoGeneratedID => Self::Auto,
            RedisEntryId::AfterLastID => Self::Max,
            RedisEntryId::UndeliveredEntryID => Self::NewInGroup,
        }
    }
}

#[cfg(feature = "fred-rs")]
impl From<&RedisEntryId> for fred::types::XID {
    fn from(id: &RedisEntryId) -> Self {
        match id {
            RedisEntryId::UserSpecifiedID {
                milliseconds,
                sequence_number,
            } => Self::Manual(fred::bytes_utils::format_bytes!(
                "{milliseconds}-{sequence_number}"
            )),
            RedisEntryId::AutoGeneratedID => Self::Auto,
            RedisEntryId::AfterLastID => Self::Max,
            RedisEntryId::UndeliveredEntryID => Self::NewInGroup,
        }
    }
}

// ─── Reply type enums ────────────────────────────────────────────────────────

#[derive(Debug, Eq, PartialEq)]
pub enum SetnxReply {
    KeySet,
    KeyNotSet, // Existing key
}

#[cfg(feature = "redis-rs")]
impl redis::FromRedisValue for SetnxReply {
    fn from_redis_value(v: redis::Value) -> Result<Self, redis::ParsingError> {
        match v {
            // SET NX returns Okay on success (newer redis crate)
            redis::Value::Okay => Ok(Self::KeySet),
            // SET NX returns "OK" on success (older format)
            redis::Value::SimpleString(ref s) if s == "OK" => Ok(Self::KeySet),
            redis::Value::BulkString(ref s) if s == b"OK" => Ok(Self::KeySet),
            // Returns Nil if key already exists
            redis::Value::Nil => Ok(Self::KeyNotSet),
            _ => {
                tracing::error!(received = ?v, "Unexpected SETNX command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected SETNX command reply: {:?}",
                    v
                )))
            }
        }
    }
}

#[cfg(feature = "fred-rs")]
impl fred::types::FromRedis for SetnxReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::String(_) => Ok(Self::KeySet),
            fred::types::RedisValue::Null => Ok(Self::KeyNotSet),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected SETNX reply",
            )),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum HsetnxReply {
    KeySet,
    KeyNotSet, // Existing key
}

#[cfg(feature = "redis-rs")]
impl redis::FromRedisValue for HsetnxReply {
    fn from_redis_value(v: redis::Value) -> Result<Self, redis::ParsingError> {
        match v {
            redis::Value::Int(1) => Ok(Self::KeySet),
            redis::Value::Int(0) => Ok(Self::KeyNotSet),
            _ => {
                tracing::error!(received = ?v, "Unexpected HSETNX command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected HSETNX command reply: {:?}",
                    v
                )))
            }
        }
    }
}

#[cfg(feature = "fred-rs")]
impl fred::types::FromRedis for HsetnxReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::Integer(1) => Ok(Self::KeySet),
            fred::types::RedisValue::Integer(0) => Ok(Self::KeyNotSet),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected HSETNX reply",
            )),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MsetnxReply {
    KeysSet,
    KeysNotSet, // At least one existing key
}

#[cfg(feature = "redis-rs")]
impl redis::FromRedisValue for MsetnxReply {
    fn from_redis_value(v: redis::Value) -> Result<Self, redis::ParsingError> {
        match v {
            redis::Value::Int(1) => Ok(Self::KeysSet),
            redis::Value::Int(0) => Ok(Self::KeysNotSet),
            _ => {
                tracing::error!(received = ?v, "Unexpected MSETNX command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected MSETNX command reply: {:?}",
                    v
                )))
            }
        }
    }
}

#[cfg(feature = "fred-rs")]
impl fred::types::FromRedis for MsetnxReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::Integer(1) => Ok(Self::KeysSet),
            fred::types::RedisValue::Integer(0) => Ok(Self::KeysNotSet),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected MSETNX reply",
            )),
        }
    }
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

#[cfg(feature = "redis-rs")]
impl redis::FromRedisValue for DelReply {
    fn from_redis_value(v: redis::Value) -> Result<Self, redis::ParsingError> {
        match v {
            redis::Value::Int(1) => Ok(Self::KeyDeleted),
            redis::Value::Int(0) => Ok(Self::KeyNotDeleted),
            _ => {
                tracing::error!(received = ?v, "Unexpected DEL command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected DEL command reply: {:?}",
                    v
                )))
            }
        }
    }
}

#[cfg(feature = "fred-rs")]
impl fred::types::FromRedis for DelReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::Integer(1) => Ok(Self::KeyDeleted),
            fred::types::RedisValue::Integer(0) => Ok(Self::KeyNotDeleted),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected DEL reply",
            )),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SaddReply {
    KeySet,
    KeyNotSet,
}

#[cfg(feature = "redis-rs")]
impl redis::FromRedisValue for SaddReply {
    fn from_redis_value(v: redis::Value) -> Result<Self, redis::ParsingError> {
        match v {
            redis::Value::Int(1) => Ok(Self::KeySet),
            redis::Value::Int(0) => Ok(Self::KeyNotSet),
            _ => {
                tracing::error!(received = ?v, "Unexpected SADD command reply from Redis");
                Err(redis::ParsingError::from(format!(
                    "Unexpected SADD command reply: {:?}",
                    v
                )))
            }
        }
    }
}

#[cfg(feature = "fred-rs")]
impl fred::types::FromRedis for SaddReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::Integer(1) => Ok(Self::KeySet),
            fred::types::RedisValue::Integer(0) => Ok(Self::KeyNotSet),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected SADD reply",
            )),
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

// fred-rs: StreamCapKind → XCapKind
#[cfg(feature = "fred-rs")]
impl From<StreamCapKind> for fred::types::XCapKind {
    fn from(item: StreamCapKind) -> Self {
        match item {
            StreamCapKind::MaxLen => Self::MaxLen,
            StreamCapKind::MinID => Self::MinID,
        }
    }
}

// fred-rs: StreamCapTrim → XCapTrim
#[cfg(feature = "fred-rs")]
impl From<StreamCapTrim> for fred::types::XCapTrim {
    fn from(item: StreamCapTrim) -> Self {
        match item {
            StreamCapTrim::Exact => Self::Exact,
            StreamCapTrim::AlmostExact => Self::AlmostExact,
        }
    }
}

// ─── Redis-rs-specific helpers ───────────────────────────────────────────────

/// Converts a `redis::Value` to `Option<String>`.
///
/// - `BulkString` → decoded as UTF-8, returns `None` if invalid
/// - `SimpleString` → `Some(s)`
/// - `Int` → `Some(i.to_string())`
/// - `Nil` / other variants → `None`
#[cfg(feature = "redis-rs")]
pub fn redis_value_to_option_string(v: &redis::Value) -> Option<String> {
    match v {
        redis::Value::BulkString(bytes) => std::str::from_utf8(bytes)
            .ok()
            .map(|utf8_str| utf8_str.to_string()),
        redis::Value::SimpleString(s) => Some(s.clone()),
        redis::Value::Int(i) => Some(i.to_string()),
        _ => None,
    }
}

/// Converts a stream entry's field map (`HashMap<String, redis::Value>`) into
/// `HashMap<String, Option<String>>`, preserving `Nil` as `None`.
#[cfg(feature = "redis-rs")]
pub fn stream_fields_to_option_strings(
    fields: std::collections::HashMap<String, redis::Value>,
) -> std::collections::HashMap<String, Option<String>> {
    fields
        .into_iter()
        .map(|(field_name, redis_value)| (field_name, redis_value_to_option_string(&redis_value)))
        .collect()
}

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

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(settings.max_failure_threshold, 5);
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

    // ── Backend-specific tests ─────────────────────────────────────────────

    #[cfg(feature = "redis-rs")]
    mod redis_rs_tests {
        use super::*;
        use redis::FromRedisValue;

        // ── redis_value_to_option_string ───────────────────────────────────

        #[test]
        fn test_redis_value_bulk_string_valid_utf8() {
            let value = redis::Value::BulkString("hello".as_bytes().to_vec());
            assert_eq!(
                redis_value_to_option_string(&value),
                Some("hello".to_string())
            );
        }

        #[test]
        fn test_redis_value_bulk_string_invalid_utf8() {
            let value = redis::Value::BulkString(vec![0xff, 0xfe]);
            assert_eq!(redis_value_to_option_string(&value), None);
        }

        #[test]
        fn test_redis_value_simple_string() {
            let value = redis::Value::SimpleString("OK".to_string());
            assert_eq!(redis_value_to_option_string(&value), Some("OK".to_string()));
        }

        #[test]
        fn test_redis_value_int() {
            let value = redis::Value::Int(42);
            assert_eq!(redis_value_to_option_string(&value), Some("42".to_string()));
        }

        #[test]
        fn test_redis_value_nil() {
            let value = redis::Value::Nil;
            assert_eq!(redis_value_to_option_string(&value), None);
        }

        #[test]
        fn test_redis_value_okay() {
            let value = redis::Value::Okay;
            assert_eq!(redis_value_to_option_string(&value), None);
        }

        #[test]
        fn test_redis_value_array() {
            let value = redis::Value::Array(vec![]);
            assert_eq!(redis_value_to_option_string(&value), None);
        }

        // ── stream_fields_to_option_strings ─────────────────────────────────

        #[test]
        fn test_stream_fields_all_string_values() {
            let fields = std::collections::HashMap::from([
                (
                    "name".to_string(),
                    redis::Value::BulkString("test".as_bytes().to_vec()),
                ),
                ("count".to_string(), redis::Value::Int(5)),
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
                    redis::Value::BulkString("value".as_bytes().to_vec()),
                ),
                ("absent".to_string(), redis::Value::Nil),
            ]);
            let result = stream_fields_to_option_strings(fields);
            assert_eq!(result.get("present").unwrap(), &Some("value".to_string()));
            assert!(result.get("absent").unwrap().is_none());
        }

        #[test]
        fn test_stream_fields_invalid_utf8_becomes_none() {
            let fields = std::collections::HashMap::from([(
                "bad".to_string(),
                redis::Value::BulkString(vec![0xff, 0xfe]),
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

        // ── SetnxReply::from_redis_value ────────────────────────────────────

        #[test]
        fn test_setnx_reply_okay() {
            let reply = SetnxReply::from_redis_value(redis::Value::Okay);
            assert_eq!(reply.unwrap(), SetnxReply::KeySet);
        }

        #[test]
        fn test_setnx_reply_simple_string_ok() {
            let reply = SetnxReply::from_redis_value(redis::Value::SimpleString("OK".to_string()));
            assert_eq!(reply.unwrap(), SetnxReply::KeySet);
        }

        #[test]
        fn test_setnx_reply_bulk_string_ok() {
            let reply = SetnxReply::from_redis_value(redis::Value::BulkString(b"OK".to_vec()));
            assert_eq!(reply.unwrap(), SetnxReply::KeySet);
        }

        #[test]
        fn test_setnx_reply_nil() {
            let reply = SetnxReply::from_redis_value(redis::Value::Nil);
            assert_eq!(reply.unwrap(), SetnxReply::KeyNotSet);
        }

        #[test]
        fn test_setnx_reply_unexpected_value() {
            let reply = SetnxReply::from_redis_value(redis::Value::Int(99));
            assert!(reply.is_err());
        }

        // ── DelReply::from_redis_value ──────────────────────────────────────

        #[test]
        fn test_del_reply_one() {
            let reply = DelReply::from_redis_value(redis::Value::Int(1));
            assert_eq!(reply.unwrap(), DelReply::KeyDeleted);
        }

        #[test]
        fn test_del_reply_zero() {
            let reply = DelReply::from_redis_value(redis::Value::Int(0));
            assert_eq!(reply.unwrap(), DelReply::KeyNotDeleted);
        }

        #[test]
        fn test_del_reply_unexpected_value() {
            let reply = DelReply::from_redis_value(redis::Value::Nil);
            assert!(reply.is_err());
        }

        // ── HsetnxReply::from_redis_value ──────────────────────────────────

        #[test]
        fn test_hsetnx_reply_key_set() {
            let reply = HsetnxReply::from_redis_value(redis::Value::Int(1));
            assert_eq!(reply.unwrap(), HsetnxReply::KeySet);
        }

        #[test]
        fn test_hsetnx_reply_key_not_set() {
            let reply = HsetnxReply::from_redis_value(redis::Value::Int(0));
            assert_eq!(reply.unwrap(), HsetnxReply::KeyNotSet);
        }

        #[test]
        fn test_hsetnx_reply_unexpected_value() {
            let reply = HsetnxReply::from_redis_value(redis::Value::Nil);
            assert!(reply.is_err());
        }

        // ── MsetnxReply::from_redis_value ──────────────────────────────────

        #[test]
        fn test_msetnx_reply_keys_set() {
            let reply = MsetnxReply::from_redis_value(redis::Value::Int(1));
            assert_eq!(reply.unwrap(), MsetnxReply::KeysSet);
        }

        #[test]
        fn test_msetnx_reply_keys_not_set() {
            let reply = MsetnxReply::from_redis_value(redis::Value::Int(0));
            assert_eq!(reply.unwrap(), MsetnxReply::KeysNotSet);
        }

        #[test]
        fn test_msetnx_reply_unexpected_value() {
            let reply = MsetnxReply::from_redis_value(redis::Value::Nil);
            assert!(reply.is_err());
        }

        // ── SaddReply::from_redis_value ────────────────────────────────────

        #[test]
        fn test_sadd_reply_key_set() {
            let reply = SaddReply::from_redis_value(redis::Value::Int(1));
            assert_eq!(reply.unwrap(), SaddReply::KeySet);
        }

        #[test]
        fn test_sadd_reply_key_not_set() {
            let reply = SaddReply::from_redis_value(redis::Value::Int(0));
            assert_eq!(reply.unwrap(), SaddReply::KeyNotSet);
        }

        #[test]
        fn test_sadd_reply_unexpected_value() {
            let reply = SaddReply::from_redis_value(redis::Value::Nil);
            assert!(reply.is_err());
        }

        // ── RedisValue constructors and accessors ──────────────────────────

        #[test]
        fn test_redis_value_new_and_into_inner() {
            let inner = redis::Value::Int(42);
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
            let rv = RedisValue::new(redis::Value::Int(7));
            assert!(rv.as_bytes().is_none());
        }

        #[test]
        fn test_redis_value_as_string_non_string() {
            let rv = RedisValue::new(redis::Value::Int(7));
            assert!(rv.as_string().is_none());
        }

        // ── RedisValue::ToRedisArgs ────────────────────────────────────────

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
        fn test_redis_value_to_redis_args_non_string_fallback() {
            let value = RedisValue::new(redis::Value::Int(42));
            let mut args = Vec::new();
            redis::ToRedisArgs::write_redis_args(&value, &mut args);
            assert_eq!(args, vec![b"".as_slice()]);
        }

        // ── RedisEntryId::ToRedisArgs ──────────────────────────────────────

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

        // ── DelReply helper methods ────────────────────────────────────────

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
}
