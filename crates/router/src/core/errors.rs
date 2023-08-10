pub mod api_error_response;
pub mod error_handlers;
pub mod transformers;
pub mod utils;
pub mod customers_error_response;

use std::fmt::Display;

use actix_web::{body::BoxBody, http::StatusCode, ResponseError};
use common_utils::errors::ErrorSwitch;
pub use common_utils::errors::{CustomResult, ParsingError, ValidationError};
use config::ConfigError;
pub use data_models::errors::StorageError as DataStorageError;
use diesel_models::errors as storage_errors;
pub use redis_interface::errors::RedisError;
use router_env::opentelemetry::metrics::MetricsError;

pub use self::{
    api_error_response::ApiErrorResponse,
    utils::{ConnectorErrorExt, StorageErrorExt},
    customers_error_response::CustomersErrorResponse,
};
use crate::services;
pub type RouterResult<T> = CustomResult<T, ApiErrorResponse>;
pub type RouterResponse<T> = CustomResult<services::ApplicationResponse<T>, ApiErrorResponse>;

pub type ApplicationResult<T> = Result<T, ApplicationError>;
pub type ApplicationResponse<T> = ApplicationResult<services::ApplicationResponse<T>>;

pub type CustomerResponse<T> = CustomResult<services::ApplicationResponse<T>, CustomersErrorResponse>;

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
    DatabaseError(error_stack::Report<storage_errors::DatabaseError>),
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
                storage_errors::DatabaseError::NoFieldsToUpdate => {
                    DataStorageError::DatabaseError("No fields to update".to_string())
                }
                storage_errors::DatabaseError::QueryGenerationFailed => {
                    DataStorageError::DatabaseError("Query generation failed".to_string())
                }
                storage_errors::DatabaseError::Others => {
                    DataStorageError::DatabaseError("Unknown database error".to_string())
                }
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

impl From<error_stack::Report<storage_errors::DatabaseError>> for StorageError {
    fn from(err: error_stack::Report<storage_errors::DatabaseError>) -> Self {
        Self::DatabaseError(err)
    }
}

impl StorageError {
    pub fn is_db_not_found(&self) -> bool {
        match self {
            Self::DatabaseError(err) => matches!(
                err.current_context(),
                storage_errors::DatabaseError::NotFound
            ),
            Self::ValueNotFound(_) => true,
            _ => false,
        }
    }

    pub fn is_db_unique_violation(&self) -> bool {
        match self {
            Self::DatabaseError(err) => matches!(
                err.current_context(),
                storage_errors::DatabaseError::UniqueViolation,
            ),
            _ => false,
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
}

impl From<MetricsError> for ApplicationError {
    fn from(err: MetricsError) -> Self {
        Self::MetricsError(err)
    }
}

impl From<std::io::Error> for ApplicationError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<ring::error::Unspecified> for EncryptionError {
    fn from(_: ring::error::Unspecified) -> Self {
        Self
    }
}

impl From<ConfigError> for ApplicationError {
    fn from(err: ConfigError) -> Self {
        Self::ConfigurationError(err)
    }
}

fn error_response<T: Display>(err: &T) -> actix_web::HttpResponse {
    actix_web::HttpResponse::BadRequest()
        .content_type(mime::APPLICATION_JSON)
        .body(format!(r#"{{ "error": {{ "message": "{err}" }} }}"#))
}

impl ResponseError for ApplicationError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::MetricsError(_)
            | Self::IoError(_)
            | Self::ConfigurationError(_)
            | Self::InvalidConfigurationValueError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        error_response(self)
    }
}

pub fn http_not_implemented() -> actix_web::HttpResponse<BoxBody> {
    ApiErrorResponse::NotImplemented {
        message: api_error_response::NotImplementedMessage::Default,
    }
    .error_response()
}

#[derive(Debug, thiserror::Error, PartialEq)]
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
    },
    #[error("{flow} flow not supported by {connector} connector")]
    FlowNotSupported { flow: String, connector: String },
    #[error("Capture method not supported")]
    CaptureMethodNotSupported,
    #[error("Missing connector mandate ID")]
    MissingConnectorMandateID,
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
    #[error("Failed at connector's end with code '{code}'")]
    FailedAtConnector { message: String, code: String },
    #[error("Payment Method Type not found")]
    MissingPaymentMethodType,
    #[error("Balance in the payment method is low")]
    InSufficientBalanceInPaymentMethod,
    #[error("Server responded with Request Timeout")]
    RequestTimeoutReceived,
}

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Failed to save card in card vault")]
    SaveCardFailed,
    #[error("Failed to fetch card details from card vault")]
    FetchCardFailed,
    #[error("Failed to encode card vault request")]
    RequestEncodingFailed,
    #[error("Failed to deserialize card vault response")]
    ResponseDeserializationFailed,
    #[error("Failed to create payment method")]
    PaymentMethodCreationFailed,
    #[error("The given payment method is currently not supported in vault")]
    PaymentMethodNotSupported,
    #[error("The given payout method is currently not supported in vault")]
    PayoutMethodNotSupported,
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: &'static str },
    #[error("The card vault returned an unexpected response: {0:?}")]
    UnexpectedResponseError(bytes::Bytes),
    #[error("Failed to update in PMD table")]
    UpdateInPaymentMethodDataTableFailed,
    #[error("Failed to fetch payment method in vault")]
    FetchPaymentMethodFailed,
    #[error("Failed to save payment method in vault")]
    SavePaymentMethodFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum KmsError {
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,
    #[error("Failed to KMS decrypt input data")]
    DecryptionFailed,
    #[error("Missing plaintext KMS decryption output")]
    MissingPlaintextDecryptionOutput,
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessTrackerError {
    #[error("An unexpected flow was specified")]
    UnexpectedFlow,
    #[error("Failed to serialize object")]
    SerializationFailed,
    #[error("Failed to deserialize object")]
    DeserializationFailed,
    #[error("Missing required field")]
    MissingRequiredField,
    #[error("Failed to insert process batch into stream")]
    BatchInsertionFailed,
    #[error("Failed to insert process into stream")]
    ProcessInsertionFailed,
    #[error("The process batch with the specified details was not found")]
    BatchNotFound,
    #[error("Failed to update process batch in stream")]
    BatchUpdateFailed,
    #[error("Failed to delete process batch from stream")]
    BatchDeleteFailed,
    #[error("An error occurred when trying to read process tracker configuration")]
    ConfigurationError,
    #[error("Failed to update process in database")]
    ProcessUpdateFailed,
    #[error("Failed to fetch processes from database")]
    ProcessFetchingFailed,
    #[error("Failed while fetching: {resource_name}")]
    ResourceFetchingFailed { resource_name: &'static str },
    #[error("Failed while executing: {flow}")]
    FlowExecutionError { flow: &'static str },
    #[error("Not Implemented")]
    NotImplemented,
    #[error("Job not found")]
    JobNotFound,
    #[error("Received Error ApiResponseError: {0}")]
    EApiErrorResponse(error_stack::Report<ApiErrorResponse>),
    #[error("Received Error StorageError: {0}")]
    EStorageError(error_stack::Report<StorageError>),
    #[error("Received Error RedisError: {0}")]
    ERedisError(error_stack::Report<RedisError>),
    #[error("Received Error ParsingError: {0}")]
    EParsingError(error_stack::Report<ParsingError>),
    #[error("Validation Error Received: {0}")]
    EValidationError(error_stack::Report<ValidationError>),
    #[error("Type Conversion error")]
    TypeConversionError,
}

macro_rules! error_to_process_tracker_error {
    ($($path: ident)::+ < $st: ident >, $($path2:ident)::* ($($inner_path2:ident)::+ <$st2:ident>) ) => {
        impl From<$($path)::+ <$st>> for ProcessTrackerError {
            fn from(err: $($path)::+ <$st> ) -> Self {
                $($path2)::*(err)
            }
        }
    };

    ($($path: ident)::+  <$($inner_path:ident)::+>, $($path2:ident)::* ($($inner_path2:ident)::+ <$st2:ident>) ) => {
        impl<'a> From< $($path)::+ <$($inner_path)::+> > for ProcessTrackerError {
            fn from(err: $($path)::+ <$($inner_path)::+> ) -> Self {
                $($path2)::*(err)
            }
        }
    };
}

error_to_process_tracker_error!(
    error_stack::Report<ApiErrorResponse>,
    ProcessTrackerError::EApiErrorResponse(error_stack::Report<ApiErrorResponse>)
);

error_to_process_tracker_error!(
    error_stack::Report<StorageError>,
    ProcessTrackerError::EStorageError(error_stack::Report<StorageError>)
);

error_to_process_tracker_error!(
    error_stack::Report<RedisError>,
    ProcessTrackerError::ERedisError(error_stack::Report<RedisError>)
);

error_to_process_tracker_error!(
    error_stack::Report<ParsingError>,
    ProcessTrackerError::EParsingError(error_stack::Report<ParsingError>)
);

error_to_process_tracker_error!(
    error_stack::Report<ValidationError>,
    ProcessTrackerError::EValidationError(error_stack::Report<ValidationError>)
);

#[derive(Debug, thiserror::Error)]
pub enum WebhooksFlowError {
    #[error("Merchant webhook config not found")]
    MerchantConfigNotFound,
    #[error("Webhook details for merchant not configured")]
    MerchantWebhookDetailsNotFound,
    #[error("Merchant does not have a webhook URL configured")]
    MerchantWebhookURLNotConfigured,
    #[error("Payments core flow failed")]
    PaymentsCoreFailed,
    #[error("Refunds core flow failed")]
    RefundsCoreFailed,
    #[error("Dispuste core flow failed")]
    DisputeCoreFailed,
    #[error("Webhook event creation failed")]
    WebhookEventCreationFailed,
    #[error("Webhook event updation failed")]
    WebhookEventUpdationFailed,
    #[error("Outgoing webhook body signing failed")]
    OutgoingWebhookSigningFailed,
    #[error("Unable to fork webhooks flow for outgoing webhooks")]
    ForkFlowFailed,
    #[error("Webhook api call to merchant failed")]
    CallToMerchantFailed,
    #[error("Webhook not received by merchant")]
    NotReceivedByMerchant,
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Webhook source verification failed")]
    WebhookSourceVerificationFailed,
    #[error("Webhook event object creation failed")]
    WebhookEventObjectCreationFailed,
    #[error("Not implemented")]
    NotImplemented,
    #[error("Dispute webhook status validation failed")]
    DisputeWebhookValidationFailed,
    #[error("Outgoing webhook body encoding failed")]
    OutgoingWebhookEncodingFailed,
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: &'static str },
}

impl ApiClientError {
    pub fn is_upstream_timeout(&self) -> bool {
        self == &Self::RequestTimeoutReceived
    }
}

impl ConnectorError {
    pub fn is_connector_timeout(&self) -> bool {
        self == &Self::RequestTimeoutReceived
    }
}

#[cfg(feature = "detailed_errors")]
pub mod error_stack_parsing {

    #[derive(serde::Deserialize)]
    pub struct NestedErrorStack<'a> {
        context: std::borrow::Cow<'a, str>,
        attachments: Vec<std::borrow::Cow<'a, str>>,
        sources: Vec<NestedErrorStack<'a>>,
    }

    #[derive(serde::Serialize, Debug)]
    struct LinearErrorStack<'a> {
        context: std::borrow::Cow<'a, str>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        attachments: Vec<std::borrow::Cow<'a, str>>,
    }

    #[derive(serde::Serialize, Debug)]
    pub struct VecLinearErrorStack<'a>(Vec<LinearErrorStack<'a>>);

    impl<'a> From<Vec<NestedErrorStack<'a>>> for VecLinearErrorStack<'a> {
        fn from(value: Vec<NestedErrorStack<'a>>) -> Self {
            let multi_layered_errors: Vec<_> = value
                .into_iter()
                .flat_map(|current_error| {
                    [LinearErrorStack {
                        context: current_error.context,
                        attachments: current_error.attachments,
                    }]
                    .into_iter()
                    .chain(
                        Into::<VecLinearErrorStack<'a>>::into(current_error.sources)
                            .0
                            .into_iter(),
                    )
                })
                .collect();
            Self(multi_layered_errors)
        }
    }
}
#[cfg(feature = "detailed_errors")]
pub use error_stack_parsing::*;
