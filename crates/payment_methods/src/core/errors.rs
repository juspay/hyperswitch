pub use common_utils::errors::{CustomResult, ParsingError, ValidationError};
use crate::core::domain::services::ApplicationResponse;
pub use hyperswitch_domain_models::errors::{
    api_error_response::{ApiErrorResponse, ErrorType, NotImplementedMessage},
    StorageError as DataStorageError,
};
use scheduler::errors as sch_errors;
use storage_impl::errors as storage_impl_errors;
pub use self::{
    sch_errors::*,
    storage_impl_errors::*,
};
pub type RouterResult<T> = CustomResult<T, ApiErrorResponse>;
pub type RouterResponse<T> = CustomResult<ApplicationResponse<T>, ApiErrorResponse>;
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