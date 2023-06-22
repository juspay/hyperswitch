//!
//! Data types and type conversions
//! from `fred`'s internal data-types to custom data-types
//!

use common_utils::errors::CustomResult;
use error_stack::IntoReport;
use fred::types::RedisValue as FredRedisValue;

use crate::errors;

pub struct RedisValue {
    inner: FredRedisValue,
}

impl std::ops::Deref for RedisValue {
    type Target = FredRedisValue;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RedisValue {
    pub fn new(value: FredRedisValue) -> Self {
        Self { inner: value }
    }
    pub fn into_inner(self) -> FredRedisValue {
        self.inner
    }
    pub fn from_string(value: String) -> Self {
        Self {
            inner: FredRedisValue::String(value.into()),
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
#[serde(default)]
pub struct RedisSettings {
    pub host: String,
    pub port: u16,
    pub cluster_enabled: bool,
    pub cluster_urls: Vec<String>,
    pub use_legacy_version: bool,
    pub pool_size: usize,
    pub reconnect_max_attempts: u32,
    /// Reconnect delay in milliseconds
    pub reconnect_delay: u32,
    /// TTL in seconds
    pub default_ttl: u32,
    /// TTL for hash-tables in seconds
    pub default_hash_ttl: u32,
    pub stream_read_count: u64,
}

impl RedisSettings {
    /// Validates the Redis configuration provided.
    pub fn validate(&self) -> CustomResult<(), errors::RedisError> {
        use common_utils::{ext_traits::ConfigExt, fp_utils::when};

        when(self.host.is_default_or_empty(), || {
            Err(errors::RedisError::InvalidConfiguration(
                "Redis `host` must be specified".into(),
            ))
            .into_report()
        })?;

        when(self.cluster_enabled && self.cluster_urls.is_empty(), || {
            Err(errors::RedisError::InvalidConfiguration(
                "Redis `cluster_urls` must be specified if `cluster_enabled` is `true`".into(),
            ))
            .into_report()
        })
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

#[derive(Eq, PartialEq)]
pub enum SetnxReply {
    KeySet,
    KeyNotSet, // Existing key
}

impl fred::types::FromRedis for SetnxReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            // Returns String ( "OK" ) in case of success
            fred::types::RedisValue::String(_) => Ok(Self::KeySet),
            // Return Null in case of failure
            fred::types::RedisValue::Null => Ok(Self::KeyNotSet),
            // Unexpected behaviour
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected SETNX command reply",
            )),
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum HsetnxReply {
    KeySet,
    KeyNotSet, // Existing key
}

impl fred::types::FromRedis for HsetnxReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::Integer(1) => Ok(Self::KeySet),
            fred::types::RedisValue::Integer(0) => Ok(Self::KeyNotSet),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected HSETNX command reply",
            )),
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum MsetnxReply {
    KeysSet,
    KeysNotSet, // At least one existing key
}

impl fred::types::FromRedis for MsetnxReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::Integer(1) => Ok(Self::KeysSet),
            fred::types::RedisValue::Integer(0) => Ok(Self::KeysNotSet),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected MSETNX command reply",
            )),
        }
    }
}

#[derive(Debug)]
pub enum StreamCapKind {
    MinID,
    MaxLen,
}

impl From<StreamCapKind> for fred::types::XCapKind {
    fn from(item: StreamCapKind) -> Self {
        match item {
            StreamCapKind::MaxLen => Self::MaxLen,
            StreamCapKind::MinID => Self::MinID,
        }
    }
}

#[derive(Debug)]
pub enum StreamCapTrim {
    Exact,
    AlmostExact,
}

impl From<StreamCapTrim> for fred::types::XCapTrim {
    fn from(item: StreamCapTrim) -> Self {
        match item {
            StreamCapTrim::Exact => Self::Exact,
            StreamCapTrim::AlmostExact => Self::AlmostExact,
        }
    }
}

#[derive(Debug)]
pub enum DelReply {
    KeyDeleted,
    KeyNotDeleted, // Key not found
}

impl fred::types::FromRedis for DelReply {
    fn from_value(value: fred::types::RedisValue) -> Result<Self, fred::error::RedisError> {
        match value {
            fred::types::RedisValue::Integer(1) => Ok(Self::KeyDeleted),
            fred::types::RedisValue::Integer(0) => Ok(Self::KeyNotDeleted),
            _ => Err(fred::error::RedisError::new(
                fred::error::RedisErrorKind::Unknown,
                "Unexpected del command reply",
            )),
        }
    }
}
