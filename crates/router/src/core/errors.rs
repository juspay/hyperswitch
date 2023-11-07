pub mod api_error_response;
pub mod customers_error_response;
pub mod error_handlers;
pub mod transformers;
pub mod utils;

use std::fmt::Display;

use actix_web::{body::BoxBody, ResponseError};
pub use common_utils::errors::{CustomResult, ParsingError, ValidationError};
pub use data_models::errors::StorageError as DataStorageError;
use diesel_models::errors as storage_errors;
pub use redis_interface::errors::RedisError;
use scheduler::errors as sch_errors;
use storage_impl::errors as storage_impl_errors;

pub use self::{
    api_error_response::ApiErrorResponse,
    customers_error_response::CustomersErrorResponse,
    sch_errors::*,
    storage_errors::*,
    storage_impl_errors::*,
    utils::{ConnectorErrorExt, StorageErrorExt},
};
use crate::services;
pub type RouterResult<T> = CustomResult<T, ApiErrorResponse>;
pub type RouterResponse<T> = CustomResult<services::ApplicationResponse<T>, ApiErrorResponse>;

pub type ApplicationResult<T> = Result<T, ApplicationError>;
pub type ApplicationResponse<T> = ApplicationResult<services::ApplicationResponse<T>>;

pub type CustomerResponse<T> =
    CustomResult<services::ApplicationResponse<T>, CustomersErrorResponse>;

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

impl_error_type!(EncryptionError, "Encryption error");

impl From<ring::error::Unspecified> for EncryptionError {
    fn from(_: ring::error::Unspecified) -> Self {
        Self
    }
}

pub fn http_not_implemented() -> actix_web::HttpResponse<BoxBody> {
    ApiErrorResponse::NotImplemented {
        message: api_error_response::NotImplementedMessage::Default,
    }
    .error_response()
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
    #[error("Missing apple pay tokenization data")]
    MissingApplePayTokenData,
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
    #[error("Merchant secret found for incoming webhook source verification is invalid")]
    WebhookVerificationSecretInvalid,
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
    #[error("The given currency method is not configured with the given connector")]
    CurrencyNotSupported {
        message: String,
        connector: &'static str,
    },
    #[error("Invalid Configuration")]
    InvalidConnectorConfig { config: &'static str },
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

#[derive(Debug, thiserror::Error)]
pub enum ApplePayDecryptionError {
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,
    #[error("Failed to decrypt input data")]
    DecryptionFailed,
    #[error("Certificate parsing failed")]
    CertificateParsingFailed,
    #[error("Certificate parsing failed")]
    MissingMerchantId,
    #[error("Key Deserialization failure")]
    KeyDeserializationFailed,
    #[error("Failed to Derive a shared secret key")]
    DerivingSharedSecretKeyFailed,
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
                    .chain(Into::<VecLinearErrorStack<'a>>::into(current_error.sources).0)
                })
                .collect();
            Self(multi_layered_errors)
        }
    }
}
#[cfg(feature = "detailed_errors")]
pub use error_stack_parsing::*;

#[derive(Debug, Clone, thiserror::Error)]
pub enum RoutingError {
    #[error("Merchant routing algorithm not found in cache")]
    CacheMiss,
    #[error("Final connector selection failed")]
    ConnectorSelectionFailed,
    #[error("[DSL] Missing required field in payment data: '{field_name}'")]
    DslMissingRequiredField { field_name: String },
    #[error("The lock on the DSL cache is most probably poisoned")]
    DslCachePoisoned,
    #[error("Expected DSL to be saved in DB but did not find")]
    DslMissingInDb,
    #[error("Unable to parse DSL from JSON")]
    DslParsingError,
    #[error("Failed to initialize DSL backend")]
    DslBackendInitError,
    #[error("Error updating merchant with latest dsl cache contents")]
    DslMerchantUpdateError,
    #[error("Error executing the DSL")]
    DslExecutionError,
    #[error("Final connector selection failed")]
    DslFinalConnectorSelectionFailed,
    #[error("[DSL] Received incorrect selection algorithm as DSL output")]
    DslIncorrectSelectionAlgorithm,
    #[error("there was an error saving/retrieving values from the kgraph cache")]
    KgraphCacheFailure,
    #[error("failed to refresh the kgraph cache")]
    KgraphCacheRefreshFailed,
    #[error("there was an error during the kgraph analysis phase")]
    KgraphAnalysisError,
    #[error("'profile_id' was not provided")]
    ProfileIdMissing,
    #[error("the profile was not found in the database")]
    ProfileNotFound,
    #[error("failed to fetch the fallback config for the merchant")]
    FallbackConfigFetchFailed,
    #[error("Invalid connector name received: '{0}'")]
    InvalidConnectorName(String),
    #[error("The routing algorithm in merchant account had invalid structure")]
    InvalidRoutingAlgorithmStructure,
    #[error("Volume split failed")]
    VolumeSplitFailed,
    #[error("Unable to parse metadata")]
    MetadataParsingError,
}
