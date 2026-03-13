pub use common_utils::errors::{CustomResult, ParsingError, ValidationError};
pub use hyperswitch_domain_models::{
    api,
    errors::api_error_response::{self, *},
};

pub type PmResult<T> = CustomResult<T, ApiErrorResponse>;
pub type PmResponse<T> = CustomResult<api::ApplicationResponse<T>, ApiErrorResponse>;
pub type VaultResult<T> = CustomResult<T, VaultError>;

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
