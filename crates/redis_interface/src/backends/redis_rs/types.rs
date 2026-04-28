//! Redis-rs–specific trait implementations for the shared types.

use crate::types::{
    DelReply, HsetnxReply, MsetnxReply, RedisEntryId, RedisValue, SaddReply, SetnxReply,
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

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            redis::Value::BulkString(bytes) => Some(bytes.as_slice()),
            redis::Value::SimpleString(s) => Some(s.as_bytes()),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match &self.inner {
            redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone()).ok(),
            redis::Value::SimpleString(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn into_inner(self) -> redis::Value {
        self.inner
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
            redis::Value::SimpleString(s) => s.write_redis_args(out),
            _ => Vec::<u8>::new().write_redis_args(out),
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
            redis::Value::Okay => Ok(Self::KeySet),
            redis::Value::SimpleString(ref s) if s == "OK" => Ok(Self::KeySet),
            redis::Value::BulkString(ref s) if s == b"OK" => Ok(Self::KeySet),
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

// ─── Redis-rs-specific helpers ───────────────────────────────────────────────

/// Converts a `redis::Value` to `Option<String>`.
pub fn redis_value_to_option_string(v: &redis::Value) -> Option<String> {
    match v {
        redis::Value::BulkString(bytes) => std::str::from_utf8(bytes).ok().map(|s| s.to_string()),
        redis::Value::SimpleString(s) => Some(s.clone()),
        redis::Value::Int(i) => Some(i.to_string()),
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
    fn test_redis_value_bulk_string_valid_utf8() {
        let value = redis::Value::BulkString("hello".as_bytes().to_vec());
        assert_eq!(
            super::redis_value_to_option_string(&value),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_redis_value_bulk_string_invalid_utf8() {
        let value = redis::Value::BulkString(vec![0xff, 0xfe]);
        assert_eq!(super::redis_value_to_option_string(&value), None);
    }

    #[test]
    fn test_redis_value_simple_string() {
        let value = redis::Value::SimpleString("OK".to_string());
        assert_eq!(
            super::redis_value_to_option_string(&value),
            Some("OK".to_string())
        );
    }

    #[test]
    fn test_redis_value_int() {
        let value = redis::Value::Int(42);
        assert_eq!(
            super::redis_value_to_option_string(&value),
            Some("42".to_string())
        );
    }

    #[test]
    fn test_redis_value_nil() {
        assert_eq!(
            super::redis_value_to_option_string(&redis::Value::Nil),
            None
        );
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
