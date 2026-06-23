//! Redis-rs–specific trait implementations for the shared types.

use crate::types::{
    ConsumerGroupDestroyReply, DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisValue,
    SaddReply, SetnxReply,
};

// ─── RedisValue impls ────────────────────────────────────────────────────────

impl RedisValue {
    pub fn new(value: redis::Value) -> Self {
        Self { inner: value }
    }

    pub fn from_bytes(val: Vec<u8>) -> Self {
        Self {
            inner: redis::Value::BulkString(val),
        }
    }

    pub fn from_string(value: String) -> Self {
        Self {
            inner: redis::Value::SimpleString(value),
        }
    }

    /// Extract bytes from the underlying redis value.
    ///
    /// Returns `Some` for string-like variants (`BulkString`, `SimpleString`,
    /// `VerbatimString`) and `None` for all others.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            redis::Value::BulkString(bytes) => Some(bytes.as_slice()),
            redis::Value::SimpleString(string) => Some(string.as_bytes()),
            redis::Value::VerbatimString { text, .. } => Some(text.as_bytes()),
            other => {
                tracing::debug!(
                    variant = value_variant_name(other),
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
            redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone()).ok(),
            redis::Value::SimpleString(string) => Some(string.clone()),
            redis::Value::VerbatimString { text, .. } => Some(text.clone()),
            redis::Value::Int(integer) => Some(integer.to_string()),
            redis::Value::Double(double) => Some(double.to_string()),
            redis::Value::Boolean(boolean) => Some(boolean.to_string()),
            redis::Value::Okay => Some("OK".to_string()),
            redis::Value::BigNumber(ref big_number) => Some(big_number.to_string()),
            other => {
                tracing::debug!(
                    variant = value_variant_name(other),
                    "as_string() called on non-string RedisValue variant, returning None"
                );
                None
            }
        }
    }
}

impl std::ops::Deref for RedisValue {
    type Target = redis::Value;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<RedisValue> for redis::Value {
    fn from(v: RedisValue) -> Self {
        v.inner
    }
}

impl redis::ToRedisArgs for RedisValue {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        match &self.inner {
            redis::Value::BulkString(bytes) => bytes.write_redis_args(out),
            redis::Value::SimpleString(string) => string.write_redis_args(out),
            redis::Value::VerbatimString { text, .. } => text.write_redis_args(out),
            redis::Value::Int(integer) => integer.write_redis_args(out),
            redis::Value::Double(double) => double.to_string().write_redis_args(out),
            redis::Value::Boolean(boolean) => boolean.write_redis_args(out),
            redis::Value::Okay => "OK".write_redis_args(out),
            redis::Value::BigNumber(ref big_number) => big_number.to_string().write_redis_args(out),
            redis::Value::Nil => {
                // Nil cannot be meaningfully represented as a Redis command argument.
                // Writing empty bytes would turn "null" into "empty string", which
                // are semantically different. Skip the write so Redis sees a missing
                // argument rather than an empty one.
                tracing::warn!("Attempted to write Nil as a Redis command argument — skipping");
            }
            // Aggregate and error types cannot be serialized as a single Redis argument.
            // These variants are only expected in Redis *responses*, not as command
            // arguments. Writing empty bytes would silently corrupt data.
            redis::Value::Array(_)
            | redis::Value::Map(_)
            | redis::Value::Set(_)
            | redis::Value::Attribute { .. }
            | redis::Value::Push { .. }
            | redis::Value::ServerError(_) => {
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

// ─── RedisEntryId ────────────────────────────────────────────────────────────

impl redis::ToRedisArgs for RedisEntryId {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        self.to_stream_id().write_redis_args(out)
    }
}

// ─── Reply type trait impls ──────────────────────────────────────────────────

impl redis::FromRedisValue for SetnxReply {
    fn from_redis_value(v: redis::Value) -> Result<Self, redis::ParsingError> {
        match v {
            // SET NX returns Okay on success
            redis::Value::Okay => Ok(Self::KeySet),
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

impl redis::FromRedisValue for ConsumerGroupDestroyReply {
    fn from_redis_value(value: redis::Value) -> Result<Self, redis::ParsingError> {
        match value {
            redis::Value::Int(1) => Ok(Self::Destroyed),
            redis::Value::Int(0) => Ok(Self::NotFound),
            _ => {
                tracing::error!(
                    received = ?value,
                    "Unexpected XGROUP DESTROY reply from Redis"
                );
                Err(redis::ParsingError::from(format!(
                    "Unexpected XGROUP DESTROY reply: {:?}",
                    value
                )))
            }
        }
    }
}

// ─── Redis-rs-specific helpers ───────────────────────────────────────────────

/// Returns the variant name of a `redis::Value` without any inner data.
/// Used for logging to avoid printing potentially large payloads.
pub(crate) fn value_variant_name(value: &redis::Value) -> &'static str {
    match value {
        redis::Value::Nil => "Nil",
        redis::Value::Int(_) => "Int",
        redis::Value::BulkString(_) => "BulkString",
        redis::Value::Array(_) => "Array",
        redis::Value::Push { .. } => "Push",
        redis::Value::Okay => "Okay",
        redis::Value::SimpleString(_) => "SimpleString",
        redis::Value::Map(_) => "Map",
        redis::Value::Attribute { .. } => "Attribute",
        redis::Value::Set(_) => "Set",
        redis::Value::Double(_) => "Double",
        redis::Value::Boolean(_) => "Boolean",
        redis::Value::VerbatimString { .. } => "VerbatimString",
        redis::Value::BigNumber(_) => "BigNumber",
        redis::Value::ServerError(_) => "ServerError",
        _ => "Unknown",
    }
}

/// Converts a `redis::Value` to `Option<String>`.
///
/// - String-like variants (`BulkString`, `SimpleString`, `VerbatimString`) → decoded string
/// - Scalar variants (`Int`, `Double`, `Boolean`, `Okay`, `BigNumber`) → string representation
/// - `Nil` and aggregate types (`Array`, `Map`, `Set`, `Attribute`, `Push`, `ServerError`) → `None`
pub fn redis_value_to_option_string(v: &redis::Value) -> Option<String> {
    match v {
        redis::Value::BulkString(bytes) => std::str::from_utf8(bytes)
            .ok()
            .map(|utf8_str| utf8_str.to_string()),
        redis::Value::SimpleString(string) => Some(string.clone()),
        redis::Value::VerbatimString { text, .. } => Some(text.clone()),
        redis::Value::Int(integer) => Some(integer.to_string()),
        redis::Value::Double(double) => Some(double.to_string()),
        redis::Value::Boolean(boolean) => Some(boolean.to_string()),
        redis::Value::Okay => Some("OK".to_string()),
        redis::Value::BigNumber(ref big_number) => Some(big_number.to_string()),
        // Nil and aggregate types have no meaningful single-string representation
        redis::Value::Nil
        | redis::Value::Array(_)
        | redis::Value::Map(_)
        | redis::Value::Set(_)
        | redis::Value::Attribute { .. }
        | redis::Value::Push { .. }
        | redis::Value::ServerError(_) => None,
        // Catch-all for future variants added to the non-exhaustive enum
        _ => None,
    }
}

// ─── Backend-specific tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use redis::FromRedisValue;

    use crate::types::{
        DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisValue, SaddReply, SetnxReply,
    };

    /// Critical: Tests that serializing bulk string produces correct bytes
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
        let value = RedisValue::new(redis::Value::Int(42));
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"42".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_double() {
        let value = RedisValue::new(redis::Value::Double(1.5));
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"1.5".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_boolean_true() {
        let value = RedisValue::new(redis::Value::Boolean(true));
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"1".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_boolean_false() {
        let value = RedisValue::new(redis::Value::Boolean(false));
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"0".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_verbatim_string() {
        let value = RedisValue::new(redis::Value::VerbatimString {
            format: redis::VerbatimFormat::Text,
            text: "verbatim".to_string(),
        });
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"verbatim".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_okay() {
        let value = RedisValue::new(redis::Value::Okay);
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert_eq!(args, vec![b"OK".as_slice()]);
    }

    #[test]
    fn test_redis_value_to_redis_args_nil_skips() {
        // Nil should not write any arguments (skip), not empty bytes
        let value = RedisValue::new(redis::Value::Nil);
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert!(
            args.is_empty(),
            "Nil should produce no arguments, got {args:?}"
        );
    }

    #[test]
    fn test_redis_value_to_redis_args_array_skips() {
        // Aggregate types should not write any arguments (skip)
        let value = RedisValue::new(redis::Value::Array(vec![
            redis::Value::Int(1),
            redis::Value::Int(2),
        ]));
        let mut args = Vec::new();
        redis::ToRedisArgs::write_redis_args(&value, &mut args);
        assert!(
            args.is_empty(),
            "Array should produce no arguments, got {args:?}"
        );
    }

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

    #[test]
    fn test_setnx_reply_okay() {
        assert_eq!(
            SetnxReply::from_redis_value(redis::Value::Okay).unwrap(),
            SetnxReply::KeySet
        );
    }

    #[test]
    fn test_setnx_reply_nil() {
        assert_eq!(
            SetnxReply::from_redis_value(redis::Value::Nil).unwrap(),
            SetnxReply::KeyNotSet
        );
    }

    #[test]
    fn test_setnx_reply_unexpected() {
        assert!(SetnxReply::from_redis_value(redis::Value::Int(99)).is_err());
    }

    #[test]
    fn test_hsetnx_reply_key_set() {
        assert_eq!(
            HsetnxReply::from_redis_value(redis::Value::Int(1)).unwrap(),
            HsetnxReply::KeySet
        );
    }

    #[test]
    fn test_hsetnx_reply_key_not_set() {
        assert_eq!(
            HsetnxReply::from_redis_value(redis::Value::Int(0)).unwrap(),
            HsetnxReply::KeyNotSet
        );
    }

    #[test]
    fn test_msetnx_reply_keys_set() {
        assert_eq!(
            MsetnxReply::from_redis_value(redis::Value::Int(1)).unwrap(),
            MsetnxReply::KeysSet
        );
    }

    #[test]
    fn test_msetnx_reply_keys_not_set() {
        assert_eq!(
            MsetnxReply::from_redis_value(redis::Value::Int(0)).unwrap(),
            MsetnxReply::KeysNotSet
        );
    }

    #[test]
    fn test_del_reply_one() {
        assert_eq!(
            DelReply::from_redis_value(redis::Value::Int(1)).unwrap(),
            DelReply::KeyDeleted
        );
    }

    #[test]
    fn test_del_reply_zero() {
        assert_eq!(
            DelReply::from_redis_value(redis::Value::Int(0)).unwrap(),
            DelReply::KeyNotDeleted
        );
    }

    #[test]
    fn test_sadd_reply_key_set() {
        assert_eq!(
            SaddReply::from_redis_value(redis::Value::Int(1)).unwrap(),
            SaddReply::KeySet
        );
    }

    #[test]
    fn test_sadd_reply_key_not_set() {
        assert_eq!(
            SaddReply::from_redis_value(redis::Value::Int(0)).unwrap(),
            SaddReply::KeyNotSet
        );
    }
}
