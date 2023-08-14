#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    // TODO: deprecate this error type to use a domain error instead
    #[error("DatabaseError: {0:?}")]
    DatabaseError(String),
    #[error("ValueNotFound: {0}")]
    ValueNotFound(String),
    #[error("DuplicateValue: {entity} already exists {key:?}")]
    DuplicateValue {
        entity: &'static str,
        key: Option<String>,
    },
    #[error("Timed out while trying to connect to the database")]
    DatabaseConnectionError,
    #[error("KV error")]
    KVError,
    #[error("Serialization failure")]
    SerializationFailed,
    #[error("MockDb error")]
    MockDbError,
    #[error("Customer with this id is Redacted")]
    CustomerRedacted,
    #[error("Deserialization failure")]
    DeserializationFailed,
    #[error("Error while encrypting data")]
    EncryptionError,
    #[error("Error while decrypting data from database")]
    DecryptionError,
    // TODO: deprecate this error type to use a domain error instead
    #[error("RedisError: {0:?}")]
    RedisError(String),
    #[error("Temporary error to be replaced")]
    TemporaryError,
}
