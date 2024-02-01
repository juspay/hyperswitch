#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    ServerNotAvailable,
    ObjectNotFound,
    InvalidRequestError,
}

#[derive(Debug, Clone, router_derive::ApiError)]
#[error(error_type_enum = ErrorType)]
// TODO: Remove this line if InternalServerError is used anywhere
#[allow(dead_code)]
pub enum DummyConnectorErrors {
    #[error(error_type = ErrorType::ServerNotAvailable, code = "DC_00", message = "Something went wrong")]
    InternalServerError,

    #[error(error_type = ErrorType::ObjectNotFound, code = "DC_01", message = "Payment does not exist in our records")]
    PaymentNotFound,

    #[error(error_type = ErrorType::InvalidRequestError, code = "DC_02", message = "Missing required param: {field_name}")]
    MissingRequiredField { field_name: &'static str },

    #[error(error_type = ErrorType::InvalidRequestError, code = "DC_03", message = "The refund amount exceeds the amount captured")]
    RefundAmountExceedsPaymentAmount,

    #[error(error_type = ErrorType::InvalidRequestError, code = "DC_04", message = "Card not supported. Please use test cards")]
    CardNotSupported,

    #[error(error_type = ErrorType::ObjectNotFound, code = "DC_05", message = "Refund does not exist in our records")]
    RefundNotFound,

    #[error(error_type = ErrorType::InvalidRequestError, code = "DC_06", message = "Payment is not successful")]
    PaymentNotSuccessful,

    #[error(error_type = ErrorType::ServerNotAvailable, code = "DC_07", message = "Error occurred while storing the payment")]
    PaymentStoringError,

    #[error(error_type = ErrorType::InvalidRequestError, code = "DC_08", message = "Payment declined: {message}")]
    PaymentDeclined { message: &'static str },
}

impl core::fmt::Display for DummyConnectorErrors {
        /// Formats the error response as a JSON string and writes it to the provided formatter.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{{"error":{}}}"#,
            serde_json::to_string(self)
                .unwrap_or_else(|_| "Dummy connector error response".to_string())
        )
    }
}

impl common_utils::errors::ErrorSwitch<api_models::errors::types::ApiErrorResponse>
    for DummyConnectorErrors
{
        /// This method switches the enum variant and returns an API error response based on the variant.
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        match self {
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("DC", 0, self.error_message(), None))
            }
            Self::PaymentNotFound => {
                AER::NotFound(ApiError::new("DC", 1, self.error_message(), None))
            }
            Self::MissingRequiredField { field_name: _ } => {
                AER::BadRequest(ApiError::new("DC", 2, self.error_message(), None))
            }
            Self::RefundAmountExceedsPaymentAmount => {
                AER::InternalServerError(ApiError::new("DC", 3, self.error_message(), None))
            }
            Self::CardNotSupported => {
                AER::BadRequest(ApiError::new("DC", 4, self.error_message(), None))
            }
            Self::RefundNotFound => {
                AER::NotFound(ApiError::new("DC", 5, self.error_message(), None))
            }
            Self::PaymentNotSuccessful => {
                AER::BadRequest(ApiError::new("DC", 6, self.error_message(), None))
            }
            Self::PaymentStoringError => {
                AER::InternalServerError(ApiError::new("DC", 7, self.error_message(), None))
            }
            Self::PaymentDeclined { message: _ } => {
                AER::BadRequest(ApiError::new("DC", 8, self.error_message(), None))
            }
        }
    }
}
