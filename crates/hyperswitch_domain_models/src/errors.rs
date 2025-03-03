pub mod api_error_response;
use crate::errors::api_error_response::ApiErrorResponse;
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

impl StorageError {
    pub fn is_db_not_found(&self) -> bool {
        match self {
            Self::DatabaseError(err) => matches!(err.current_context(), DatabaseError::NotFound),
            Self::ValueNotFound(_) => true,
            _ => false,
        }
    }
}

pub trait StorageErrorExt<T, E> {
    #[track_caller]
    fn to_not_found_response(self, not_found_response: E) -> error_stack::Result<T, E>;

    #[track_caller]
    fn to_duplicate_response(self, duplicate_response: E) -> error_stack::Result<T, E>;
}

impl<T> StorageErrorExt<T, ApiErrorResponse> for error_stack::Result<T, StorageError> {
    #[track_caller]
    fn to_not_found_response(
        self,
        not_found_response: ApiErrorResponse,
    ) -> error_stack::Result<T, ApiErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                StorageError::ValueNotFound(_) => not_found_response,
                StorageError::CustomerRedacted => ApiErrorResponse::CustomerRedacted,
                _ => ApiErrorResponse::InternalServerError,
            };
            err.change_context(new_err)
        })
    }

    #[track_caller]
    fn to_duplicate_response(
        self,
        duplicate_response: ApiErrorResponse,
    ) -> error_stack::Result<T, ApiErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                StorageError::DuplicateValue { .. } => duplicate_response,
                _ => ApiErrorResponse::InternalServerError,
            };
            err.change_context(new_err)
        })
    }
}
