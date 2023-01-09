pub(crate) mod api_error_response;
pub(crate) mod error_handlers;
pub(crate) mod utils;

use std::fmt::Display;

use actix_web::{body::BoxBody, http::StatusCode, ResponseError};
pub use common_utils::errors::{CustomResult, ParsingError, ValidationError};
use config::ConfigError;
use error_stack;
pub use redis_interface::errors::RedisError;
use router_env::opentelemetry::metrics::MetricsError;
use storage_models::errors as storage_errors;

pub use self::api_error_response::ApiErrorResponse;
pub(crate) use self::utils::{ConnectorErrorExt, StorageErrorExt};
use crate::services;
pub type RouterResult<T> = CustomResult<T, ApiErrorResponse>;
pub type RouterResponse<T> = CustomResult<services::ApplicationResponse<T>, ApiErrorResponse>;

pub type ApplicationResult<T> = Result<T, ApplicationError>;
pub type ApplicationResponse<T> = ApplicationResult<services::ApplicationResponse<T>>;

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

// FIXME: Make this a derive macro instead
macro_rules! router_error_error_stack_specific {
    ($($path: ident)::+ < $st: ident >, $($path2:ident)::* ($($inner_path2:ident)::+ <$st2:ident>) ) => {
        impl From<$($path)::+ <$st>> for ApplicationError {
            fn from(err: $($path)::+ <$st> ) -> Self {
                $($path2)::*(err)
            }
        }
    };

    ($($path: ident)::+  <$($inner_path:ident)::+>, $($path2:ident)::* ($($inner_path2:ident)::+ <$st2:ident>) ) => {
        impl<'a> From< $($path)::+ <$($inner_path)::+> > for ApplicationError {
            fn from(err: $($path)::+ <$($inner_path)::+> ) -> Self {
                $($path2)::*(err)
            }
        }
    };
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("DatabaseError: {0}")]
    DatabaseError(error_stack::Report<storage_errors::DatabaseError>),
    #[error("ValueNotFound: {0}")]
    ValueNotFound(String),
    #[error("DuplicateValue: {0}")]
    DuplicateValue(String),
    #[error("KV error")]
    KVError,
    #[error("MockDb error")]
    MockDbError,
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

impl_error_type!(AuthenticationError, "Authentication error");
impl_error_type!(AuthorisationError, "Authorisation error");
impl_error_type!(EncryptionError, "Encryption error");
impl_error_type!(UnexpectedError, "Unexpected error");

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    // Display's impl can be overridden by the attribute error marco.
    // Don't use Debug here, Debug gives error stack in response.
    #[error("{{ error_description: Error while Authenticating, error_message: {0} }}")]
    EAuthenticationError(error_stack::Report<AuthenticationError>),

    #[error("{{ error_description: Error while Authorizing, error_message: {0} }}")]
    EAuthorisationError(error_stack::Report<AuthorisationError>),

    #[error("{{ error_description: Connector implementation missing, error_message: {0} }}")]
    NotImplementedByConnector(String), //Feature not implemented by chosen connector.

    #[error("{{ error_description: Unexpected error, error_message: {0} }}")]
    EUnexpectedError(error_stack::Report<UnexpectedError>),

    #[error("{{ error_description: Error while parsing, error_message: {0} }}")]
    EParsingError(error_stack::Report<ParsingError>),

    #[error("Application configuration error: {0}")]
    ConfigurationError(ConfigError),

    #[error("{{ error_description: Database operation failed, error_message: {0} }}")]
    EDatabaseError(error_stack::Report<storage_errors::DatabaseError>),

    #[error("{{ error_description: Encryption module operation failed, error_message: {0} }}")]
    EEncryptionError(error_stack::Report<EncryptionError>),

    #[error("Metrics error: {0}")]
    EMetrics(MetricsError),

    #[error("I/O: {0}")]
    EIo(std::io::Error),
}

router_error_error_stack_specific!(
    error_stack::Report<storage_errors::DatabaseError>,
    ApplicationError::EDatabaseError(error_stack::Report<DatabaseError>)
);
router_error_error_stack_specific!(
    error_stack::Report<AuthenticationError>,
    ApplicationError::EAuthenticationError(error_stack::Report<AuthenticationError>)
);
router_error_error_stack_specific!(
    error_stack::Report<UnexpectedError>,
    ApplicationError::EUnexpectedError(error_stack::Report<UnexpectedError>)
);
router_error_error_stack_specific!(
    error_stack::Report<ParsingError>,
    ApplicationError::EParsingError(error_stack::Report<ParsingError>)
);
router_error_error_stack_specific!(
    error_stack::Report<EncryptionError>,
    ApplicationError::EEncryptionError(error_stack::Report<EncryptionError>)
);

impl From<MetricsError> for ApplicationError {
    fn from(err: MetricsError) -> Self {
        Self::EMetrics(err)
    }
}

impl From<std::io::Error> for ApplicationError {
    fn from(err: std::io::Error) -> Self {
        Self::EIo(err)
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
        .append_header(("Via", "Juspay_Router"))
        .content_type("application/json")
        .body(format!(
            "{{\n\"error\": {{\n\"message\": \"{err}\" \n}} \n}}\n"
        ))
}

impl ResponseError for ApplicationError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::EParsingError(_)
            | Self::EAuthenticationError(_)
            | Self::EAuthorisationError(_) => StatusCode::BAD_REQUEST,

            Self::EDatabaseError(_)
            | Self::NotImplementedByConnector(_)
            | Self::EMetrics(_)
            | Self::EIo(_)
            | Self::ConfigurationError(_)
            | Self::EEncryptionError(_)
            | Self::EUnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        error_response(self)
    }
}

pub fn http_not_implemented() -> actix_web::HttpResponse<BoxBody> {
    ApiErrorResponse::NotImplemented.error_response()
}

#[derive(Debug, thiserror::Error)]
pub enum ApiClientError {
    #[error("Header map construction failed")]
    HeaderMapConstructionFailed,
    #[error("Invalid proxy configuration")]
    InvalidProxyConfiguration,
    #[error("Client construction failed")]
    ClientConstructionFailed,
    #[error("Certificate decode failed")]
    CertificateDecodeFailed,

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

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("Error while obtaining URL for the integration")]
    FailedToObtainIntegrationUrl,
    #[error("Failed to encode connector request")]
    RequestEncodingFailed,
    #[error("Request encoding failed : {0}")]
    RequestEncodingFailedWithReason(String),
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
    #[error("Failed to handle connector response")]
    ResponseHandlingFailed,
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: String },
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
    #[error("Missing connector transaction ID")]
    MissingConnectorTransactionID,
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
}

#[derive(Debug, thiserror::Error)]
pub enum CardVaultError {
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
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: String },
    #[error("The card vault returned an unexpected response: {0:?}")]
    UnexpectedResponseError(bytes::Bytes),
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
    ResourceFetchingFailed { resource_name: String },
    #[error("Failed while executing: {flow}")]
    FlowExecutionError { flow: String },
    #[error("Not Implemented")]
    NotImplemented,
    #[error("Job not found")]
    JobNotFound,
    #[error("Recieved Error ApiResponseError: {0}")]
    EApiErrorResponse(error_stack::Report<ApiErrorResponse>),
    #[error("Recieved Error StorageError: {0}")]
    EStorageError(error_stack::Report<StorageError>),
    #[error("Recieved Error RedisError: {0}")]
    ERedisError(error_stack::Report<RedisError>),
    #[error("Recieved Error ParsingError: {0}")]
    EParsingError(error_stack::Report<ParsingError>),
    #[error("Validation Error Recieved: {0}")]
    EValidationError(error_stack::Report<ValidationError>),
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
    #[error("Webhook event creation failed")]
    WebhookEventCreationFailed,
    #[error("Unable to fork webhooks flow for outgoing webhooks")]
    ForkFlowFailed,
    #[error("Webhook api call to merchant failed")]
    CallToMerchantFailed,
    #[error("Webhook not received by merchant")]
    NotReceivedByMerchant,
}
