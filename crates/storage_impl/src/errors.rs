use std::fmt::Display;

use actix_web::ResponseError;
use common_utils::errors::ErrorSwitch;
use config::ConfigError;
use data_models::errors::StorageError as DataStorageError;
use http::StatusCode;
pub use redis_interface::errors::RedisError;
use router_env::opentelemetry::metrics::MetricsError;

use crate::{errors as storage_errors, store::errors::DatabaseError};

pub type ApplicationResult<T> = Result<T, ApplicationError>;

macro_rules! impl_error_display {
    ($st: ident, $arg: tt) => {
        impl Display for $st {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    fmt,
                    "{{ error_type: {:?}, error_description: {} }}",
                    self, $arg
                )
            }
        }
    };
}
macro_rules! impl_error_type {
    ($name: ident, $arg: tt) => {
        #[derive(Debug)]
        pub struct $name;

        impl_error_display!($name, $arg);

        impl std::error::Error for $name {}
    };
}

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
        /// This method converts the current instance of a type into a DataStorageError
    fn switch(&self) -> DataStorageError {
        self.into()
    }
}

#[allow(clippy::from_over_into)]
impl Into<DataStorageError> for &StorageError {
        /// Converts the current `StorageError` enum variant into a `DataStorageError` enum variant, mapping each variant to its corresponding `DataStorageError` variant. Additionally, it handles the conversion of error types specific to database, redis, and other storage systems. If needed, it updates the error type to encompass and propagate the missing or generic error type.
    fn into(self) -> DataStorageError {
        match self {
            StorageError::DatabaseError(i) => match i.current_context() {
                storage_errors::DatabaseError::DatabaseConnectionError => {
                    DataStorageError::DatabaseConnectionError
                }
                // TODO: Update this error type to encompass & propagate the missing type (instead of generic `db value not found`)
                storage_errors::DatabaseError::NotFound => {
                    DataStorageError::ValueNotFound(String::from("db value not found"))
                }
                // TODO: Update this error type to encompass & propagate the duplicate type (instead of generic `db value not found`)
                storage_errors::DatabaseError::UniqueViolation => {
                    DataStorageError::DuplicateValue {
                        entity: "db entity",
                        key: None,
                    }
                }
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
        /// Converts a `error_stack::Report<RedisError>` into a `Self` enum variant `RedisError`.
    fn from(err: error_stack::Report<RedisError>) -> Self {
            Self::RedisError(err)
    }
}

impl From<error_stack::Report<DatabaseError>> for StorageError {
        /// Constructs a new instance of the current enum variant with the given DatabaseError wrapped in an error_stack::Report.
    fn from(err: error_stack::Report<DatabaseError>) -> Self {
        Self::DatabaseError(err)
    }
}

impl StorageError {
        /// Checks if the current error is a DatabaseError with a NotFound context.
    pub fn is_db_not_found(&self) -> bool {
        match self {
            Self::DatabaseError(err) => matches!(err.current_context(), DatabaseError::NotFound),
            _ => false,
        }
    }

        /// Checks if the current error is a unique violation in the database.
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
        /// Converts a Redis error to a failed response for data storage, based on the current context.
    fn to_redis_failed_response(self, key: &str) -> error_stack::Report<DataStorageError> {
        match self.current_context() {
            RedisError::NotFound => self.change_context(DataStorageError::ValueNotFound(format!(
                "Data does not exist for key {key}",
            ))),
            RedisError::SetNxFailed => self.change_context(DataStorageError::DuplicateValue {
                entity: "redis",
                key: Some(key.to_string()),
            }),
            _ => self.change_context(DataStorageError::KVError),
        }
    }
}

impl_error_type!(EncryptionError, "Encryption error");

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    // Display's impl can be overridden by the attribute error marco.
    // Don't use Debug here, Debug gives error stack in response.
    #[error("Application configuration error: {0}")]
    ConfigurationError(ConfigError),

    #[error("Invalid configuration value provided: {0}")]
    InvalidConfigurationValueError(String),

    #[error("Metrics error: {0}")]
    MetricsError(MetricsError),

    #[error("I/O: {0}")]
    IoError(std::io::Error),

    #[error("Error while constructing api client: {0}")]
    ApiClientError(ApiClientError),
}

impl From<MetricsError> for ApplicationError {
        /// Converts a MetricsError into another type
    fn from(err: MetricsError) -> Self {
        Self::MetricsError(err)
    }
}

impl From<std::io::Error> for ApplicationError {
        /// Creates a new instance of the enum variant Self::IoError with the provided std::io::Error.
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<ring::error::Unspecified> for EncryptionError {
        /// Converts a `ring::error::Unspecified` error into the implementing type.
    fn from(_: ring::error::Unspecified) -> Self {
        Self
    }
}

impl From<ConfigError> for ApplicationError {
        /// Converts a ConfigError into a Self instance.
    fn from(err: ConfigError) -> Self {
        Self::ConfigurationError(err)
    }
}

/// This function takes an error message of type T that implements the Display trait and returns an HTTP response with a status code of 400 (Bad Request), content type of application/json, and a JSON body containing the error message.
fn error_response<T: Display>(err: &T) -> actix_web::HttpResponse {
    actix_web::HttpResponse::BadRequest()
        .content_type(mime::APPLICATION_JSON)
        .body(format!(r#"{{ "error": {{ "message": "{err}" }} }}"#))
}

impl ResponseError for ApplicationError {
        /// This method returns the status code based on the type of error. If the error is of type MetricsError, IoError, ConfigurationError, InvalidConfigurationValueError, or ApiClientError, it returns an internal server error status code.
    fn status_code(&self) -> StatusCode {
        match self {
            Self::MetricsError(_)
            | Self::IoError(_)
            | Self::ConfigurationError(_)
            | Self::InvalidConfigurationValueError(_)
            | Self::ApiClientError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

        /// This method returns an HTTP response with an error status code and message.
    fn error_response(&self) -> actix_web::HttpResponse {
        error_response(self)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Clone)]
pub enum ApiClientError {
    #[error("Header map construction failed")]
    HeaderMapConstructionFailed,
    #[error("Invalid proxy configuration")]
    InvalidProxyConfiguration,
    #[error("Client construction failed")]
    ClientConstructionFailed,
    #[error("Certificate decode failed")]
    CertificateDecodeFailed,
    #[error("Request body serialization failed")]
    BodySerializationFailed,
    #[error("Unexpected state reached/Invariants conflicted")]
    UnexpectedState,

    #[error("URL encoding of request payload failed")]
    UrlEncodingFailed,
    #[error("Failed to send request to connector {0}")]
    RequestNotSent(String),
    #[error("Failed to decode response")]
    ResponseDecodingFailed,

    #[error("Server responded with Request Timeout")]
    RequestTimeoutReceived,

    #[error("connection closed before a message could complete")]
    ConnectionClosed,

    #[error("Server responded with Internal Server Error")]
    InternalServerErrorReceived,
    #[error("Server responded with Bad Gateway")]
    BadGatewayReceived,
    #[error("Server responded with Service Unavailable")]
    ServiceUnavailableReceived,
    #[error("Server responded with Gateway Timeout")]
    GatewayTimeoutReceived,
    #[error("Server responded with unexpected response")]
    UnexpectedServerResponse,
}

impl ApiClientError {
        /// Checks if the current instance is equal to the RequestTimeoutReceived variant of the enum.
    pub fn is_upstream_timeout(&self) -> bool {
        self == &Self::RequestTimeoutReceived
    }
        /// Checks if the current connection is closed.
    pub fn is_connection_closed(&self) -> bool {
        self == &Self::ConnectionClosed
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
    InvalidWalletToken,
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
}

impl From<diesel::result::Error> for HealthCheckDBError {
        /// Converts a diesel result error into a custom error enum.
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
