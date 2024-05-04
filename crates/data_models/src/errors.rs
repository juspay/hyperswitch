use diesel_models::errors::DatabaseError;

pub type StorageResult<T> = error_stack::Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Initialization Error")]
    InitializationError,
    // TODO: deprecate this error type to use a domain error instead
    #[error("DatabaseError: {0:?}")]
    DatabaseError(error_stack::Report<DatabaseError>),
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
    #[error("Kafka error")]
    KafkaError,
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
}
