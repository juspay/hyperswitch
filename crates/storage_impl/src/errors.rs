pub use common_enums::{ApiClientError, ApplicationError, ApplicationResult};
pub use redis_interface::errors::RedisError;

use crate::store::errors::DatabaseError;
pub type StorageResult<T> = error_stack::Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Initialization Error")]
    InitializationError,
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

impl From<error_stack::Report<RedisError>> for StorageError {
    fn from(err: error_stack::Report<RedisError>) -> Self {
        match err.current_context() {
            RedisError::NotFound => Self::ValueNotFound("redis value not found".to_string()),
            RedisError::JsonSerializationFailed => Self::SerializationFailed,
            RedisError::JsonDeserializationFailed => Self::DeserializationFailed,
            _ => Self::RedisError(err),
        }
    }
}

impl From<diesel::result::Error> for StorageError {
    fn from(err: diesel::result::Error) -> Self {
        Self::from(error_stack::report!(DatabaseError::from(err)))
    }
}

impl From<error_stack::Report<DatabaseError>> for StorageError {
    fn from(err: error_stack::Report<DatabaseError>) -> Self {
        match err.current_context() {
            DatabaseError::DatabaseConnectionError => Self::DatabaseConnectionError,
            DatabaseError::NotFound => Self::ValueNotFound(String::from("db value not found")),
            DatabaseError::UniqueViolation => Self::DuplicateValue {
                entity: "db entity",
                key: None,
            },
            _ => Self::DatabaseError(err),
        }
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
            Self::DuplicateValue { .. } => true,
            _ => false,
        }
    }
}

pub trait RedisErrorExt {
    #[track_caller]
    fn to_redis_failed_response(self, key: &str) -> error_stack::Report<StorageError>;
}

impl RedisErrorExt for error_stack::Report<RedisError> {
    fn to_redis_failed_response(self, key: &str) -> error_stack::Report<StorageError> {
        match self.current_context() {
            RedisError::NotFound => self.change_context(StorageError::ValueNotFound(format!(
                "Data does not exist for key {key}",
            ))),
            RedisError::SetNxFailed | RedisError::SetAddMembersFailed => {
                self.change_context(StorageError::DuplicateValue {
                    entity: "redis",
                    key: Some(key.to_string()),
                })
            }
            _ => self.change_context(StorageError::KVError),
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
    #[error("Connector '{connector}' rejected field '{field_name}': length {received_length} exceeds maximum of {max_length}'")]
    MaxFieldLengthViolated {
        connector: String,
        field_name: String,
        max_length: usize,
        received_length: usize,
    },
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
    #[error("Field {fields} doesn't match with the ones used during mandate creation")]
    MandatePaymentDataMismatch { fields: String },
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
    #[error("The Process Tracker data was not found")]
    ValueNotFound,
    #[error("Failed to update billing connector")]
    RecordBackToBillingConnectorFailed,
    #[error("Failed to fetch billing connector account id")]
    BillingMerchantConnectorAccountIdNotFound,
    #[error("Failed to generate payment sync data")]
    PaymentsResponseGenerationFailed,
    #[error("Outgoing Webhook Failed")]
    RevenueRecoveryOutgoingWebhookFailed,
}
#[derive(Debug, Clone, thiserror::Error)]
pub enum HealthCheckDecisionEngineError {
    #[error("Failed to establish Decision Engine connection")]
    FailedToCallDecisionEngineService,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum HealthCheckUnifiedConnectorServiceError {
    #[error("Failed to establish Unified Connector Service connection")]
    FailedToCallUnifiedConnectorService,
}
