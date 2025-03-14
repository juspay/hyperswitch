pub use common_enums::{ApiClientError, ApplicationError, ApplicationResult};
use common_utils::errors::ErrorSwitch;
use hyperswitch_domain_models::errors::StorageError as DataStorageError;
pub use redis_interface::errors::RedisError;

use crate::store::errors::DatabaseError;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
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
    #[error("RedisError: {0:?}")]
    RedisError(error_stack::Report<RedisError>),
}

impl ErrorSwitch<DataStorageError> for StorageError {
    fn switch(&self) -> DataStorageError {
        self.into()
    }
}

#[allow(clippy::from_over_into)]
impl Into<DataStorageError> for &StorageError {
    fn into(self) -> DataStorageError {
        match self {
            StorageError::DatabaseError(i) => match i.current_context() {
                DatabaseError::DatabaseConnectionError => DataStorageError::DatabaseConnectionError,
                // TODO: Update this error type to encompass & propagate the missing type (instead of generic `db value not found`)
                DatabaseError::NotFound => {
                    DataStorageError::ValueNotFound(String::from("db value not found"))
                }
                // TODO: Update this error type to encompass & propagate the duplicate type (instead of generic `db value not found`)
                DatabaseError::UniqueViolation => DataStorageError::DuplicateValue {
                    entity: "db entity",
                    key: None,
                },
                err => DataStorageError::DatabaseError(error_stack::report!(*err)),
            },
            StorageError::ValueNotFound(i) => DataStorageError::ValueNotFound(i.clone()),
            StorageError::DuplicateValue { entity, key } => DataStorageError::DuplicateValue {
                entity,
                key: key.clone(),
            },
            StorageError::DatabaseConnectionError => DataStorageError::DatabaseConnectionError,
            StorageError::KVError => DataStorageError::KVError,
            StorageError::SerializationFailed => DataStorageError::SerializationFailed,
            StorageError::MockDbError => DataStorageError::MockDbError,
            StorageError::KafkaError => DataStorageError::KafkaError,
            StorageError::CustomerRedacted => DataStorageError::CustomerRedacted,
            StorageError::DeserializationFailed => DataStorageError::DeserializationFailed,
            StorageError::EncryptionError => DataStorageError::EncryptionError,
            StorageError::DecryptionError => DataStorageError::DecryptionError,
            StorageError::RedisError(i) => match i.current_context() {
                // TODO: Update this error type to encompass & propagate the missing type (instead of generic `redis value not found`)
                RedisError::NotFound => {
                    DataStorageError::ValueNotFound("redis value not found".to_string())
                }
                RedisError::JsonSerializationFailed => DataStorageError::SerializationFailed,
                RedisError::JsonDeserializationFailed => DataStorageError::DeserializationFailed,
                i => DataStorageError::RedisError(format!("{:?}", i)),
            },
        }
    }
}

impl From<error_stack::Report<RedisError>> for StorageError {
    fn from(err: error_stack::Report<RedisError>) -> Self {
        Self::RedisError(err)
    }
}

impl From<diesel::result::Error> for StorageError {
    fn from(err: diesel::result::Error) -> Self {
        Self::from(error_stack::report!(DatabaseError::from(err)))
    }
}

impl From<error_stack::Report<DatabaseError>> for StorageError {
    fn from(err: error_stack::Report<DatabaseError>) -> Self {
        Self::DatabaseError(err)
    }
}

impl StorageError {
    pub fn is_db_not_found(&self) -> bool {
        match self {
            Self::DatabaseError(err) => matches!(err.current_context(), DatabaseError::NotFound),
            Self::ValueNotFound(_) => true,
            Self::RedisError(err) => matches!(err.current_context(), RedisError::NotFound),
            _ => false,
        }
    }

    pub fn is_db_unique_violation(&self) -> bool {
        match self {
            Self::DatabaseError(err) => {
                matches!(err.current_context(), DatabaseError::UniqueViolation,)
            }
            _ => false,
        }
    }
}

pub trait RedisErrorExt {
    #[track_caller]
    fn to_redis_failed_response(self, key: &str) -> error_stack::Report<DataStorageError>;
}

impl RedisErrorExt for error_stack::Report<RedisError> {
    fn to_redis_failed_response(self, key: &str) -> error_stack::Report<DataStorageError> {
        match self.current_context() {
            RedisError::NotFound => self.change_context(DataStorageError::ValueNotFound(format!(
                "Data does not exist for key {key}",
            ))),
            RedisError::SetNxFailed | RedisError::SetAddMembersFailed => {
                self.change_context(DataStorageError::DuplicateValue {
                    entity: "redis",
                    key: Some(key.to_string()),
                })
            }
            _ => self.change_context(DataStorageError::KVError),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConnectorError {
    #[error("Error while obtaining URL for the integration")]
    FailedToObtainIntegrationUrl,
    #[error("Failed to encode connector request")]
    RequestEncodingFailed,
    #[error("Request encoding failed : {0}")]
    RequestEncodingFailedWithReason(String),
    #[error("Parsing failed")]
    ParsingFailed,
    #[error("Failed to deserialize connector response")]
    ResponseDeserializationFailed,
    #[error("Failed to execute a processing step: {0:?}")]
    ProcessingStepFailed(Option<bytes::Bytes>),
    #[error("The connector returned an unexpected response: {0:?}")]
    UnexpectedResponseError(bytes::Bytes),
    #[error("Failed to parse custom routing rules from merchant account")]
    RoutingRulesParsingError,
    #[error("Failed to obtain preferred connector from merchant account")]
    FailedToObtainPreferredConnector,
    #[error("An invalid connector name was provided")]
    InvalidConnectorName,
    #[error("An invalid Wallet was used")]
    InvalidWallet,
    #[error("Failed to handle connector response")]
    ResponseHandlingFailed,
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: &'static str },
    #[error("Missing required fields: {field_names:?}")]
    MissingRequiredFields { field_names: Vec<&'static str> },
    #[error("Failed to obtain authentication type")]
    FailedToObtainAuthType,
    #[error("Failed to obtain certificate")]
    FailedToObtainCertificate,
    #[error("Connector meta data not found")]
    NoConnectorMetaData,
    #[error("Failed to obtain certificate key")]
    FailedToObtainCertificateKey,
    #[error("This step has not been implemented for: {0}")]
    NotImplemented(String),
    #[error("{message} is not supported by {connector}")]
    NotSupported {
        message: String,
        connector: &'static str,
        payment_experience: String,
    },
    #[error("{flow} flow not supported by {connector} connector")]
    FlowNotSupported { flow: String, connector: String },
    #[error("Capture method not supported")]
    CaptureMethodNotSupported,
    #[error("Missing connector transaction ID")]
    MissingConnectorTransactionID,
    #[error("Missing connector refund ID")]
    MissingConnectorRefundID,
    #[error("Webhooks not implemented for this connector")]
    WebhooksNotImplemented,
    #[error("Failed to decode webhook event body")]
    WebhookBodyDecodingFailed,
    #[error("Signature not found for incoming webhook")]
    WebhookSignatureNotFound,
    #[error("Failed to verify webhook source")]
    WebhookSourceVerificationFailed,
    #[error("Could not find merchant secret in DB for incoming webhook source verification")]
    WebhookVerificationSecretNotFound,
    #[error("Incoming webhook object reference ID not found")]
    WebhookReferenceIdNotFound,
    #[error("Incoming webhook event type not found")]
    WebhookEventTypeNotFound,
    #[error("Incoming webhook event resource object not found")]
    WebhookResourceObjectNotFound,
    #[error("Could not respond to the incoming webhook event")]
    WebhookResponseEncodingFailed,
    #[error("Invalid Date/time format")]
    InvalidDateFormat,
    #[error("Date Formatting Failed")]
    DateFormattingFailed,
    #[error("Invalid Data format")]
    InvalidDataFormat { field_name: &'static str },
    #[error("Payment Method data / Payment Method Type / Payment Experience Mismatch ")]
    MismatchedPaymentData,
    #[error("Failed to parse Wallet token")]
    InvalidWalletToken { wallet_name: String },
    #[error("Missing Connector Related Transaction ID")]
    MissingConnectorRelatedTransactionID { id: String },
    #[error("File Validation failed")]
    FileValidationFailed { reason: String },
    #[error("Missing 3DS redirection payload: {field_name}")]
    MissingConnectorRedirectionPayload { field_name: &'static str },
}

#[derive(Debug, thiserror::Error)]
pub enum HealthCheckDBError {
    #[error("Error while connecting to database")]
    DBError,
    #[error("Error while writing to database")]
    DBWriteError,
    #[error("Error while reading element in the database")]
    DBReadError,
    #[error("Error while deleting element in the database")]
    DBDeleteError,
    #[error("Unpredictable error occurred")]
    UnknownError,
    #[error("Error in database transaction")]
    TransactionError,
    #[error("Error while executing query in Sqlx Analytics")]
    SqlxAnalyticsError,
    #[error("Error while executing query in Clickhouse Analytics")]
    ClickhouseAnalyticsError,
    #[error("Error while executing query in Opensearch")]
    OpensearchError,
}

impl From<diesel::result::Error> for HealthCheckDBError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::DatabaseError(_, _) => Self::DBError,

            diesel::result::Error::RollbackErrorOnCommit { .. }
            | diesel::result::Error::RollbackTransaction
            | diesel::result::Error::AlreadyInTransaction
            | diesel::result::Error::NotInTransaction
            | diesel::result::Error::BrokenTransactionManager => Self::TransactionError,

            _ => Self::UnknownError,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HealthCheckRedisError {
    #[error("Failed to establish Redis connection")]
    RedisConnectionError,
    #[error("Failed to set key value in Redis")]
    SetFailed,
    #[error("Failed to get key value in Redis")]
    GetFailed,
    #[error("Failed to delete key value in Redis")]
    DeleteFailed,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum HealthCheckLockerError {
    #[error("Failed to establish Locker connection")]
    FailedToCallLocker,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum HealthCheckGRPCServiceError {
    #[error("Failed to establish connection with gRPC service")]
    FailedToCallService,
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum RecoveryError {
    #[error("Failed to make a recovery payment")]
    PaymentCallFailed,
    #[error("Encountered a Process Tracker Task Failure")]
    ProcessTrackerFailure,
    #[error("The encountered task is invalid")]
    InvalidTask,
    #[error("The Intended data was not found")]
    ValueNotFound,
}
