pub mod customers_error_response;
pub mod error_handlers;
pub mod transformers;
#[cfg(feature = "olap")]
pub mod user;
pub mod utils;

use std::fmt::Display;

use actix_web::{body::BoxBody, ResponseError};
pub use common_utils::errors::{CustomResult, ParsingError, ValidationError};
use diesel_models::errors as storage_errors;
pub use hyperswitch_domain_models::errors::{
    api_error_response::{ApiErrorResponse, ErrorType, NotImplementedMessage},
    StorageError as DataStorageError,
};
pub use hyperswitch_interfaces::errors::ConnectorError;
pub use redis_interface::errors::RedisError;
use scheduler::errors as sch_errors;
use storage_impl::errors as storage_impl_errors;
#[cfg(feature = "olap")]
pub use user::*;

pub use self::{
    customers_error_response::CustomersErrorResponse,
    sch_errors::*,
    storage_errors::*,
    storage_impl_errors::*,
    utils::{ConnectorErrorExt, StorageErrorExt},
};
use crate::services;
pub type RouterResult<T> = CustomResult<T, ApiErrorResponse>;
pub type RouterResponse<T> = CustomResult<services::ApplicationResponse<T>, ApiErrorResponse>;

pub type ApplicationResult<T> = error_stack::Result<T, ApplicationError>;
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

#[macro_export]
macro_rules! capture_method_not_supported {
    ($connector:expr, $capture_method:expr) => {
        Err(errors::ConnectorError::NotSupported {
            message: format!("{} for selected payment method", $capture_method),
            connector: $connector,
        }
        .into())
    };
    ($connector:expr, $capture_method:expr, $payment_method_type:expr) => {
        Err(errors::ConnectorError::NotSupported {
            message: format!("{} for {}", $capture_method, $payment_method_type),
            connector: $connector,
        }
        .into())
    };
}

#[macro_export]
macro_rules! unimplemented_payment_method {
    ($payment_method:expr, $connector:expr) => {
        errors::ConnectorError::NotImplemented(format!(
            "{} through {}",
            $payment_method, $connector
        ))
    };
    ($payment_method:expr, $flow:expr, $connector:expr) => {
        errors::ConnectorError::NotImplemented(format!(
            "{} {} through {}",
            $payment_method, $flow, $connector
        ))
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
        message: NotImplementedMessage::Default,
    }
    .error_response()
}

#[derive(Debug, thiserror::Error)]
pub enum HealthCheckOutGoing {
    #[error("Outgoing call failed with error: {message}")]
    OutGoingFailed { message: String },
}

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Failed to save card in card vault")]
    SaveCardFailed,
    #[error("Failed to fetch card details from card vault")]
    FetchCardFailed,
    #[error("Failed to delete card in card vault")]
    DeleteCardFailed,
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
    #[error("Failed to generate fingerprint")]
    GenerateFingerprintFailed,
    #[error("Failed to encrypt vault request")]
    RequestEncryptionFailed,
    #[error("Failed to decrypt vault response")]
    ResponseDecryptionFailed,
    #[error("Failed to call vault")]
    VaultAPIError,
    #[error("Failed while calling locker API")]
    ApiError,
}

#[derive(Debug, thiserror::Error)]
pub enum AwsKmsError {
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,
    #[error("Failed to AWS KMS decrypt input data")]
    DecryptionFailed,
    #[error("Missing plaintext AWS KMS decryption output")]
    MissingPlaintextDecryptionOutput,
    #[error("Failed to UTF-8 decode decryption output")]
    Utf8DecodingFailed,
}

#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum WebhooksFlowError {
    #[error("Merchant webhook config not found")]
    MerchantConfigNotFound,
    #[error("Webhook details for merchant not configured")]
    MerchantWebhookDetailsNotFound,
    #[error("Merchant does not have a webhook URL configured")]
    MerchantWebhookUrlNotConfigured,
    #[error("Webhook event updation failed")]
    WebhookEventUpdationFailed,
    #[error("Outgoing webhook body signing failed")]
    OutgoingWebhookSigningFailed,
    #[error("Webhook api call to merchant failed")]
    CallToMerchantFailed,
    #[error("Webhook not received by merchant")]
    NotReceivedByMerchant,
    #[error("Dispute webhook status validation failed")]
    DisputeWebhookValidationFailed,
    #[error("Outgoing webhook body encoding failed")]
    OutgoingWebhookEncodingFailed,
    #[error("Failed to update outgoing webhook process tracker task")]
    OutgoingWebhookProcessTrackerTaskUpdateFailed,
    #[error("Failed to schedule retry attempt for outgoing webhook")]
    OutgoingWebhookRetrySchedulingFailed,
    #[error("Outgoing webhook response encoding failed")]
    OutgoingWebhookResponseEncodingFailed,
}

impl WebhooksFlowError {
    pub(crate) fn is_webhook_delivery_retryable_error(&self) -> bool {
        match self {
            Self::MerchantConfigNotFound
            | Self::MerchantWebhookDetailsNotFound
            | Self::MerchantWebhookUrlNotConfigured
            | Self::OutgoingWebhookResponseEncodingFailed => false,

            Self::WebhookEventUpdationFailed
            | Self::OutgoingWebhookSigningFailed
            | Self::CallToMerchantFailed
            | Self::NotReceivedByMerchant
            | Self::DisputeWebhookValidationFailed
            | Self::OutgoingWebhookEncodingFailed
            | Self::OutgoingWebhookProcessTrackerTaskUpdateFailed
            | Self::OutgoingWebhookRetrySchedulingFailed => true,
        }
    }
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

#[derive(Debug, thiserror::Error)]
pub enum PazeDecryptionError {
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,
    #[error("Failed to decrypt input data")]
    DecryptionFailed,
    #[error("Certificate parsing failed")]
    CertificateParsingFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum GooglePayDecryptionError {
    #[error("Invalid expiration time")]
    InvalidExpirationTime,
    #[error("Failed to base64 decode input data")]
    Base64DecodingFailed,
    #[error("Failed to decrypt input data")]
    DecryptionFailed,
    #[error("Failed to deserialize input data")]
    DeserializationFailed,
    #[error("Certificate parsing failed")]
    CertificateParsingFailed,
    #[error("Key deserialization failure")]
    KeyDeserializationFailed,
    #[error("Failed to derive a shared ephemeral key")]
    DerivingSharedEphemeralKeyFailed,
    #[error("Failed to derive a shared secret key")]
    DerivingSharedSecretKeyFailed,
    #[error("Failed to parse the tag")]
    ParsingTagError,
    #[error("HMAC verification failed")]
    HmacVerificationFailed,
    #[error("Failed to derive Elliptic Curve key")]
    DerivingEcKeyFailed,
    #[error("Failed to Derive Public key")]
    DerivingPublicKeyFailed,
    #[error("Failed to Derive Elliptic Curve group")]
    DerivingEcGroupFailed,
    #[error("Failed to allocate memory for big number")]
    BigNumAllocationFailed,
    #[error("Failed to get the ECDSA signature")]
    EcdsaSignatureFailed,
    #[error("Failed to verify the signature")]
    SignatureVerificationFailed,
    #[error("Invalid signature is provided")]
    InvalidSignature,
    #[error("Failed to parse the Signed Key")]
    SignedKeyParsingFailure,
    #[error("The Signed Key is expired")]
    SignedKeyExpired,
    #[error("Failed to parse the ECDSA signature")]
    EcdsaSignatureParsingFailed,
    #[error("Invalid intermediate signature is provided")]
    InvalidIntermediateSignature,
    #[error("Invalid protocol version")]
    InvalidProtocolVersion,
    #[error("Decrypted Token has expired")]
    DecryptedTokenExpired,
    #[error("Failed to parse the given value")]
    ParsingFailed,
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
    #[error("Unable to retrieve success based routing config")]
    SuccessBasedRoutingConfigError,
    #[error("Params not found in success based routing config")]
    SuccessBasedRoutingParamsNotFoundError,
    #[error("Unable to calculate success based routing config from dynamic routing service")]
    SuccessRateCalculationError,
    #[error("Success rate client from dynamic routing gRPC service not initialized")]
    SuccessRateClientInitializationError,
    #[error("Unable to convert from '{from}' to '{to}'")]
    GenericConversionError { from: String, to: String },
    #[error("Invalid success based connector label received from dynamic routing service: '{0}'")]
    InvalidSuccessBasedConnectorLabel(String),
    #[error("unable to find '{field}'")]
    GenericNotFoundError { field: String },
    #[error("Unable to deserialize from '{from}' to '{to}'")]
    DeserializationError { from: String, to: String },
    #[error("Unable to retrieve contract based routing config")]
    ContractBasedRoutingConfigError,
    #[error("Params not found in contract based routing config")]
    ContractBasedRoutingParamsNotFoundError,
    #[error("Unable to calculate contract score from dynamic routing service")]
    ContractScoreCalculationError,
    #[error("contract routing client from dynamic routing gRPC service not initialized")]
    ContractRoutingClientInitializationError,
    #[error("Invalid contract based connector label received from dynamic routing service: '{0}'")]
    InvalidContractBasedConnectorLabel(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConditionalConfigError {
    #[error("failed to fetch the fallback config for the merchant")]
    FallbackConfigFetchFailed,
    #[error("The lock on the DSL cache is most probably poisoned")]
    DslCachePoisoned,
    #[error("Merchant routing algorithm not found in cache")]
    CacheMiss,
    #[error("Expected DSL to be saved in DB but did not find")]
    DslMissingInDb,
    #[error("Unable to parse DSL from JSON")]
    DslParsingError,
    #[error("Failed to initialize DSL backend")]
    DslBackendInitError,
    #[error("Error executing the DSL")]
    DslExecutionError,
    #[error("Error constructing the Input")]
    InputConstructionError,
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkTokenizationError {
    #[error("Failed to save network token in vault")]
    SaveNetworkTokenFailed,
    #[error("Failed to fetch network token details from vault")]
    FetchNetworkTokenFailed,
    #[error("Failed to encode network token vault request")]
    RequestEncodingFailed,
    #[error("Failed to deserialize network token service response")]
    ResponseDeserializationFailed,
    #[error("Failed to delete network token")]
    DeleteNetworkTokenFailed,
    #[error("Network token service not configured")]
    NetworkTokenizationServiceNotConfigured,
    #[error("Failed while calling Network Token Service API")]
    ApiError,
}
